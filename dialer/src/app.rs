use tracing;

use crate::freeswitch::types::FsEventKind;
use crate::telephony::{OriginateRequest, TelephonyEvent, TelephonyPort};
use crate::freeswitch::{EslEventFormat, EslSupervisorConfig, FreeswitchTelephonyAdapter};


#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub worker_name: String,
    pub log_level: String,

    pub freeswitch_host: String,
    pub freeswitch_port: u16,
    pub freeswitch_password: String,
}


impl WorkerConfig {
    pub fn from_env_and_args() -> anyhow::Result<Self> {
        let worker_name = std::env::var("WORKER_NAME").unwrap_or_else(|_| "dialer_worker".into());
        let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "info".into());

        let freeswitch_host = std::env::var("FREESWITCH_HOST")?;
        let freeswitch_port: u16 = std::env::var("FREESWITCH_PORT")?.parse()?;
        let freeswitch_password = std::env::var("FREESWITCH_PASSWORD")?;

        Ok(Self {
            worker_name,
            log_level,
            freeswitch_host,
            freeswitch_port,
            freeswitch_password,
        })
    }
}


struct DialerWorker {
    cfg: WorkerConfig
}

trait Worker {
    async fn run(&mut self) -> anyhow::Result<()>;
}

impl Worker for DialerWorker {
    async fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting dialer worker");

        tracing::debug!(
            host = %self.cfg.freeswitch_host,
            port = self.cfg.freeswitch_port,
            "Connecting to FreeSWITCH"
        );

        let supervisor_config = EslSupervisorConfig {
            host: self.cfg.freeswitch_host.clone(),
            port: self.cfg.freeswitch_port,
            password: self.cfg.freeswitch_password.clone(),
            event_format: EslEventFormat::Json,
            event_list: vec![
                FsEventKind::CHANNEL_ORIGINATE,
                FsEventKind::CHANNEL_CREATE,
                FsEventKind::CHANNEL_PROGRESS,
                FsEventKind::CHANNEL_ANSWER,
                FsEventKind::CHANNEL_HANGUP,
                FsEventKind::CHANNEL_DESTROY,
            ]
        };

        let mut telephony = FreeswitchTelephonyAdapter::connect(supervisor_config).await?;

        let mut event_rx = telephony.take_event_rx();

        // Wait for transport to be up
        loop {
            let event = event_rx.recv().await
                .ok_or_else(|| {
                    tracing::error!("Channel closed, no more events");
                    anyhow::anyhow!("Channel closed, no more events")
                })?;
            
            if let TelephonyEvent::TransportUp = event {
                tracing::info!("Telephony connection established");
                break;
            }
            
            match event {
                TelephonyEvent::TransportDown => {
                    tracing::info!("Telephony connection lost, waiting for transport up");
                },
                _ => {
                    tracing::debug!("Received event while waiting for transport up: {:?}", event);
                }
            }
        }

        // Originate the call
        tracing::debug!("Originating call");
        let originate_result = telephony.originate(OriginateRequest {
            from: "+2015557782".to_string(),
            to: "+2014007782".to_string(),
            context: "default".to_string(),
            extension: "1001".to_string(),
            priority: 1,
        }).await?;

        let originated_call_id = originate_result.channel_leg_id.clone();
        tracing::info!("Originated a call with id {}", originated_call_id);

        // Wait for the call to be answered, then hang up
        loop {
            let event = event_rx.recv().await
                .ok_or_else(|| {
                    tracing::error!("Channel closed, no more events");
                    anyhow::anyhow!("Channel closed, no more events")
                })?;
            
            match event {
                TelephonyEvent::CallAnswered { call_id } => {
                    if call_id == originated_call_id {
                        tracing::info!("Call {} answered, hanging up", call_id);
                        telephony.hangup(crate::telephony::HangupRequest {
                            call_id: call_id.clone(),
                        }).await?;
                        tracing::info!("Hangup sent for call {}", call_id);
                    } else {
                        tracing::debug!("Call answered event for different call: {} (waiting for {})", call_id, originated_call_id);
                    }
                },
                TelephonyEvent::CallEnded { call_id } => {
                    if call_id == originated_call_id {
                        tracing::info!("Call {} ended", call_id);
                        break;
                    }
                },
                TelephonyEvent::TransportDown => {
                    tracing::info!("Telephony connection lost");
                    break;
                },
                TelephonyEvent::Unknown { message } => {
                    //tracing::info!("Unknown event received: {}", message);
                }
                _ => {
                    tracing::debug!("Received event: {:?}", event);
                }
            }
        }

        // Wait for Ctrl+C (SIGINT) and exit gracefully
        use tokio::signal;
        signal::ctrl_c().await.expect("Failed to listen for ctrl_c");
        tracing::info!("Ctrl+C received, shutting down");
        Ok(())
    }
}


async fn build_worker(cfg: WorkerConfig) -> anyhow::Result<impl Worker> {
    Ok(DialerWorker { cfg })
}


pub async fn run_worker(cfg: WorkerConfig) -> anyhow::Result<()> {
    let mut worker = build_worker(cfg).await?;
    worker.run().await
}
