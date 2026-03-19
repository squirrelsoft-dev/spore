use agent_sdk::AgentError;
use std::fmt;

#[derive(Debug, Clone)]
pub enum OrchestratorError {
    NoRoute { input: String },
    AgentUnavailable { name: String, reason: String },
    EscalationFailed { chain: Vec<String>, reason: String },
    HttpError { url: String, reason: String },
}

impl fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OrchestratorError::NoRoute { input } => {
                write!(f, "No route found for input: {}", input)
            }
            OrchestratorError::AgentUnavailable { name, reason } => {
                write!(f, "Agent '{}' unavailable: {}", name, reason)
            }
            OrchestratorError::EscalationFailed { chain, reason } => {
                write!(
                    f,
                    "Escalation failed through chain [{}]: {}",
                    chain.join(" -> "),
                    reason
                )
            }
            OrchestratorError::HttpError { url, reason } => {
                write!(f, "HTTP error calling {}: {}", url, reason)
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<OrchestratorError> for AgentError {
    fn from(err: OrchestratorError) -> Self {
        AgentError::Internal(err.to_string())
    }
}
