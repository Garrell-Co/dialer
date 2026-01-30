use anyhow::Result;
use tokio::sync::mpsc;

use super::esl::{EslEvent, EslHandle};
use crate::freeswitch::types::FsEventKind;
use crate::freeswitch::{EslSupervisor, EslSupervisorConfig};
use crate::telephony::{
    DestinationType, HangupRequest, OriginateRequest, OriginateResult, TelephonyEvent, TelephonyPort,
};

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
    let call_id = ev
        .event_headers
        .get("Unique-ID")
        .cloned()
        .unwrap_or_default();

    let event_name = ev
        .event_headers
        .get("Event-Name")
        .cloned()
        .unwrap_or_default();

    match event_name.parse::<FsEventKind>() {
        Ok(FsEventKind::CHANNEL_CREATE) => Ok(TelephonyEvent::CallLegCreated { call_id }),
        Ok(FsEventKind::CHANNEL_HANGUP) | Ok(FsEventKind::CHANNEL_HANGUP_COMPLETE) => {
            let hangup_reason = ev.event_headers.get("Hangup-Cause").cloned();
            Ok(TelephonyEvent::CallEnded {
                call_id,
                reason: hangup_reason,
            })
        }
        Ok(FsEventKind::CHANNEL_ANSWER) => Ok(TelephonyEvent::CallAnswered { call_id }),
        Ok(FsEventKind::CHANNEL_ORIGINATE) => Ok(TelephonyEvent::CallOriginated { call_id }),
        Ok(FsEventKind::CHANNEL_HOLD) => Ok(TelephonyEvent::CallHeld { call_id }),
        Ok(FsEventKind::CHANNEL_UNHOLD) => Ok(TelephonyEvent::CallResumed { call_id }),
        Ok(_) | Err(_) => {
            // Unknown event kind or parse error
            if let Some(b) = ev.event_body.as_ref() {
                if let Ok(s) = std::str::from_utf8(b) {
                    tracing::info!(
                        "Unknown or unhandled event: {}\nBody (UTF-8): '{}'",
                        event_name,
                        s
                    );
                }
            }
            Ok(TelephonyEvent::Unknown {
                message: format!("{:?}", event_name),
            })
        }
    }
}

#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, req: OriginateRequest) -> Result<OriginateResult> {
        let mut channel_vars = format!("origination_uuid={}", req.id);
        
        channel_vars.push_str(&format!(",caller_id_number={}", req.from));
        if let Some(ref name) = req.caller_id_name {
            // Quote values that contain spaces or special characters
            let quoted_name = if name.contains(' ') || name.contains(',') || name.contains('}') {
                format!("'{}'", name.replace('\'', "\\'"))
            } else {
                name.clone()
            };
            channel_vars.push_str(&format!(",caller_id_name={}", quoted_name));
        }
        
        let destination = match req.destination {
            DestinationType::Loopback { extension, context } => {
                format!("loopback/{}/{}", extension, context)
            }
            DestinationType::RegisteredUser { user, domain } => {
                if let Some(domain) = domain {
                    format!("user/{}@{}", user, domain)
                } else {
                    // Use default domain (could be made configurable)
                    format!("user/{}@192.168.86.28", user)
                }
            }
            DestinationType::External { destination } => {
                format!("sofia/external/{}", destination)
            }
            DestinationType::Gateway { gateway_name, number } => {
                format!("sofia/gateway/{}/{}", gateway_name, number)
            }
        };
        
        // Build application string (default to park if not specified)
        let application = req.application.unwrap_or_else(|| "park()".to_string());
        
        // Build the full originate command
        let command = format!(
            "originate {{{}}}{} &{}",
            channel_vars, destination, application
        );
        
        tracing::debug!(
            command = %command,
            "Sending originate command to FreeSWITCH"
        );
        
        let res = self.esl_handle.api(command.clone()).await?;
        
        // Check response body for errors
        let response_body = res
            .event_body
            .as_ref()
            .and_then(|body| std::str::from_utf8(body).ok())
            .unwrap_or_default();
        
        tracing::debug!(
            response_body = %response_body,
            "Received originate response from FreeSWITCH"
        );
        
        // Check if FreeSWITCH returned an error
        if response_body.trim().starts_with("-ERR") {
            let error_msg = response_body.trim().to_string();
            tracing::error!(
                command = %command,
                error = %error_msg,
                "FreeSWITCH returned error for originate command"
            );
            return Err(anyhow::anyhow!("FreeSWITCH error: {}", error_msg));
        }
        
        // Extract channel UUID from response
        // Success response format: "+OK <uuid>" or just "<uuid>"
        let id = response_body
            .split_whitespace()
            .last()
            .unwrap_or("")
            .to_string();
        
        if id.is_empty() {
            tracing::warn!(
                response_body = %response_body,
                "Could not extract channel UUID from originate response"
            );
        } else {
            tracing::debug!(
                channel_uuid = %id,
                "Successfully originated call"
            );
        }
        
        Ok(OriginateResult { channel_leg_id: id })
    }

    async fn hangup(&self, req: HangupRequest) -> Result<()> {
        self.esl_handle
            .api(format!("uuid_kill {} NORMAL_CLEARING", req.call_id))
            .await?;
        Ok(())
    }

    async fn hangup_all(&self) -> Result<()> {
        self.esl_handle.api("hupall".to_string()).await?;
        Ok(())
    }

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

    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent> {
        self.domain_rx.take().expect("domain_rx already taken")
    }
}
