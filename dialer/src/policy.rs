pub mod manual_phone;

use crate::controller::ControllerCommand;
use crate::store::CallStore;

/// Result of a policy evaluation.
#[derive(Debug, Clone, PartialEq)]
pub enum PolicyDecision {
    Allow,
    Deny { reason: String },
}

/// Trait for evaluating whether a command should be allowed.
pub trait PolicyEngine: Send + Sync {
    fn evaluate(&self, command: &ControllerCommand, store: &dyn CallStore) -> PolicyDecision;
}
