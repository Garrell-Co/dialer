use anyhow::Result;

use tokio::sync::mpsc;

/// Destination type for call origination
/// 
/// Examples:
/// - Loopback: `DestinationType::Loopback { extension: "1000".into(), context: "default".into() }`
/// - Registered user: `DestinationType::RegisteredUser { user: "1001".into(), domain: Some("example.com".into()) }`
/// - External/PSTN: `DestinationType::External { destination: "+1234567890@gateway.com".into() }`
/// - Gateway/Trunk: `DestinationType::Gateway { gateway_name: "my_gateway".into(), number: "+1234567890".into() }`
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum DestinationType {
    /// Loopback call for internal routing
    /// Used for internal call routing through dialplan
    Loopback {
        extension: String,
        context: String,
    },
    /// Call to a registered user/endpoint
    /// Used to call registered users (e.g., SIP users, softphones)
    RegisteredUser {
        user: String,
        domain: Option<String>, // If None, uses default domain
    },
    /// External/PSTN call
    /// Used for outbound calls to external destinations
    /// Destination format depends on the telephony adapter implementation
    External {
        destination: String,
    },
    /// Call through a specific gateway/trunk
    /// Used to call through a configured gateway or trunk
    Gateway {
        gateway_name: String,
        number: String,
    },
}

/// Request to originate a call
#[allow(dead_code)]
pub struct OriginateRequest {
    /// Unique identifier for this call origination
    pub id: String,
    /// Caller ID number (e.g., "1000" or "+1234567890")
    pub from: String,
    /// Optional caller ID name (e.g., "John Doe")
    pub caller_id_name: Option<String>,
    /// Destination for the call
    pub destination: DestinationType,
    /// Application to execute when call is answered
    /// Format depends on the telephony adapter implementation
    /// If None, adapter-specific default will be used
    pub application: Option<String>,
}

pub struct OriginateResult {
    pub channel_leg_id: String,
}

pub struct HangupRequest {
    pub call_id: String,
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

#[allow(dead_code)]
#[async_trait::async_trait]
pub trait TelephonyPort {
    async fn originate(&self, request: OriginateRequest) -> Result<OriginateResult>;
    async fn hangup(&self, request: HangupRequest) -> Result<()>;
    async fn hangup_all(&self) -> Result<()>;
    fn take_event_rx(&mut self) -> mpsc::Receiver<TelephonyEvent>;
}
