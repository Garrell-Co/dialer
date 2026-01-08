use anyhow::Result;
use std::collections::HashMap;

use tokio::sync::mpsc;

pub struct OriginateRequest {
    pub id: String,
    pub from: String,
    pub to: String,
    pub context: String,
    pub extension: String,
    pub priority: u8,
}

pub struct OriginateResult {
    pub channel_leg_id: String,
}

pub struct HangupRequest {
    pub call_id: String,
}

#[derive(Clone)]
pub enum CallLegEventType {
    Created,
    Ringing,
    Answered,
    Hungup { reason: String },
    Unknown { name: String },
}

#[derive(Clone)]
pub struct CallLegEvent {
    pub call_leg_id: String,
    pub at: std::time::SystemTime,
    pub kind: CallLegEventType,
    pub raw: Option<HashMap<String, String>>,
}

#[derive(Debug)]
pub enum TelephonyEvent {
    CallLegCreated {
        call_id: String,
    },
    CallOriginated {
        call_id: String,
    },
    CallEnded {
        call_id: String,
        reason: Option<String>,
    },
    CallAnswered {
        call_id: String,
    },
    TransportUp,
    TransportDown,
    Unknown {
        message: String,
    },
}

#[async_trait::async_trait]
pub trait TelephonyPort {
    async fn originate(&self, request: OriginateRequest) -> Result<OriginateResult>;
    async fn hangup(&self, request: HangupRequest) -> Result<()>;
    async fn hangup_all(&self) -> Result<()>;
    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent>;
}
