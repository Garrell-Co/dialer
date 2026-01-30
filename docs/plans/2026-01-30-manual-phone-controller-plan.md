# Manual Phone Controller Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace the single-shot `run_worker` with a long-lived controller loop that accepts commands via REST API, validates them through a policy engine, drives the telephony adapter, and pushes state changes over WebSocket.

**Architecture:** REST API (axum) sends commands via mpsc channel to a controller select! loop. The loop consults a policy engine, calls TelephonyPort methods, updates an in-memory CallStore, and broadcasts state changes over a tokio broadcast channel to WebSocket clients.

**Tech Stack:** Rust, tokio, axum, serde/serde_json, dashmap, existing FreeSWITCH ESL adapter.

---

### Task 1: Add new dependencies to Cargo.toml

**Files:**
- Modify: `dialer/Cargo.toml`

**Step 1: Add dependencies**

Add these to the `[dependencies]` section of `dialer/Cargo.toml`:

```toml
axum = { version = "0.8", features = ["ws"] }
dashmap = "6"
serde = { version = "1", features = ["derive"] }
tower-http = { version = "0.6", features = ["cors"] }
```

**Step 2: Verify it compiles**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo check`
Expected: compiles with no errors (warnings are fine)

**Step 3: Commit**

```bash
git add dialer/Cargo.toml
git commit -m "feat: add axum, dashmap, serde, tower-http dependencies"
```

---

### Task 2: Call Store trait and in-memory implementation

**Files:**
- Create: `dialer/src/store/mod.rs`
- Create: `dialer/src/store/memory.rs`
- Modify: `dialer/src/lib.rs` (add `pub mod store`)
- Modify: `dialer/src/telephony.rs` (derive Clone on DestinationType)

**Step 1: Write the tests**

Add tests at the bottom of `dialer/src/store/memory.rs`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib store`
Expected: FAIL - module not found

**Step 3: Add DestinationType derives**

In `dialer/src/telephony.rs`, add `Serialize, Deserialize` derives to `DestinationType` and `PartialEq`:

Change line 13 from:
```rust
pub enum DestinationType {
```
to (add derives above it):
```rust
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum DestinationType {
```

Remove the existing `#[derive(Debug, Clone)]` on line 12 if present.

**Step 4: Write store/mod.rs**

Create `dialer/src/store/mod.rs`:

```rust
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
```

**Step 5: Write store/memory.rs**

Create `dialer/src/store/memory.rs`:

```rust
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
```

Append the tests from Step 1 at the bottom of this file.

**Step 6: Register module in lib.rs**

In `dialer/src/lib.rs`, add `pub mod store;`:

```rust
pub mod app;
pub mod store;
pub mod telephony;

mod freeswitch;
```

**Step 7: Run tests to verify they pass**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib store`
Expected: 5 tests PASS

**Step 8: Commit**

```bash
git add dialer/src/store/ dialer/src/lib.rs dialer/src/telephony.rs
git commit -m "feat: add CallStore trait and in-memory implementation"
```

---

### Task 3: Policy engine trait and manual phone implementation

**Files:**
- Create: `dialer/src/policy/mod.rs`
- Create: `dialer/src/policy/manual_phone.rs`
- Create: `dialer/src/controller/mod.rs` (just the ControllerCommand enum for now)
- Modify: `dialer/src/lib.rs` (add `pub mod policy` and `pub mod controller`)

**Step 1: Write the tests**

Add tests at the bottom of `dialer/src/policy/manual_phone.rs`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib policy`
Expected: FAIL - module not found

**Step 3: Write controller/mod.rs (ControllerCommand only)**

Create `dialer/src/controller/mod.rs`:

```rust
pub mod commands;

use crate::telephony::DestinationType;

/// Commands sent from the API to the controller loop.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "action")]
pub enum ControllerCommand {
    Dial {
        destination: DestinationType,
        from: String,
        caller_id_name: Option<String>,
    },
    Hangup {
        call_id: String,
    },
    Hold {
        call_id: String,
    },
    Resume {
        call_id: String,
    },
    Transfer {
        call_id: String,
        destination: DestinationType,
    },
}
```

