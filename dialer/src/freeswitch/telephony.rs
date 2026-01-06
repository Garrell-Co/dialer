use anyhow::Result;
use tokio::sync::mpsc;

use crate::freeswitch::{EslSupervisor, EslSupervisorConfig};
use crate::freeswitch::types::FsEventKind;
use crate::telephony::{HangupRequest, OriginateRequest, OriginateResult, TelephonyEvent, TelephonyPort};
use super::esl::{EslHandle, EslEvent};

pub struct FreeswitchTelephonyAdapter {
    domain_rx: Option<mpsc::Receiver<TelephonyEvent>>,
    esl_handle: EslHandle,
}

impl FreeswitchTelephonyAdapter {

    pub fn new(
        esl_handle: EslHandle,
        mut esl_event_rx: mpsc::Receiver<EslEvent>,
        mut connection_state_rx: mpsc::Receiver<TelephonyEvent>,
    ) -> Self {
        let (domain_tx, domain_rx) = mpsc::channel::<TelephonyEvent>(100);

        // Spawn task to convert ESL events to TelephonyEvent
        let domain_tx_events = domain_tx.clone();
        tokio::spawn(async move {
            while let Some(esl_event) = esl_event_rx.recv().await {
                match convert_esl_event(esl_event) {
                    Ok(telephony_event) => {
                        if domain_tx_events.send(telephony_event).await.is_err() {
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

        // Spawn task to forward connection state events (TransportUp/TransportDown)
        tokio::spawn(async move {
            while let Some(connection_event) = connection_state_rx.recv().await {
                if domain_tx.send(connection_event).await.is_err() {
                    tracing::error!("Domain event channel closed");
                    break;
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
        let (esl_handle, esl_event_rx, connection_state_rx) = EslSupervisor::spawn(config);
        Ok(Self::new(esl_handle, esl_event_rx, connection_state_rx))
    }
}


/// Convert ESL event to telephony event
fn convert_esl_event(ev: EslEvent) -> Result<TelephonyEvent> {
    let call_id = ev.event_headers.get("Unique-ID")
        .cloned()
        .unwrap_or_default();
    
    let event_name = ev.event_headers.get("Event-Name")
        .cloned()
        .unwrap_or_default();
    
    match event_name.parse::<FsEventKind>() {
        Ok(FsEventKind::CHANNEL_CREATE) => Ok(TelephonyEvent::CallOffered { call_id }),
        Ok(FsEventKind::CHANNEL_HANGUP) => Ok(TelephonyEvent::CallEnded { call_id }),
        Ok(FsEventKind::CHANNEL_ANSWER) => Ok(TelephonyEvent::CallAnswered { call_id }),
        Ok(_) | Err(_) => {
            // Unknown event kind or parse error
            Ok(TelephonyEvent::Unknown { message: format!("{:?}", ev.event_headers) } )
        },
    }
}

#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, req: OriginateRequest) -> Result<OriginateResult> {
        let res = self.esl_handle.api(format!("originate loopback/{} {} park", req.extension, req.context)).await?;
        let id = res.event_body
            .as_ref()
            .and_then(|body| std::str::from_utf8(body).ok())
            .map(|s| s.trim().split_whitespace().last().unwrap_or("").to_string())
            .unwrap_or_default();
        Ok(OriginateResult { channel_leg_id: id })
    }

    async fn hangup(&self, req: HangupRequest) -> Result<()> {
        self.esl_handle.api(format!("api uuid_kill {} NORMAL_CLEARING", req.call_id)).await?;
        Ok(())
    }

    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent> {
        self.domain_rx.take().expect("domain_rx already taken")
    }
}


