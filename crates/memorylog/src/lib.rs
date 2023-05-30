use std::ops;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub index: u64,
    pub range: ops::Range<u64>,
    pub events: Vec<AccessEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MemoryEvent {
    Reserve(ReservationEvent),
    Access(AccessEvent),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReservationEvent {
    pub ptr: u64,
    pub size: u64,
    pub alignment: u64,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessEvent {
    pub instruction_address: u64,
    pub instruction_bytes: Vec<u8>,
    pub access_address: u64,
    pub data_after: Vec<u8>,
    pub is_write: bool,
    // pub context: CONTEXT,
}
