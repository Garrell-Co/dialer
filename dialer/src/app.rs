use std::collections::HashSet;

use tracing;

use crate::freeswitch::types::FsEventKind;
use crate::freeswitch::{EslEventFormat, EslSupervisorConfig, FreeswitchTelephonyAdapter};
use crate::telephony::{OriginateRequest, TelephonyEvent, TelephonyPort};

#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub worker_name: String,
    pub log_level: String,

    pub freeswitch_host: String,
    pub freeswitch_port: u16,
    pub freeswitch_password: String,
}

impl WorkerConfig {
    pub fn from_env_and_args(default_worker_name: &str) -> anyhow::Result<Self> {
        let worker_name = std::env::var("WORKER_NAME").unwrap_or_else(|_| default_worker_name.into());
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

/// Run a worker that originates a single call and monitors it until completion
pub async fn run_worker(cfg: WorkerConfig, originate_req: OriginateRequest) -> anyhow::Result<()> {
    tracing::info!("Starting worker: {}", cfg.worker_name);

    tracing::debug!(
        host = %cfg.freeswitch_host,
        port = cfg.freeswitch_port,
        "Connecting to FreeSWITCH"
    );

    let supervisor_config = EslSupervisorConfig {
        host: cfg.freeswitch_host.clone(),
        port: cfg.freeswitch_port,
        password: cfg.freeswitch_password.clone(),
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
            FsEventKind::CHANNEL_HANGUP_COMPLETE
        ],
    };

    let mut telephony = FreeswitchTelephonyAdapter::connect(supervisor_config).await?;

    let mut event_rx = telephony.take_event_rx();

    // Wait for transport to be up
    loop {
        let event = event_rx.recv().await.ok_or_else(|| {
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
            }
            _ => {
                tracing::debug!("Received event while waiting for transport up: {:?}", event);
            }
        }
    }

    tracing::info!("Originating call: {:?}", originate_req.destination);
    let actual_id = telephony.originate(originate_req).await?;

    let mut calls = HashSet::<String>::new();
    let originated_call_id = actual_id.channel_leg_id.clone();

    // Wait for the call to complete
    loop {
        let event = event_rx.recv().await.ok_or_else(|| {
            tracing::error!("Channel closed, no more events");
            anyhow::anyhow!("Channel closed, no more events")
        })?;
        match event {
            TelephonyEvent::CallAnswered { call_id } => {
                tracing::info!("Call {} answered", call_id);
            }
            TelephonyEvent::CallEnded { call_id, reason } => {
                if let Some(ref hangup_reason) = reason {
                    tracing::info!("Call {} ended with reason: {}", call_id, hangup_reason);
                } else {
                    tracing::info!("Call {} ended", call_id);
                }
                calls.remove(&call_id);
                
                // If this is the call we originated, break the loop
                if call_id == originated_call_id {
                    tracing::info!("Originated call ended, exiting");
                    break;
                }
            }
            TelephonyEvent::CallOriginated { call_id } => {
                tracing::info!("Call {} originated", call_id);
                calls.insert(call_id);
            }
            TelephonyEvent::CallLegCreated { call_id } => {
                tracing::info!("Call leg {} created", call_id);
                calls.insert(call_id);
            }
            TelephonyEvent::TransportDown => {
                tracing::info!("Telephony connection lost");
                calls.clear();
            }
            TelephonyEvent::Unknown { message } => {
                tracing::debug!("Unknown event received: {}", message);
            }
            _ => {
                tracing::debug!("Received event: {:?}", event);
            }
        }
    }
    
    Ok(())
}

