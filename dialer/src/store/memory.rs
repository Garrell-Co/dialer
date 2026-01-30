use dashmap::DashMap;

use super::{CallRecord, CallState, CallStore};

pub struct MemoryCallStore {
    calls: DashMap<String, CallRecord>,
}

impl MemoryCallStore {
    pub fn new() -> Self {
        Self {
            calls: DashMap::new(),
        }
    }
}

impl CallStore for MemoryCallStore {
    fn insert(&self, record: CallRecord) {
        self.calls.insert(record.call_id.clone(), record);
    }

    fn update_state(&self, call_id: &str, state: CallState) {
        if let Some(mut entry) = self.calls.get_mut(call_id) {
            entry.state = state;
        }
    }

    fn get(&self, call_id: &str) -> Option<CallRecord> {
        self.calls.get(call_id).map(|entry| entry.clone())
    }

    fn get_active(&self) -> Vec<CallRecord> {
        self.calls
            .iter()
            .filter(|entry| !matches!(entry.value().state, CallState::Ended { .. }))
            .map(|entry| entry.value().clone())
            .collect()
    }

    fn remove(&self, call_id: &str) {
        self.calls.remove(call_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telephony::DestinationType;

    fn make_record(call_id: &str) -> CallRecord {
        CallRecord {
            call_id: call_id.to_string(),
            destination: DestinationType::Loopback {
                extension: "1000".into(),
                context: "default".into(),
            },
            from: "1000".to_string(),
            state: CallState::Ringing,
        }
    }

    #[test]
    fn test_insert_and_get() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1"));
        let record = store.get("call-1").unwrap();
        assert_eq!(record.call_id, "call-1");
        assert_eq!(record.state, CallState::Ringing);
    }

    #[test]
    fn test_update_state() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1"));
        store.update_state("call-1", CallState::Answered);
        let record = store.get("call-1").unwrap();
        assert_eq!(record.state, CallState::Answered);
    }

    #[test]
    fn test_get_active_excludes_ended() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1"));
        store.insert(make_record("call-2"));
        store.update_state("call-1", CallState::Ended { reason: None });
        let active = store.get_active();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].call_id, "call-2");
    }

    #[test]
    fn test_remove() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1"));
        store.remove("call-1");
        assert!(store.get("call-1").is_none());
    }

    #[test]
    fn test_get_nonexistent_returns_none() {
        let store = MemoryCallStore::new();
        assert!(store.get("nope").is_none());
    }
}
