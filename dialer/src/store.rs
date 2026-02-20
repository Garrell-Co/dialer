pub mod memory;

/// State of a call in the system.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "status")]
pub enum CallState {
    Ringing,
    Answered,
    Held,
    Ended { reason: Option<String> },
}

/// A record of a call tracked by the store.
#[derive(Debug, Clone)]
pub struct CallRecord {
    pub call_id: String,
    pub destination: crate::telephony::DestinationType,
    pub from: String,
    pub state: CallState,
}

/// Trait for call state storage. Implementations must be thread-safe.
pub trait CallStore: Send + Sync {
    fn insert(&self, record: CallRecord);
    fn update_state(&self, call_id: &str, state: CallState);
    fn get(&self, call_id: &str) -> Option<CallRecord>;
    fn get_active(&self) -> Vec<CallRecord>;
    fn remove(&self, call_id: &str);
}
