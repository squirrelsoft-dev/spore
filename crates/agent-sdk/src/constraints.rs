use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,
    pub escalate_to: String,
    pub allowed_actions: Vec<String>,
}
