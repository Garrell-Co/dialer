use crate::controller::ControllerCommand;
use crate::store::{CallState, CallStore};

use super::{PolicyDecision, PolicyEngine};

/// Policy for a manual phone: one active call at a time,
/// commands only valid against existing non-ended calls.
pub struct ManualPhonePolicy;

impl PolicyEngine for ManualPhonePolicy {
    fn evaluate(&self, command: &ControllerCommand, store: &dyn CallStore) -> PolicyDecision {
        match command {
            ControllerCommand::Dial { .. } => {
                if store.get_active().is_empty() {
                    PolicyDecision::Allow
                } else {
                    PolicyDecision::Deny {
                        reason: "An active call already exists".into(),
                    }
                }
            }
            ControllerCommand::Hangup { call_id }
            | ControllerCommand::Hold { call_id }
            | ControllerCommand::Resume { call_id }
            | ControllerCommand::Transfer { call_id, .. } => {
                match store.get(call_id) {
                    None => PolicyDecision::Deny {
                        reason: format!("Call {} not found", call_id),
                    },
                    Some(record) if matches!(record.state, CallState::Ended { .. }) => {
                        PolicyDecision::Deny {
                            reason: format!("Call {} has already ended", call_id),
                        }
                    }
                    Some(_) => PolicyDecision::Allow,
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::memory::MemoryCallStore;
    use crate::store::{CallRecord, CallState};
    use crate::telephony::DestinationType;

    fn loopback_dest() -> DestinationType {
        DestinationType::Loopback {
            extension: "1000".into(),
            context: "default".into(),
        }
    }

    fn make_record(call_id: &str, state: CallState) -> CallRecord {
        CallRecord {
            call_id: call_id.to_string(),
            destination: loopback_dest(),
            from: "1000".to_string(),
            state,
        }
    }

    #[test]
    fn test_dial_allowed_when_no_active_calls() {
        let store = MemoryCallStore::new();
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Dial {
            destination: loopback_dest(),
            from: "1000".into(),
            caller_id_name: None,
        };
        assert_eq!(policy.evaluate(&cmd, &store), PolicyDecision::Allow);
    }

    #[test]
    fn test_dial_denied_when_active_call_exists() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1", CallState::Answered));
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Dial {
            destination: loopback_dest(),
            from: "1000".into(),
            caller_id_name: None,
        };
        assert!(matches!(
            policy.evaluate(&cmd, &store),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn test_dial_allowed_when_only_ended_calls() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1", CallState::Ended { reason: None }));
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Dial {
            destination: loopback_dest(),
            from: "1000".into(),
            caller_id_name: None,
        };
        assert_eq!(policy.evaluate(&cmd, &store), PolicyDecision::Allow);
    }

    #[test]
    fn test_hangup_denied_for_nonexistent_call() {
        let store = MemoryCallStore::new();
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Hangup {
            call_id: "nope".into(),
        };
        assert!(matches!(
            policy.evaluate(&cmd, &store),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn test_hangup_allowed_for_active_call() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1", CallState::Answered));
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Hangup {
            call_id: "call-1".into(),
        };
        assert_eq!(policy.evaluate(&cmd, &store), PolicyDecision::Allow);
    }

    #[test]
    fn test_hangup_denied_for_ended_call() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1", CallState::Ended { reason: None }));
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Hangup {
            call_id: "call-1".into(),
        };
        assert!(matches!(
            policy.evaluate(&cmd, &store),
            PolicyDecision::Deny { .. }
        ));
    }

    #[test]
    fn test_hold_allowed_for_answered_call() {
        let store = MemoryCallStore::new();
        store.insert(make_record("call-1", CallState::Answered));
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Hold {
            call_id: "call-1".into(),
        };
        assert_eq!(policy.evaluate(&cmd, &store), PolicyDecision::Allow);
    }

    #[test]
    fn test_transfer_denied_for_nonexistent_call() {
        let store = MemoryCallStore::new();
        let policy = ManualPhonePolicy;
        let cmd = ControllerCommand::Transfer {
            call_id: "nope".into(),
            destination: loopback_dest(),
        };
        assert!(matches!(
            policy.evaluate(&cmd, &store),
            PolicyDecision::Deny { .. }
        ));
    }
}
