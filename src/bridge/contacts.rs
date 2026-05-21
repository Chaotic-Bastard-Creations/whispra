use crate::slot::SlotKey;
use std::collections::HashMap;

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Initiator,
    Responder,
}

pub struct Contact {
    pub name: String,
    pub role: Role,
    pub slot_key: SlotKey,
    pub content_key: [u8; 32],
    pub send_counter: u64,
    pub recv_counter: u64,
}

pub struct ContactBook {
    inner: HashMap<String, Contact>,
}

impl ContactBook {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    pub fn insert(&mut self, contact: Contact) {
        self.inner.insert(contact.name.clone(), contact);
    }

    pub fn get(&self, name: &str) -> Option<&Contact> {
        self.inner.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Contact> {
        self.inner.get_mut(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Contact> {
        self.inner.values()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
