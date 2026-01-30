pub mod commands;

use crate::telephony::DestinationType;

/// Commands sent from the API to the controller loop.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "action")]
pub enum ControllerCommand {
    Dial {
        destination: DestinationType,
        from: String,
        caller_id_name: Option<String>,
    },
    Hangup {
        call_id: String,
    },
    Hold {
        call_id: String,
    },
    Resume {
        call_id: String,
    },
    Transfer {
        call_id: String,
        destination: DestinationType,
    },
}
