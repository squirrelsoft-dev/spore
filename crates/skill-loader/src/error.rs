use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum SkillError {
    IoError {
        path: PathBuf,
        source: String,
    },
    ParseError {
        path: PathBuf,
        source: String,
    },
    ValidationError {
        skill_name: String,
        reasons: Vec<String>,
    },
}

impl fmt::Display for SkillError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SkillError::IoError { path, source } => {
                write!(f, "IO error reading {}: {}", path.display(), source)
            }
            SkillError::ParseError { path, source } => {
                write!(f, "parse error in {}: {}", path.display(), source)
            }
            SkillError::ValidationError {
                skill_name,
                reasons,
            } => {
                write!(
                    f,
                    "validation error for skill '{}': {}",
                    skill_name,
                    reasons.join("; ")
                )
            }
        }
    }
}

impl std::error::Error for SkillError {}
