# Manual Phone Controller Design

## Summary

Add a controller loop architecture that accepts commands from the UI via a REST API, consults a policy engine, and drives the telephony adapter. The first vertical slice is a manual agent phone supporting dial, hangup, hold/resume, and transfer.

## Architecture

```
UI  →  REST API  →  Controller Loop  →  Telephony Adapter
                        ↕                      ↓
                   Policy Engine          FreeSWITCH
                        ↕
                    Call Store  ←───── (events feed back)
                        ↓
                   WebSocket  →  UI
```

- **REST API** (axum) accepts commands from the UI and sends them to the controller loop via a `tokio::mpsc` channel.
- **Controller Loop** is a `tokio::select!` loop listening on two channels: commands from the API and events from the telephony adapter. Commands are validated by the policy engine before execution. Events update the call store and broadcast state changes.
- **Policy Engine** takes a command and the current call store state, returns allow or deny. In this slice it validates basics only.
- **Call Store** is a trait-backed in-memory store holding active calls and their states. Both the controller and the WebSocket event broadcaster read from it.
- **WebSocket** pushes call state changes to connected UI clients in real time.

## REST API

```
POST   /calls              → Dial
DELETE /calls/:id           → Hangup
POST   /calls/:id/hold     → Hold
POST   /calls/:id/resume   → Resume
POST   /calls/:id/transfer → Transfer
```

## Commands

```rust
enum ControllerCommand {
    Dial { destination: DestinationType, from: String, caller_id_name: Option<String> },
    Hangup { call_id: String },
    Hold { call_id: String },
    Resume { call_id: String },
    Transfer { call_id: String, destination: DestinationType },
}
```

## TelephonyPort Changes

Two new methods added to the existing trait:

```rust
async fn hold(&self, call_id: &str) -> Result<()>;
async fn transfer(&self, call_id: &str, destination: DestinationType) -> Result<()>;
```

Resume is handled by `hold` (FreeSWITCH's `uuid_hold` toggles, or the adapter takes a flag).

## WebSocket Events

```rust
struct CallStateUpdate {
    call_id: String,
    state: CallState, // Ringing, Answered, Held, Ended
    reason: Option<String>,
}
```

## Call Store

```rust
#[derive(Debug, Clone, PartialEq)]
enum CallState {
    Ringing,
    Answered,
    Held,
    Ended { reason: Option<String> },
}

struct CallRecord {
    call_id: String,
    destination: DestinationType,
    from: String,
    state: CallState,
}

trait CallStore: Send + Sync {
    fn insert(&self, record: CallRecord);
    fn update_state(&self, call_id: &str, state: CallState);
    fn get(&self, call_id: &str) -> Option<CallRecord>;
    fn get_active(&self) -> Vec<CallRecord>;
    fn remove(&self, call_id: &str);
}
```

In-memory implementation uses `DashMap<String, CallRecord>`. A database-backed implementation can replace it later without changing the controller or adapter.

## Policy Engine

```rust
enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

trait PolicyEngine: Send + Sync {
    fn evaluate(&self, command: &ControllerCommand, store: &dyn CallStore) -> PolicyDecision;
}
```

Validation rules for this slice:

- **Dial**: deny if there is already an active (non-ended) call.
- **Hangup/Hold/Resume/Transfer**: deny if the call_id does not exist or is already ended.

## Controller Loop

```rust
async fn run_controller(
    telephony: &dyn TelephonyPort,
    store: &dyn CallStore,
    policy: &dyn PolicyEngine,
    command_rx: mpsc::Receiver<ControllerCommand>,
    event_rx: mpsc::Receiver<TelephonyEvent>,
    state_tx: broadcast::Sender<CallStateUpdate>,
) -> Result<()> {
    loop {
        tokio::select! {
            Some(cmd) = command_rx.recv() => {
                match policy.evaluate(&cmd, store) {
                    PolicyDecision::Allow => {
                        execute_command(cmd, telephony, store).await?;
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
        }
    }
}
```

`execute_command` maps each `ControllerCommand` variant to the corresponding `TelephonyPort` method and inserts/updates the call store. `apply_event` translates `TelephonyEvent`s into `CallState` transitions and returns a `CallStateUpdate` for the WebSocket broadcast.

## File Organization

New and modified files within `dialer/src/`:

```
dialer/src/
├── bin/
│   └── dialer_worker.rs        # Modified: wire up all components, start servers
├── app.rs                       # Modified: replace run_worker with run_controller
├── telephony.rs                 # Modified: add hold, transfer to TelephonyPort trait
├── controller/
│   ├── mod.rs                   # ControllerCommand enum, run_controller loop
│   └── commands.rs              # execute_command mapping
├── policy/
│   ├── mod.rs                   # PolicyEngine trait, PolicyDecision enum
│   └── manual_phone.rs          # Validation-only implementation for this slice
├── store/
│   ├── mod.rs                   # CallStore trait, CallRecord, CallState
│   └── memory.rs                # In-memory DashMap implementation
├── api/
│   ├── mod.rs                   # axum router setup
│   ├── routes.rs                # REST endpoint handlers
│   └── websocket.rs             # WebSocket upgrade and broadcast handler
├── freeswitch/
│   ├── ...                      # Existing files unchanged
│   └── telephony.rs             # Modified: implement hold, transfer
```

New Cargo.toml dependencies: `axum`, `tower`, `serde`, `dashmap`.

## Design Decisions

- **REST for commands, WebSocket for events**: clean separation of concerns; commands are request/response, state updates are real-time push.
- **Trait-based TelephonyPort**: strongly typed, compile-time checked. The controller calls known methods rather than dispatching generic command objects.
- **Separate CallStore trait**: decouples state management from both controller and adapter. Enables future database swap without refactoring either.
- **Policy engine as validation only**: keeps the first slice simple. Future slices can add enrichment, campaign rules, rate limiting by implementing the same trait.
