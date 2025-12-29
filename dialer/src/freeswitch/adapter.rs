use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{broadcast};

use crate::telephony::{TelephonyPort, OriginateRequest, HangupRequest, CallLegEvent};

use super::esl::{EventSocket};

pub struct FreeswitchTelephonyAdapter {
    event_socket: Arc<dyn EventSocket>,
}

impl FreeswitchTelephonyAdapter {
    pub fn new(event_socket: Arc<dyn EventSocket>) -> Self {
        Self { event_socket: event_socket.clone() }
    }
}

impl TelephonyPort for FreeswitchTelephonyAdapter {
    async fn connect(&self) -> Result<()> {
        self.event_socket.connect().await?;
        Ok(())
    }

    fn subscribe(&self) -> Result<broadcast::Receiver<CallLegEvent>> {
        let (_, rx) = broadcast::channel::<CallLegEvent>(100);
        Ok(rx)
    }

    fn originate(&self, request: OriginateRequest) -> Result<()> {
        Ok(())
    }

    fn hangup(&self, request: HangupRequest) -> Result<()> {
        Ok(())
    }
}