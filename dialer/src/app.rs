use std::sync::Arc;

use tracing;

use crate::telephony::{TelephonyPort, OriginateRequest, CallLegEventType};
use crate::freeswitch::adapter::FreeswitchTelephonyAdapter;
use crate::freeswitch::esl::{EslClient, EslClientConfig, EslEventFormat};


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


struct DialerWorker<T: TelephonyPort> {
    cfg: WorkerConfig,
    telephony: T,
}


impl<T: TelephonyPort> DialerWorker<T> {
    pub fn new(cfg: WorkerConfig, telephony: T) -> Self {
        Self { cfg, telephony }
    }
}

trait Worker {
    async fn run(&self) -> anyhow::Result<()>;
}

impl<T: TelephonyPort + Send + Sync> Worker for DialerWorker<T> {
    async fn run(&self) -> anyhow::Result<()> {
        tracing::info!("Starting dialer worker");
        let mut receiver = self.telephony.subscribe()?;
        tracing::debug!("Subscribed to telephony events");

        self.telephony.connect().await?;

        loop {
            tracing::debug!("Originating call");
            self.telephony.originate(OriginateRequest {
                from: "+2015557782".to_string(),
                to: "+2014007782".to_string(),
                context: "default".to_string(),
                extension: "1001".to_string(),
                priority: 1,
            })?;

            let event = receiver.recv().await
                .map_err(|e| {
                    tracing::error!(error = %e, "Failed to receive event");
                    anyhow::anyhow!("Failed to receive event: {}", e)
                })?;
            match event.kind {
                CallLegEventType::Ringing => {
                    tracing::info!(call_leg_id = %event.call_leg_id, "Call leg ringing");
                }
                CallLegEventType::Answered => {
                    tracing::info!(call_leg_id = %event.call_leg_id, "Call leg answered");
                }
                CallLegEventType::Hungup { reason } => {
                    tracing::info!(call_leg_id = %event.call_leg_id, reason = %reason, "Call leg hung up");
                }
                _ => {
                    tracing::debug!(call_leg_id = %event.call_leg_id, "Received event");
                }
            }
        }
    }
}


async fn build_worker(cfg: WorkerConfig) -> anyhow::Result<impl Worker> {
    tracing::debug!(
        host = %cfg.freeswitch_host,
        port = cfg.freeswitch_port,
        "Connecting to FreeSWITCH"
    );

    let esl_client = EslClient::new(EslClientConfig {
        host: cfg.freeswitch_host.clone(),
        port: cfg.freeswitch_port.clone(),
        password: cfg.freeswitch_password.clone(),
        event_format: EslEventFormat::Json,
    });

    let telephony = FreeswitchTelephonyAdapter::new(Arc::new(esl_client));

    Ok(DialerWorker::new(cfg, telephony))
}


pub async fn run_worker(cfg: WorkerConfig) -> anyhow::Result<()> {
    let worker = build_worker(cfg).await?;
    worker.run().await
}
