use anyhow::{Result};
use tokio::sync::{mpsc};

use super::esl::{EslCommand, EslClientConfig, EslConnector, EslPort};
use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};


pub struct FreeswitchTelephonyAdapter {
    esl: Box<dyn EslPort>,
    domain_rx: Option<mpsc::Receiver<TelephonyEvent>>
}

impl FreeswitchTelephonyAdapter {
    pub async fn connect(
        connector: &dyn EslConnector,
        esl_config: &EslClientConfig
    ) -> Result<Self> {
        let mut esl = connector.connect().await?;
        esl.send_raw(format!("auth {}\n\n", esl_config.password)).await?;
        esl.send_raw(format!("event {} ALL", esl_config.event_format)).await?;

        let mut esl_rx = esl.take_event_rx();
        let (domain_tx, domain_rx) = mpsc::channel::<TelephonyEvent>(100);

        tokio::spawn(async move {
            while let Some(ev) = esl_rx.recv().await {
                let dom = match ev.event_name.as_str() {
                    "CHANNEL_CREATE" => TelephonyEvent::CallOffered { 
                        call_id: ev.headers.get("Unique-ID").cloned().unwrap_or_default()
                    },
                    "CHANNEL_HANGUP" => TelephonyEvent::CallEnded { 
                        call_id: ev.headers.get("Unique-ID").cloned().unwrap_or_default()
                    },
                    _ => continue,
                };

                if domain_tx.send(dom).await.is_err() {
                    break;
                }
            }

            let _ = domain_tx.send(TelephonyEvent::TransportDown).await;
        });

        Ok( Self {
            esl,
            domain_rx: Some(domain_rx)
        } )
    }
}


#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, request: OriginateRequest) -> Result<()> {
        // Build Freeswitch compatible command string
        self.esl.api("originate".to_string()).await?;
        Ok(())
    }

    async fn hangup(&self, request: HangupRequest) -> Result<()> {
        Ok(())
    }

    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent> {
        self.domain_rx.take().expect("domain_rx already taken")
    }
}