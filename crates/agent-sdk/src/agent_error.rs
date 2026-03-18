use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AgentError {
    ToolCallFailed {
        tool: String,
        reason: String,
    },
    ConfidenceTooLow {
        confidence: f32,
        threshold: f32,
    },
    MaxTurnsExceeded {
        turns: u32,
    },
    ActionDisallowed {
        action: String,
        allowed: Vec<String>,
    },
    Internal(String),
}

impl fmt::Display for AgentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentError::ToolCallFailed { tool, reason } => {
                write!(f, "Tool call '{}' failed: {}", tool, reason)
            }
            AgentError::ConfidenceTooLow {
                confidence,
                threshold,
            } => {
                write!(
                    f,
                    "Confidence {:.2} is below threshold {:.2}",
                    confidence, threshold
                )
            }
            AgentError::MaxTurnsExceeded { turns } => {
                write!(f, "Max turns exceeded: {} turns used", turns)
            }
            AgentError::ActionDisallowed { action, allowed } => {
                write!(
                    f,
                    "Action '{}' is not in allowed actions: [{}]",
                    action,
                    allowed.join(", ")
                )
            }
            AgentError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for AgentError {}