Create `dialer/src/controller/commands.rs` as an empty file for now (placeholder for Task 5):

```rust
// execute_command will be implemented in Task 5
```

**Step 4: Write policy/mod.rs**

Create `dialer/src/policy/mod.rs`:

```rust
pub mod manual_phone;

use crate::controller::ControllerCommand;
use crate::store::CallStore;

/// Result of a policy evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Trait for evaluating whether a command should be allowed.
pub trait PolicyEngine: Send + Sync {
    fn evaluate(&self, command: &ControllerCommand, store: &dyn CallStore) -> PolicyDecision;
}
```

**Step 5: Write policy/manual_phone.rs**

Create `dialer/src/policy/manual_phone.rs`:

```rust
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
```

Append the tests from Step 1 at the bottom of this file.

**Step 6: Register modules in lib.rs**

Update `dialer/src/lib.rs`:

```rust
pub mod app;
pub mod controller;
pub mod policy;
pub mod store;
pub mod telephony;

mod freeswitch;
```

**Step 7: Run tests to verify they pass**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib policy`
Expected: 8 tests PASS

**Step 8: Commit**

```bash
git add dialer/src/controller/ dialer/src/policy/ dialer/src/lib.rs
git commit -m "feat: add PolicyEngine trait and ManualPhonePolicy implementation"
```

---

### Task 4: Add hold and transfer to TelephonyPort and FreeSWITCH adapter

**Files:**
- Modify: `dialer/src/telephony.rs:88-94` (add hold, resume, transfer to trait)
- Modify: `dialer/src/freeswitch/telephony.rs:108-221` (implement new methods)

**Step 1: Add methods to TelephonyPort trait**

In `dialer/src/telephony.rs`, add three methods to the `TelephonyPort` trait (after `hangup_all`):

```rust
#[async_trait::async_trait]
pub trait TelephonyPort {
    async fn originate(&self, request: OriginateRequest) -> Result<OriginateResult>;
    async fn hangup(&self, request: HangupRequest) -> Result<()>;
    async fn hangup_all(&self) -> Result<()>;
    async fn hold(&self, call_id: &str) -> Result<()>;
    async fn resume(&self, call_id: &str) -> Result<()>;
    async fn transfer(&self, call_id: &str, destination: DestinationType) -> Result<()>;
    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent>;
}
```

**Step 2: Add CallHeld and CallResumed variants to TelephonyEvent**

In `dialer/src/telephony.rs`, add to the `TelephonyEvent` enum:

```rust
#[derive(Debug)]
pub enum TelephonyEvent {
    CallLegCreated { call_id: String },
    CallOriginated { call_id: String },
    CallEnded { call_id: String, reason: Option<String> },
    CallAnswered { call_id: String },
    CallHeld { call_id: String },
    CallResumed { call_id: String },
    TransportUp,
    TransportDown,
    Unknown { message: String },
}
```

**Step 3: Implement in FreeSWITCH adapter**

In `dialer/src/freeswitch/telephony.rs`, add these methods to the `impl TelephonyPort for FreeswitchTelephonyAdapter` block (before the closing `}`):

```rust
    async fn hold(&self, call_id: &str) -> Result<()> {
        let command = format!("uuid_hold {}", call_id);
        tracing::debug!(command = %command, "Sending hold command");
        let res = self.esl_handle.api(command).await?;
        let body = res.event_body.as_ref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or_default();
        if body.trim().starts_with("-ERR") {
            return Err(anyhow::anyhow!("FreeSWITCH error: {}", body.trim()));
        }
        Ok(())
    }

    async fn resume(&self, call_id: &str) -> Result<()> {
        let command = format!("uuid_hold off {}", call_id);
        tracing::debug!(command = %command, "Sending resume command");
        let res = self.esl_handle.api(command).await?;
        let body = res.event_body.as_ref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or_default();
        if body.trim().starts_with("-ERR") {
            return Err(anyhow::anyhow!("FreeSWITCH error: {}", body.trim()));
        }
        Ok(())
    }

    async fn transfer(&self, call_id: &str, destination: DestinationType) -> Result<()> {
        let dest_str = match destination {
            DestinationType::Loopback { extension, context } => {
                format!("{} XML {}", extension, context)
            }
            DestinationType::RegisteredUser { user, domain } => {
                if let Some(domain) = domain {
                    format!("user/{}@{}", user, domain)
                } else {
                    format!("user/{}", user)
                }
            }
            DestinationType::External { destination } => destination,
            DestinationType::Gateway { gateway_name, number } => {
                format!("sofia/gateway/{}/{}", gateway_name, number)
            }
        };
        let command = format!("uuid_transfer {} {}", call_id, dest_str);
        tracing::debug!(command = %command, "Sending transfer command");
        let res = self.esl_handle.api(command).await?;
        let body = res.event_body.as_ref()
            .and_then(|b| std::str::from_utf8(b).ok())
            .unwrap_or_default();
        if body.trim().starts_with("-ERR") {
            return Err(anyhow::anyhow!("FreeSWITCH error: {}", body.trim()));
        }
        Ok(())
    }
