use anyhow::{Result};
use tokio::sync::{mpsc};

use crate::telephony::{HangupRequest, OriginateRequest, TelephonyEvent, TelephonyPort};
use super::esl::{EslCommand};
use super::connector;
use super::connector::{EslClientConfig};


pub struct FreeswitchTelephonyAdapter {
    domain_rx: Option<mpsc::Receiver<TelephonyEvent>>,
    cmd_tx: mpsc::Sender<EslCommand>
}

impl FreeswitchTelephonyAdapter {
    pub async fn connect(
        connector: EslClientConfig,
    ) -> Result<Self> {
        let (cmd_tx, cmd_rx) = mpsc::channel::<EslCommand>(100);
        let (domain_tx, domain_rx) = mpsc::channel::<TelephonyEvent>(100);

        let connector_config = connector.clone();
        tokio::spawn(connector::connection_manager_task(connector_config, domain_tx, cmd_rx));

        Ok( Self {
            domain_rx: Some(domain_rx),
            cmd_tx
        } )
    }
}


#[async_trait::async_trait]
impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn originate(&self, request: OriginateRequest) -> Result<()> {
        Ok(())
    }

    async fn hangup(&self, request: HangupRequest) -> Result<()> {
        Ok(())
    }

    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent> {
        self.domain_rx.take().expect("domain_rx already taken")
    }
}