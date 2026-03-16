use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    ToolNotFound { name: String },
    DuplicateEntry { name: String },
    ConnectionFailed { endpoint: String, reason: String },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::ToolNotFound { name } => {
                write!(f, "tool not found: '{}'", name)
            }
            RegistryError::DuplicateEntry { name } => {
                write!(f, "duplicate tool entry: '{}'", name)
            }
            RegistryError::ConnectionFailed { endpoint, reason } => {
                write!(f, "connection to '{}' failed: {}", endpoint, reason)
            }
        }
    }
}

impl std::error::Error for RegistryError {}