```

**Step 4: Add CHANNEL_HOLD and CHANNEL_UNHOLD event conversion**

In `dialer/src/freeswitch/telephony.rs`, update the `convert_esl_event` function's match arm. Add these cases before the `Ok(_) | Err(_)` catch-all:

```rust
        Ok(FsEventKind::CHANNEL_HOLD) => Ok(TelephonyEvent::CallHeld { call_id }),
        Ok(FsEventKind::CHANNEL_UNHOLD) => Ok(TelephonyEvent::CallResumed { call_id }),
```

**Step 5: Verify it compiles**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo check`
Expected: compiles (may have warnings about unused variants, that's fine)

**Step 6: Commit**

```bash
git add dialer/src/telephony.rs dialer/src/freeswitch/telephony.rs
git commit -m "feat: add hold, resume, transfer to TelephonyPort and FreeSWITCH adapter"
```

---

### Task 5: Controller loop and command execution

**Files:**
- Modify: `dialer/src/controller/mod.rs` (add run_controller)
- Modify: `dialer/src/controller/commands.rs` (add execute_command and apply_event)

**Step 1: Write tests**

Add tests at the bottom of `dialer/src/controller/commands.rs`:

```rust
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
```

**Step 2: Run tests to verify they fail**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib controller`
Expected: FAIL - functions not found

**Step 3: Write controller/commands.rs**

Replace `dialer/src/controller/commands.rs`:

```rust
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
                call_id: id,
                destination,
                from,
                state: CallState::Ringing,
            });
            telephony.originate(req).await?;
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
```

**Step 4: Write the controller loop in controller/mod.rs**

Update `dialer/src/controller/mod.rs`:

```rust
pub mod commands;

use anyhow::Result;
use tokio::sync::{broadcast, mpsc};

use crate::policy::{PolicyDecision, PolicyEngine};
use crate::store::CallStore;
use crate::telephony::{DestinationType, TelephonyEvent, TelephonyPort};

use commands::{apply_event, execute_command, CallStateUpdate};

/// Commands sent from the API to the controller loop.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "action")]
pub enum ControllerCommand {
    Dial {
        destination: DestinationType,
        from: String,
        caller_id_name: Option<String>,
    },
    Hangup {
        call_id: String,
    },
    Hold {
        call_id: String,
    },
    Resume {
        call_id: String,
    },
    Transfer {
        call_id: String,
        destination: DestinationType,
    },
}

