use anyhow::Result;
use tokio::sync::mpsc;

use crate::freeswitch::{EslSupervisor, EslSupervisorConfig};
use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};
use super::esl::{EslHandle, EslEvent};
use super::connector::EslClientConfig;

pub struct FreeswitchTelephonyAdapter {
    domain_rx: Option<mpsc::Receiver<TelephonyEvent>>,
    esl_handle: EslHandle,
}

impl FreeswitchTelephonyAdapter {
    /// Create a new adapter from an EslHandle and event receiver
    /// This makes the adapter testable by allowing injection of the handle and events
    pub fn new(esl_handle: EslHandle, mut esl_event_rx: mpsc::Receiver<EslEvent>) -> Self {
        let (domain_tx, domain_rx) = mpsc::channel::<TelephonyEvent>(100);

        // Spawn task to convert ESL events to TelephonyEvent
        tokio::spawn(async move {
            while let Some(esl_event) = esl_event_rx.recv().await {
                match convert_esl_event(esl_event) {
                    Ok(telephony_event) => {
                        if domain_tx.send(telephony_event).await.is_err() {
                            tracing::error!("Domain event channel closed");
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::debug!("Unhandled ESL event: {}", e);
                    }
                }
            }
        });

        Self {
            domain_rx: Some(domain_rx),
            esl_handle,
        }
    }

    /// Convenience method that creates the supervisor and calls new
    pub async fn connect(config: EslSupervisorConfig) -> Result<Self> {
        
        let supervisor_config = EslSupervisorConfig {
            host: config.host,
            port: config.port,
            password: config.password,
            event_format: config.event_format,
        };

        let (esl_handle, esl_event_rx) = EslSupervisor::spawn(supervisor_config);
        Ok(Self::new(esl_handle, esl_event_rx))
    }
}

/// Convert ESL event to telephony event
fn convert_esl_event(ev: EslEvent) -> Result<TelephonyEvent> {
    let call_id = ev.headers.get("Unique-ID")
        .cloned()
        .unwrap_or_default();
    
    match ev.event_name.as_str() {
        "CHANNEL_CREATE" => Ok(TelephonyEvent::CallOffered { call_id }),
        "CHANNEL_HANGUP" => Ok(TelephonyEvent::CallEnded { call_id }),
        _ => Err(anyhow::anyhow!("Unhandled event type: {}", ev.event_name)),
    }
}

#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, _request: OriginateRequest) -> Result<()> {
        // TODO: Implement originate using self.esl_handle
        Ok(())
    }

    async fn hangup(&self, _request: HangupRequest) -> Result<()> {
        // TODO: Implement hangup using self.esl_handle
        Ok(())
    }

    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent> {
        self.domain_rx.take().expect("domain_rx already taken")
    }
}
