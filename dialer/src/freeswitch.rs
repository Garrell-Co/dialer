mod telephony;
mod esl;
mod parser;

// Re-export commonly used types for cleaner imports
pub(crate) use telephony::FreeswitchTelephonyAdapter;
pub(crate) use esl::{EslEventFormat, EslSupervisor, EslSupervisorConfig};