use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
