use std::collections::HashMap;

use serde::Deserialize;

use crate::error::OrchestratorError;

#[derive(Debug, Clone, Deserialize)]
pub struct OrchestratorConfig {
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub similarity_threshold: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub url: String,
}

impl OrchestratorConfig {
    pub fn from_env() -> Result<Self, OrchestratorError> {
        let endpoints_raw = read_required_env("AGENT_ENDPOINTS")?;
        let endpoints = parse_comma_pairs(&endpoints_raw, "AGENT_ENDPOINTS")?;

        let descriptions: HashMap<String, String> = match std::env::var("AGENT_DESCRIPTIONS") {
            Ok(val) if !val.trim().is_empty() => parse_comma_pairs(&val, "AGENT_DESCRIPTIONS")?
                .into_iter()
                .collect(),
            _ => HashMap::new(),
        };

        let agents = endpoints
            .into_iter()
            .map(|(name, url)| {
                let description = descriptions.get(&name).cloned().unwrap_or_default();
                AgentConfig {
                    name,
                    description,
                    url,
                }
            })
            .collect();

        Ok(OrchestratorConfig {
            agents,
            embedding_provider: read_optional_env("EMBEDDING_PROVIDER"),
            embedding_model: read_optional_env("EMBEDDING_MODEL"),
            similarity_threshold: parse_optional_f64_env("SIMILARITY_THRESHOLD")?,
        })
    }

    pub fn from_file(path: &str) -> Result<Self, OrchestratorError> {
        let content = std::fs::read_to_string(path).map_err(|e| OrchestratorError::Config {
            reason: format!("failed to read {path}: {e}"),
        })?;

        serde_yaml::from_str(&content).map_err(|e| OrchestratorError::Config {
            reason: format!("failed to parse {path}: {e}"),
        })
    }
}

fn read_optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
}

fn parse_optional_f64_env(name: &str) -> Result<Option<f64>, OrchestratorError> {
    match read_optional_env(name) {
        None => Ok(None),
        Some(val) => val
            .parse::<f64>()
            .map(Some)
            .map_err(|_| OrchestratorError::Config {
                reason: format!("{name} must be a valid floating-point number, got '{val}'"),
            }),
    }
}

fn read_required_env(name: &str) -> Result<String, OrchestratorError> {
    let value = std::env::var(name).unwrap_or_default();
    if value.trim().is_empty() {
        return Err(OrchestratorError::Config {
            reason: format!("environment variable {name} is required but missing or empty"),
        });
    }
    Ok(value)
}

fn parse_comma_pairs(
    input: &str,
    var_name: &str,
) -> Result<Vec<(String, String)>, OrchestratorError> {
    input
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|entry| {
            let (k, v) = entry
                .split_once('=')
                .ok_or_else(|| OrchestratorError::Config {
                    reason: format!(
                        "invalid pair '{entry}' in {var_name}, expected format 'key=value'"
                    ),
                })?;
            let key = k.trim().to_string();
            let value = v.trim().to_string();
            if key.is_empty() || value.is_empty() {
                return Err(OrchestratorError::Config {
                    reason: format!("empty key or value in pair '{entry}' in {var_name}"),
                });
            }
            Ok((key, value))
        })
        .collect()
}
