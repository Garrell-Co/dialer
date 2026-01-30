use anyhow::Result;

use crate::store::{CallRecord, CallState, CallStore};
use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};

use super::ControllerCommand;

/// State update pushed to WebSocket clients.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CallStateUpdate {
    pub call_id: String,
    pub state: CallState,
}

/// Execute a validated command against the telephony adapter and update the store.
pub async fn execute_command(
    cmd: ControllerCommand,
    telephony: &dyn TelephonyPort,
    store: &dyn CallStore,
) -> Result<()> {
    match cmd {
        ControllerCommand::Dial {
            destination,
            from,
            caller_id_name,
        } => {
            let id = uuid::Uuid::new_v4().to_string();
            let req = OriginateRequest {
                id: id.clone(),
                from: from.clone(),
                caller_id_name,
                destination: destination.clone(),
                application: None,
            };
            store.insert(CallRecord {
                call_id: id.clone(),
                destination,
                from,
                state: CallState::Ringing,
            });
            if let Err(e) = telephony.originate(req).await {
                store.remove(&id);
                return Err(e);
            }
        }
        ControllerCommand::Hangup { call_id } => {
            telephony
                .hangup(HangupRequest {
                    call_id: call_id.clone(),
                })
                .await?;
        }
        ControllerCommand::Hold { call_id } => {
            telephony.hold(&call_id).await?;
        }
        ControllerCommand::Resume { call_id } => {
            telephony.resume(&call_id).await?;
        }
        ControllerCommand::Transfer {
            call_id,
            destination,
        } => {
            telephony.transfer(&call_id, destination).await?;
        }
    }
    Ok(())
}

/// Apply a telephony event to the store. Returns a state update if one should be broadcast.
pub fn apply_event(event: TelephonyEvent, store: &dyn CallStore) -> Option<CallStateUpdate> {
    match event {
        TelephonyEvent::CallLegCreated { call_id } => {
            // If we don't already have this call (originated calls are pre-inserted),
            // insert a placeholder. This handles inbound or bridged legs.
            if store.get(&call_id).is_none() {
                store.insert(CallRecord {
                    call_id: call_id.clone(),
                    destination: crate::telephony::DestinationType::Loopback {
                        extension: "unknown".into(),
                        context: "unknown".into(),
                    },
                    from: "unknown".into(),
                    state: CallState::Ringing,
                });
            }
            Some(CallStateUpdate {
                call_id,
                state: CallState::Ringing,
            })
        }
        TelephonyEvent::CallOriginated { call_id } => Some(CallStateUpdate {
            call_id,
            state: CallState::Ringing,
        }),
        TelephonyEvent::CallAnswered { call_id } => {
            store.update_state(&call_id, CallState::Answered);
            Some(CallStateUpdate {
                call_id,
                state: CallState::Answered,
            })
        }
        TelephonyEvent::CallEnded { call_id, reason } => {
            let state = CallState::Ended {
                reason: reason.clone(),
            };
            store.update_state(&call_id, state.clone());
            Some(CallStateUpdate {
                call_id,
                state,
            })
        }
        TelephonyEvent::CallHeld { call_id } => {
            store.update_state(&call_id, CallState::Held);
            Some(CallStateUpdate {
                call_id,
                state: CallState::Held,
            })
        }
        TelephonyEvent::CallResumed { call_id } => {
            store.update_state(&call_id, CallState::Answered);
            Some(CallStateUpdate {
                call_id,
                state: CallState::Answered,
            })
        }
        TelephonyEvent::TransportUp => None,
        TelephonyEvent::TransportDown => {
            // Mark all active calls as ended
            for record in store.get_active() {
                store.update_state(
                    &record.call_id,
                    CallState::Ended {
                        reason: Some("Transport lost".into()),
                    },
                );
            }
            None
        }
        TelephonyEvent::Unknown { .. } => None,
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

    #[test]
    fn test_apply_event_call_leg_created() {
        let store = MemoryCallStore::new();
        let event = TelephonyEvent::CallLegCreated {
            call_id: "call-1".into(),
        };
        let update = apply_event(event, &store);
        assert!(update.is_some());
        assert!(store.get("call-1").is_some());
    }

    #[test]
    fn test_apply_event_call_answered_updates_state() {
        let store = MemoryCallStore::new();
        store.insert(CallRecord {
            call_id: "call-1".into(),
            destination: loopback_dest(),
            from: "1000".into(),
            state: CallState::Ringing,
        });
        let event = TelephonyEvent::CallAnswered {
            call_id: "call-1".into(),
        };
        let update = apply_event(event, &store);
        assert!(update.is_some());
        let record = store.get("call-1").unwrap();
        assert_eq!(record.state, CallState::Answered);
    }

    #[test]
    fn test_apply_event_call_ended() {
        let store = MemoryCallStore::new();
        store.insert(CallRecord {
            call_id: "call-1".into(),
            destination: loopback_dest(),
            from: "1000".into(),
            state: CallState::Answered,
        });
        let event = TelephonyEvent::CallEnded {
            call_id: "call-1".into(),
            reason: Some("NORMAL_CLEARING".into()),
        };
        let update = apply_event(event, &store);
        assert!(update.is_some());
        let record = store.get("call-1").unwrap();
        assert_eq!(
            record.state,
            CallState::Ended {
                reason: Some("NORMAL_CLEARING".into())
            }
        );
    }

    #[test]
    fn test_apply_event_call_held() {
        let store = MemoryCallStore::new();
        store.insert(CallRecord {
            call_id: "call-1".into(),
            destination: loopback_dest(),
            from: "1000".into(),
            state: CallState::Answered,
        });
        let event = TelephonyEvent::CallHeld {
            call_id: "call-1".into(),
        };
        let update = apply_event(event, &store);
        assert!(update.is_some());
        let record = store.get("call-1").unwrap();
        assert_eq!(record.state, CallState::Held);
    }

    #[test]
    fn test_apply_event_call_resumed() {
        let store = MemoryCallStore::new();
        store.insert(CallRecord {
            call_id: "call-1".into(),
            destination: loopback_dest(),
            from: "1000".into(),
            state: CallState::Held,
        });
        let event = TelephonyEvent::CallResumed {
            call_id: "call-1".into(),
        };
        let update = apply_event(event, &store);
        assert!(update.is_some());
        let record = store.get("call-1").unwrap();
        assert_eq!(record.state, CallState::Answered);
    }

    #[test]
    fn test_apply_event_transport_down_clears_store() {
        let store = MemoryCallStore::new();
        store.insert(CallRecord {
            call_id: "call-1".into(),
            destination: loopback_dest(),
            from: "1000".into(),
            state: CallState::Answered,
        });
        let event = TelephonyEvent::TransportDown;
        apply_event(event, &store);
        assert!(store.get_active().is_empty());
    }
}
