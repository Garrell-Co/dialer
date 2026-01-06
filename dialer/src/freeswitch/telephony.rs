use anyhow::Result;
use tokio::sync::mpsc;

use crate::freeswitch::{EslSupervisor, EslSupervisorConfig};
use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};
use super::esl::{EslHandle, EslEvent};

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

/// Print the entire ESL event
fn print_esl_event(ev: &EslEvent) {
    let event_name = ev.event_headers.get("Event-Name")
        .or_else(|| ev.frame_headers.get("Event-Name"))
        .cloned()
        .unwrap_or_else(|| "Unknown".to_string());
    tracing::info!("ESL Event - Name: {}", event_name);
    tracing::info!("ESL Event - Frame Headers: {:?}", ev.frame_headers);
    tracing::info!("ESL Event - Event Headers: {:?}", ev.event_headers);
    
    match &ev.event_body {
        Some(body_bytes) => {
            match String::from_utf8(body_bytes.clone()) {
                Ok(body_str) => {
                    tracing::info!("ESL Event - Body: {}", body_str);
                }
                Err(_) => {
                    tracing::info!("ESL Event - Body (binary, {} bytes): {:?}", body_bytes.len(), body_bytes);
                }
            }
        }
        None => {
            tracing::info!("ESL Event - Body: (empty)");
        }
    }
}

/// Convert ESL event to telephony event
fn convert_esl_event(ev: EslEvent) -> Result<TelephonyEvent> {
    let call_id = ev.event_headers.get("Unique-ID")
        .or_else(|| ev.frame_headers.get("Unique-ID"))
        .cloned()
        .unwrap_or_default();
    
    let event_name = ev.event_headers.get("Event-Name")
        .or_else(|| ev.frame_headers.get("Event-Name"))
        .cloned()
        .unwrap_or_default();
    
    match event_name.as_str() {
        "CHANNEL_CREATE" => Ok(TelephonyEvent::CallOffered { call_id }),
        "CHANNEL_HANGUP" => Ok(TelephonyEvent::CallEnded { call_id }),
        _ => {
            //print_esl_event(&ev);
            Ok(TelephonyEvent::Unknown { message: format!("{:?}", ev.event_headers) } )
        },
    }
}

#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, req: OriginateRequest) -> Result<()> {
        self.esl_handle.api(format!("originate loopback/{}/{}", req.extension, req.context)).await?;
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


