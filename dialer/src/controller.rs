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
