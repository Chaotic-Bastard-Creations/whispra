use crate::bridge::contacts::Role;
use crate::bridge::state::{AppState, Event, OutgoingMessage};
use crate::cell;
use crate::epoch::Epoch;
use crate::slot::{DecoyKey, SlotId};
use crate::{Direction, EPOCH_MS, READ_BUCKET};
use rand::seq::SliceRandom;
use tokio::time::{interval_at, Duration, Instant, MissedTickBehavior};

pub async fn run(state: AppState) {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let ms_until_next = EPOCH_MS - (now_ms % EPOCH_MS);
    let start = Instant::now() + Duration::from_millis(ms_until_next);

    let mut ticker = interval_at(start, Duration::from_millis(EPOCH_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);

    let decoy_key = DecoyKey::random();

    loop {
        ticker.tick().await;
        let epoch = Epoch::now();
        state.metrics.set_epoch(epoch.0);
        match tick(epoch, &state, &decoy_key).await {
            Ok(()) => state.set_connected(true),
            Err(e) => {
                state.set_connected(false);
                eprintln!("epoch loop error: {e}");
                if let Err(reconnect_error) = reconnect(&state).await {
                    eprintln!("upstream reconnect failed: {reconnect_error}");
                }
            }
        }
    }
}

async fn reconnect(state: &AppState) -> anyhow::Result<()> {
    let mut conn = state.conn.lock().await;
    conn.reconnect().await?;
    state.set_connected(true);
    Ok(())
}

struct RecvPlan {
    contact_name: String,
    slot: SlotId,
    key: [u8; 32],
}

async fn tick(epoch: Epoch, state: &AppState, decoy_key: &DecoyKey) -> anyhow::Result<()> {
    let outgoing_msg: Option<OutgoingMessage> = state.outgoing.lock().await.pop_front();

    struct SendInfo {
        slot: SlotId,
        cell: Box<[u8; crate::CELL_SIZE]>,
        contact_name: String,
        next_counter: u64,
    }

    let (send_info, recv_plans): (Option<SendInfo>, Vec<RecvPlan>) = {
        let mut contacts = state.contacts.lock().await;

        let send = if let Some(ref msg) = outgoing_msg {
            if let Some(c) = contacts.get_mut(&msg.to) {
                let send_dir = match c.role {
                    Role::Initiator => Direction::InitiatorToResponder,
                    Role::Responder => Direction::ResponderToInitiator,
                };
                let counter = c.send_counter;
                match cell::seal(&c.content_key, counter, &msg.payload) {
                    Ok(sealed) => {
                        let slot = c.slot_key.slot(send_dir, epoch);
                        Some(SendInfo {
                            slot,
                            cell: sealed,
                            contact_name: c.name.clone(),
                            next_counter: counter + 1,
                        })
                    }
                    Err(_) => None,
                }
            } else {
                None
            }
        } else {
            None
        };

        let recv: Vec<RecvPlan> = contacts
            .iter()
            .take(READ_BUCKET)
            .map(|c| {
                let recv_dir = match c.role {
                    Role::Initiator => Direction::ResponderToInitiator,
                    Role::Responder => Direction::InitiatorToResponder,
                };
                RecvPlan {
                    contact_name: c.name.clone(),
                    slot: c.slot_key.slot(recv_dir, epoch),
                    key: c.content_key,
                }
            })
            .collect();

        (send, recv)
    };

    let sent_counter_update = send_info
        .as_ref()
        .map(|si| (si.contact_name.clone(), si.next_counter));

    let (put_slot, put_cell) = match send_info {
        Some(si) => (si.slot, si.cell),
        None => {
            let slot = decoy_key.slot(epoch, 0);
            let key: [u8; 32] = rand::random();
            let cell = cell::seal(&key, 0, &[])?;
            (slot, cell)
        }
    };

    let real_count = recv_plans.len();
    enum ReadSlot {
        Real(RecvPlan),
        Decoy(SlotId),
    }

    let mut plan: Vec<ReadSlot> = recv_plans.into_iter().map(ReadSlot::Real).collect();
    for i in 0..READ_BUCKET.saturating_sub(real_count) {
        plan.push(ReadSlot::Decoy(decoy_key.slot(epoch, 1 + i as u32)));
    }
    plan.shuffle(&mut rand::rng());

    let hits: Vec<(String, [u8; 32], Box<[u8; crate::CELL_SIZE]>)> = {
        let mut conn = state.conn.lock().await;
        let sample = match conn.put(put_slot, put_cell).await {
            Ok(sample) => sample,
            Err(e) => {
                if sent_counter_update.is_some() {
                    if let Some(msg) = outgoing_msg {
                        state.outgoing.lock().await.push_front(msg);
                    }
                }
                return Err(e);
            }
        };
        state.record_io(sample);

        if let Some((contact_name, next_counter)) = &sent_counter_update {
            if let Some(c) = state.contacts.lock().await.get_mut(contact_name) {
                c.send_counter = *next_counter;
            }
        }

        let mut hits = Vec::new();
        for entry in plan {
            match entry {
                ReadSlot::Real(rp) => {
                    let (cell, sample) = conn.get(rp.slot).await?;
                    state.record_io(sample);
                    if let Some(cell) = cell {
                        hits.push((rp.contact_name, rp.key, cell));
                    }
                }
                ReadSlot::Decoy(slot) => {
                    let (_, sample) = conn.get(slot).await?;
                    state.record_io(sample);
                }
            }
        }
        hits
    };

    for (name, key, cell) in hits {
        if let Ok(opened) = cell::open(&key, &cell) {
            {
                let mut contacts = state.contacts.lock().await;
                if let Some(c) = contacts.get_mut(&name) {
                    if opened.counter >= c.recv_counter {
                        c.recv_counter = opened.counter + 1;
                    }
                }
            }
            let _ = state.events.send(Event::Message {
                from: name,
                counter: opened.counter,
                payload: String::from_utf8_lossy(&opened.payload).into_owned(),
            });
        }
    }

    Ok(())
}
