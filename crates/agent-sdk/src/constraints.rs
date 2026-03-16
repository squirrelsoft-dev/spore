use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalate_to: Option<String>,
    pub allowed_actions: Vec<String>,
}
