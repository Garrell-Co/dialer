use anyhow::Result;
use crate::telephony::{TelephonyPort, OriginateRequest, CallLegEventType};

pub struct HelloWorld {
    telephony_port: Box<dyn TelephonyPort>,
}

impl HelloWorld {
    pub fn new(telephony_port: Box<dyn TelephonyPort>) -> Self {
        Self { telephony_port }
    }

    pub async fn start(&self) -> Result<()> {
        let mut receiver = self.telephony_port.subscribe()?;

        loop {
            self.telephony_port.originate(OriginateRequest {
                from: "+2015557782".to_string(),
                to: "+2014007782".to_string(),
                context: "default".to_string(),
                extension: "1001".to_string(),
                priority: 1,
            })?;

            let event = receiver.recv().await
                .map_err(|e| anyhow::anyhow!("Failed to receive event: {}", e))?;
            match event.kind {
                CallLegEventType::Ringing => {
                    println!("Ringing: {}", event.call_leg_id);
                }
                CallLegEventType::Answered => {
                    println!("Answered: {}", event.call_leg_id);
                }
                CallLegEventType::Hungup { reason } => {
                    println!("Hungup: {} - {}", event.call_leg_id, reason);
                }
                _ => {}
            }
        }
    }
}