use std::collections::HashSet;

use tracing;

use crate::freeswitch::types::FsEventKind;
use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};
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
                FsEventKind::CHANNEL_BRIDGE,
                FsEventKind::CHANNEL_UNBRIDGE,
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

        tracing::debug!("Originating call");
        let origination_uuid = "8d47cccc-1320-445e-b9ab-23db31c8b35f";
        let actual_id = telephony.originate(OriginateRequest {
            id: origination_uuid.to_string(),
            from: "+2015557782".to_string(),
            to: "+2014007782".to_string(),
            context: "loopback-test".to_string(),
            extension: "park".to_string(),
            priority: 1,
        }).await?;

        let mut calls = HashSet::<String>::new();

        // Wait for the call to be answered, then hang up
        loop {
            let event = event_rx.recv().await
                .ok_or_else(|| {
                    tracing::error!("Channel closed, no more events");
                    anyhow::anyhow!("Channel closed, no more events")
                })?;
            match event {
                TelephonyEvent::CallAnswered { call_id } => {
                    tracing::info!("Call {} answered", call_id);
                    let req = HangupRequest { call_id: actual_id.channel_leg_id.clone() };
                    telephony.hangup(req).await?;
                },
                TelephonyEvent::CallEnded { call_id, reason } => {
                    if let Some(ref hangup_reason) = reason {
                        tracing::info!("Call {} ended with reason: {}", call_id, hangup_reason);
                    } else {
                        tracing::info!("Call {} ended", call_id);
                    }
                    calls.remove(&call_id);
                },
                TelephonyEvent::CallOriginated { call_id } => {
                    tracing::info!("Call {} originated", call_id);
                    calls.insert(call_id);
                },
                TelephonyEvent::CallLegCreated { call_id } => {
                    tracing::info!("Call {} leg created", call_id);
                    calls.insert(call_id);
                },
                TelephonyEvent::TransportDown => {
                    tracing::info!("Telephony connection lost");
                    calls.clear();
                },
                TelephonyEvent::Unknown { message } => {
                    tracing::info!("Unknown event received: {}", message);
                }
                _ => {
                    tracing::info!("Received event: {:?}", event);
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
