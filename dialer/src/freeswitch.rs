mod esl;
mod reader;
mod telephony;

pub mod types;

// Re-export commonly used types for cleaner imports
pub(crate) use esl::{EslEventFormat, EslSupervisor, EslSupervisorConfig};
pub(crate) use telephony::FreeswitchTelephonyAdapter;
