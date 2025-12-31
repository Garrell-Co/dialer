use tracing;

use crate::telephony::{OriginateRequest, TelephonyPort};
use crate::freeswitch::telephony::{FreeswitchTelephonyAdapter};
use crate::freeswitch::esl::{EslClientConfig, EslEventFormat};


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
    telephony: T,
}

trait Worker {
    async fn run(&mut self) -> anyhow::Result<()>;
}

impl<T: TelephonyPort + Send + Sync> Worker for DialerWorker<T> {
    async fn run(&mut self) -> anyhow::Result<()> {
        tracing::info!("Starting dialer worker");

        tracing::debug!("Originating call");
        self.telephony.originate(OriginateRequest {
            from: "+2015557782".to_string(),
            to: "+2014007782".to_string(),
            context: "default".to_string(),
            extension: "1001".to_string(),
            priority: 1,
        }).await?;

        let mut event_rx = self.telephony.take_event_rx();

        loop {
            let event = event_rx.recv().await
                .ok_or_else(|| {
                    tracing::error!("Channel closed, no more events");
                    anyhow::anyhow!("Channel closed, no more events")
                })?;
            match event {
                _ => {
                    tracing::debug!("Received event");
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

    let client_config = EslClientConfig {
                                            host: cfg.freeswitch_host.clone(),
                                            port: cfg.freeswitch_port.clone(),
                                            password: cfg.freeswitch_password.clone(),
                                            event_format: EslEventFormat::Plain,
                                        };

    let telephony = FreeswitchTelephonyAdapter::connect(&client_config, &client_config).await?;

    Ok(DialerWorker { telephony })
}


pub async fn run_worker(cfg: WorkerConfig) -> anyhow::Result<()> {
    let mut worker = build_worker(cfg).await?;
    worker.run().await
}
