use crate::bridge::conn::{Connection, IoSample};
use crate::bridge::contacts::ContactBook;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex as StdMutex,
};
use std::time::{Duration, Instant};
use tokio::sync::{broadcast, Mutex};

// All outbound network access from this binary must go through metered_client().
// This is enforced by convention, not by the compiler. CI: grep for
// reqwest::Client::new and fail if found outside state.rs.

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Event {
    Message {
        from: String,
        counter: u64,
        payload: String,
    },
    ContactAdded {
        name: String,
    },
    Status {
        connected: bool,
    },
    Lagged,
}

pub struct OutgoingMessage {
    pub to: String,
    pub payload: Vec<u8>,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct MetricsSnapshot {
    pub bytes_up_per_sec: u64,
    pub bytes_down_per_sec: u64,
    pub epoch: u64,
    pub uptime_sec: u64,
}

#[derive(Debug, Default)]
pub struct RuntimeStats {
    telemetry: AtomicU64,
    analytics: AtomicU64,
    uploads: AtomicU64,
    contact_reads: AtomicU64,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct RuntimeStatsSnapshot {
    pub telemetry: u64,
    pub analytics: u64,
    pub uploads: u64,
    pub contact_reads: u64,
}

impl RuntimeStats {
    pub fn snapshot(&self) -> RuntimeStatsSnapshot {
        RuntimeStatsSnapshot {
            telemetry: self.telemetry.load(Ordering::SeqCst),
            analytics: self.analytics.load(Ordering::SeqCst),
            uploads: self.uploads.load(Ordering::SeqCst),
            contact_reads: self.contact_reads.load(Ordering::SeqCst),
        }
    }
}

struct MetricsInner {
    connected_since: Option<Instant>,
    window_started: Instant,
    bytes_up_window: u64,
    bytes_down_window: u64,
    bytes_up_per_sec: u64,
    bytes_down_per_sec: u64,
    epoch: u64,
}

pub struct BridgeMetrics {
    inner: StdMutex<MetricsInner>,
}

impl BridgeMetrics {
    fn new(connected: bool) -> Self {
        let now = Instant::now();
        Self {
            inner: StdMutex::new(MetricsInner {
                connected_since: connected.then_some(now),
                window_started: now,
                bytes_up_window: 0,
                bytes_down_window: 0,
                bytes_up_per_sec: 0,
                bytes_down_per_sec: 0,
                epoch: 0,
            }),
        }
    }

    pub fn record_io(&self, sample: IoSample) {
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("bridge metrics mutex poisoned");
        Self::roll_window(&mut inner, now);
        inner.bytes_up_window = inner.bytes_up_window.saturating_add(sample.bytes_up);
        inner.bytes_down_window = inner.bytes_down_window.saturating_add(sample.bytes_down);
    }

    pub fn set_epoch(&self, epoch: u64) {
        let mut inner = self.inner.lock().expect("bridge metrics mutex poisoned");
        inner.epoch = epoch;
    }

    fn set_connected(&self, connected: bool) {
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("bridge metrics mutex poisoned");
        Self::roll_window(&mut inner, now);
        inner.connected_since = connected.then_some(now);
    }

    pub fn snapshot(&self) -> MetricsSnapshot {
        let now = Instant::now();
        let mut inner = self.inner.lock().expect("bridge metrics mutex poisoned");
        Self::roll_window(&mut inner, now);
        MetricsSnapshot {
            bytes_up_per_sec: inner.bytes_up_per_sec,
            bytes_down_per_sec: inner.bytes_down_per_sec,
            epoch: inner.epoch,
            uptime_sec: inner
                .connected_since
                .map(|since| now.duration_since(since).as_secs())
                .unwrap_or(0),
        }
    }

    fn roll_window(inner: &mut MetricsInner, now: Instant) {
        let elapsed_secs = now.duration_since(inner.window_started).as_secs();
        if elapsed_secs == 0 {
            return;
        }

        inner.bytes_up_per_sec = inner.bytes_up_window / elapsed_secs;
        inner.bytes_down_per_sec = inner.bytes_down_window / elapsed_secs;
        inner.bytes_up_window = 0;
        inner.bytes_down_window = 0;
        inner.window_started += Duration::from_secs(elapsed_secs);
    }
}

#[derive(Clone)]
pub struct AppState {
    pub contacts: Arc<Mutex<ContactBook>>,
    pub outgoing: Arc<Mutex<VecDeque<OutgoingMessage>>>,
    pub events: broadcast::Sender<Event>,
    pub conn: Arc<Mutex<Connection>>,
    pub token: Arc<str>,
    connected: Arc<AtomicBool>,
    pub metrics: Arc<BridgeMetrics>,
    pub runtime_stats: Arc<RuntimeStats>,
    pub edge_name: Arc<str>,
}

impl AppState {
    pub fn new(conn: Connection, token: String, edge_name: String) -> Self {
        let (events, _) = broadcast::channel(256);
        Self {
            contacts: Arc::new(Mutex::new(ContactBook::new())),
            outgoing: Arc::new(Mutex::new(VecDeque::new())),
            events,
            conn: Arc::new(Mutex::new(conn)),
            token: token.into(),
            connected: Arc::new(AtomicBool::new(true)),
            metrics: Arc::new(BridgeMetrics::new(true)),
            runtime_stats: Arc::new(RuntimeStats::default()),
            edge_name: edge_name.into(),
        }
    }

    pub fn connected(&self) -> bool {
        self.connected.load(Ordering::SeqCst)
    }

    pub fn set_connected(&self, connected: bool) {
        let previous = self.connected.swap(connected, Ordering::SeqCst);
        if previous != connected {
            self.metrics.set_connected(connected);
            let _ = self.events.send(Event::Status { connected });
        }
    }

    pub fn record_io(&self, sample: IoSample) {
        self.metrics.record_io(sample);
    }

    pub fn metrics_snapshot(&self) -> MetricsSnapshot {
        self.metrics.snapshot()
    }

    pub fn runtime_stats_snapshot(&self) -> RuntimeStatsSnapshot {
        self.runtime_stats.snapshot()
    }
}