/// Run the controller loop. Listens for commands and telephony events,
/// applies policy, executes commands, and broadcasts state updates.
pub async fn run_controller(
    telephony: &dyn TelephonyPort,
    store: &dyn CallStore,
    policy: &dyn PolicyEngine,
    mut command_rx: mpsc::Receiver<ControllerCommand>,
    mut event_rx: mpsc::Receiver<TelephonyEvent>,
    state_tx: broadcast::Sender<CallStateUpdate>,
) -> Result<()> {
    // Wait for transport to be up before accepting commands
    loop {
        match event_rx.recv().await {
            Some(TelephonyEvent::TransportUp) => {
                tracing::info!("Telephony transport up, controller ready");
                break;
            }
            Some(TelephonyEvent::TransportDown) => {
                tracing::info!("Waiting for telephony transport...");
            }
            Some(other) => {
                tracing::debug!("Received event while waiting for transport: {:?}", other);
            }
            None => return Err(anyhow::anyhow!("Event channel closed before transport up")),
        }
    }

    // Main loop
    loop {
        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                match policy.evaluate(&cmd, store) {
                    PolicyDecision::Allow => {
                        if let Err(e) = execute_command(cmd, telephony, store).await {
                            tracing::error!("Command execution failed: {}", e);
                        }
                    }
                    PolicyDecision::Deny { reason } => {
                        tracing::warn!("Command denied: {}", reason);
                    }
                }
            }
            Some(event) = event_rx.recv() => {
                if let Some(update) = apply_event(event, store) {
                    let _ = state_tx.send(update);
                }
            }
            else => break,
        }
    }

    Ok(())
}
```

**Step 5: Run tests to verify they pass**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test --lib controller`
Expected: 7 tests PASS

**Step 6: Verify full project compiles**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo check`
Expected: compiles

**Step 7: Commit**

```bash
git add dialer/src/controller/
git commit -m "feat: add controller loop and command execution"
```

---

### Task 6: REST API and WebSocket server

**Files:**
- Create: `dialer/src/api/mod.rs`
- Create: `dialer/src/api/routes.rs`
- Create: `dialer/src/api/websocket.rs`
- Modify: `dialer/src/lib.rs` (add `pub mod api`)

**Step 1: Write api/mod.rs**

Create `dialer/src/api/mod.rs`:

```rust
pub mod routes;
pub mod websocket;

use std::sync::Arc;

use axum::Router;
use tokio::sync::{broadcast, mpsc};
use tower_http::cors::CorsLayer;

use crate::controller::commands::CallStateUpdate;
use crate::controller::ControllerCommand;
use crate::store::CallStore;

/// Shared state available to all API handlers.
#[derive(Clone)]
pub struct AppState {
    pub command_tx: mpsc::Sender<ControllerCommand>,
    pub state_tx: broadcast::Sender<CallStateUpdate>,
    pub store: Arc<dyn CallStore>,
}

/// Build the axum router with all routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(routes::call_routes())
        .merge(websocket::ws_routes())
        .layer(CorsLayer::permissive())
        .with_state(state)
}
```

**Step 2: Write api/routes.rs**

Create `dialer/src/api/routes.rs`:

```rust
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::controller::ControllerCommand;
use crate::telephony::DestinationType;

use super::AppState;

#[derive(Deserialize)]
pub struct DialRequest {
    pub destination: DestinationType,
    pub from: String,
    pub caller_id_name: Option<String>,
}

#[derive(Deserialize)]
pub struct TransferRequest {
    pub destination: DestinationType,
}

pub fn call_routes() -> Router<AppState> {
    Router::new()
        .route("/calls", post(dial))
        .route("/calls/{id}", delete(hangup))
        .route("/calls/{id}/hold", post(hold))
        .route("/calls/{id}/resume", post(resume))
        .route("/calls/{id}/transfer", post(transfer))
}

