use anyhow::Result;
use tokio::sync::broadcast;

use crate::telephony::{TelephonyPort, OriginateRequest, HangupRequest, CallLegEvent};

use super::esl::{EslClient, EslClientConfig};

pub struct FreeswitchTelephonyAdapter {
    freeswitch_client: EslClient,
}

impl FreeswitchTelephonyAdapter {
    pub fn new(config: EslClientConfig) -> Self {
        let esl_client = EslClient::new(config);
        Self { freeswitch_client: esl_client }
    }
}

impl TelephonyPort for FreeswitchTelephonyAdapter {
    fn connect(&self) -> Result<()> {
        Ok(())
    }

    fn subscribe(&self) -> Result<broadcast::Receiver<CallLegEvent>> {
        Ok(broadcast::channel(100).1)
    }

    fn originate(&self, request: OriginateRequest) -> Result<()> {
        Ok(())
    }

    fn hangup(&self, request: HangupRequest) -> Result<()> {
        Ok(())
    }
}