use anyhow::Result;
use std::collections::HashMap;

use tokio::sync::broadcast;

pub struct OriginateRequest {
    pub from: String,
    pub to: String,
    pub context: String,
    pub extension: String,
    pub priority: u8,
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

pub trait TelephonyPort {
    fn connect(&self) -> Result<()>;
    fn subscribe(&self) -> Result<broadcast::Receiver<CallLegEvent>>;
    fn originate(&self, request: OriginateRequest) -> Result<()>;
    fn hangup(&self, request: HangupRequest) -> Result<()>;
}