async fn dial(
    State(state): State<AppState>,
    Json(req): Json<DialRequest>,
) -> StatusCode {
    let cmd = ControllerCommand::Dial {
        destination: req.destination,
        from: req.from,
        caller_id_name: req.caller_id_name,
    };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn hangup(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Hangup { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn hold(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Hold { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn resume(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> StatusCode {
    let cmd = ControllerCommand::Resume { call_id: id };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

async fn transfer(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<TransferRequest>,
) -> StatusCode {
    let cmd = ControllerCommand::Transfer {
        call_id: id,
        destination: req.destination,
    };
    match state.command_tx.send(cmd).await {
        Ok(_) => StatusCode::ACCEPTED,
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
```

**Step 3: Write api/websocket.rs**

Create `dialer/src/api/websocket.rs`:

```rust
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;

use super::AppState;

pub fn ws_routes() -> Router<AppState> {
    Router::new().route("/ws", get(ws_upgrade))
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_ws(socket, state))
}

async fn handle_ws(mut socket: WebSocket, state: AppState) {
    let mut rx = state.state_tx.subscribe();

    loop {
        match rx.recv().await {
            Ok(update) => {
                let json = match serde_json::to_string(&update) {
                    Ok(j) => j,
                    Err(e) => {
                        tracing::error!("Failed to serialize state update: {}", e);
                        continue;
                    }
                };
                if socket.send(Message::Text(json.into())).await.is_err() {
                    // Client disconnected
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!("WebSocket client lagged, skipped {} messages", n);
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
}
```

**Step 4: Register module in lib.rs**

Update `dialer/src/lib.rs`:

```rust
pub mod api;
pub mod app;
pub mod controller;
pub mod policy;
pub mod store;
pub mod telephony;

mod freeswitch;
```

**Step 5: Verify it compiles**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo check`
Expected: compiles

**Step 6: Commit**

```bash
git add dialer/src/api/ dialer/src/lib.rs
git commit -m "feat: add REST API routes and WebSocket event broadcasting"
```

---

### Task 7: Wire everything together in dialer_worker.rs

**Files:**
- Modify: `dialer/src/bin/dialer_worker.rs` (replace current main with wired-up system)
- Modify: `dialer/src/app.rs` (can remove `run_worker` or keep it alongside; the new entry point bypasses it)

**Step 1: Rewrite dialer_worker.rs**

Replace the contents of `dialer/src/bin/dialer_worker.rs`:

```rust
use std::sync::Arc;

use dialer::api::{self, AppState};
use dialer::app::WorkerConfig;
use dialer::controller;
use dialer::freeswitch::types::FsEventKind;
use dialer::freeswitch::{EslEventFormat, EslSupervisorConfig, FreeswitchTelephonyAdapter};
use dialer::policy::manual_phone::ManualPhonePolicy;
use dialer::store::memory::MemoryCallStore;
use dialer::telephony::TelephonyPort;

use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::{fmt, EnvFilter};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let cfg = WorkerConfig::from_env_and_args("dialer_worker")?;
    init_tracing(&cfg)?;

    tracing::info!(
        "Starting dialer worker '{}' at log_level={}",
        cfg.worker_name,
        cfg.log_level
    );

    // Build telephony adapter
    let supervisor_config = EslSupervisorConfig {
        host: cfg.freeswitch_host.clone(),
        port: cfg.freeswitch_port,
        password: cfg.freeswitch_password.clone(),
        event_format: EslEventFormat::Json,
        event_list: vec![
            FsEventKind::CHANNEL_CREATE,
            FsEventKind::CHANNEL_STATE,
            FsEventKind::CHANNEL_DESTROY,
            FsEventKind::CHANNEL_ANSWER,
            FsEventKind::CHANNEL_HANGUP,
            FsEventKind::CHANNEL_HANGUP_COMPLETE,
            FsEventKind::CHANNEL_PROGRESS,
            FsEventKind::CHANNEL_PROGRESS_MEDIA,
            FsEventKind::CHANNEL_PARK,
            FsEventKind::CHANNEL_UNPARK,
            FsEventKind::CHANNEL_ORIGINATE,
            FsEventKind::CHANNEL_OUTGOING,
            FsEventKind::CHANNEL_BRIDGE,
            FsEventKind::CHANNEL_UNBRIDGE,
            FsEventKind::CHANNEL_HOLD,
            FsEventKind::CHANNEL_UNHOLD,
            FsEventKind::CHANNEL_EXECUTE,
            FsEventKind::CHANNEL_EXECUTE_COMPLETE,
            FsEventKind::CHANNEL_APPLICATION,
            FsEventKind::CHANNEL_DATA,
            FsEventKind::CHANNEL_UUID,
            FsEventKind::CHANNEL_CALLSTATE,
        ],
    };

    let mut telephony = FreeswitchTelephonyAdapter::connect(supervisor_config).await?;
    let event_rx = telephony.take_event_rx();

    // Build components
    let store: Arc<dyn dialer::store::CallStore> = Arc::new(MemoryCallStore::new());
    let policy = ManualPhonePolicy;
    let (command_tx, command_rx) = mpsc::channel(100);
    let (state_tx, _) = broadcast::channel(100);

    // Build API
    let app_state = AppState {
        command_tx,
        state_tx: state_tx.clone(),
        store: store.clone(),
    };
    let router = api::build_router(app_state);

    let api_port = std::env::var("API_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(3001u16);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", api_port)).await?;
    tracing::info!("API server listening on 0.0.0.0:{}", api_port);

    // Spawn API server
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, router).await {
            tracing::error!("API server error: {}", e);
        }
    });

    // Run controller loop (blocks until shutdown)
    controller::run_controller(&telephony, store.as_ref(), &policy, command_rx, event_rx, state_tx)
        .await
}

fn init_tracing(cfg: &WorkerConfig) -> anyhow::Result<()> {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(&cfg.log_level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    Ok(())
}
```

**Step 2: Fix module visibility**

The binary imports `dialer::freeswitch::*` directly. The `freeswitch` module is currently `mod freeswitch` (private). Change `dialer/src/lib.rs` to make it public:

```rust
pub mod api;
pub mod app;
pub mod controller;
pub mod policy;
pub mod store;
pub mod telephony;

pub mod freeswitch;
```

Also in `dialer/src/freeswitch.rs`, make the re-exports public:

```rust
mod esl;
mod reader;
mod telephony;

pub mod types;

pub use esl::{EslEventFormat, EslSupervisor, EslSupervisorConfig};
pub use telephony::FreeswitchTelephonyAdapter;
```

**Step 3: Verify it compiles**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo check`
Expected: compiles

**Step 4: Run all tests**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo test`
Expected: all tests pass

**Step 5: Commit**

```bash
git add dialer/src/bin/dialer_worker.rs dialer/src/lib.rs dialer/src/freeswitch.rs
git commit -m "feat: wire up controller loop, API server, and WebSocket in dialer_worker"
```

---

### Task 8: Smoke test with curl

This task verifies the full system end-to-end. Requires a running FreeSWITCH instance.

**Step 1: Start the dialer worker**

Run: `cd /home/jeremy/workspace/dialer/dialer && cargo run --bin dialer_worker`
Expected: Logs show "API server listening on 0.0.0.0:3001" and "Telephony transport up, controller ready"

**Step 2: Test the dial endpoint**

In a separate terminal:

```bash
curl -X POST http://localhost:3001/calls \
  -H "Content-Type: application/json" \
  -d '{"destination": {"type": "Loopback", "extension": "9196", "context": "default"}, "from": "1000"}'
```

Expected: HTTP 202 Accepted. Worker logs show originate command sent.

**Step 3: Test hangup**

```bash
curl -X DELETE http://localhost:3001/calls/<call_id_from_logs>
```

Expected: HTTP 202 Accepted. Worker logs show hangup.

**Step 4: Test WebSocket**

```bash
# In a separate terminal, connect before dialing:
# Requires websocat or similar tool
websocat ws://localhost:3001/ws
```

Expected: JSON state updates stream in as calls change state.

**Step 5: Commit any fixes**

If any fixes were needed during smoke testing, commit them:

```bash
git add -u
git commit -m "fix: smoke test corrections"
```
