use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentRequest {
    pub id: Uuid,
    pub input: String,
    pub context: Option<Value>,
    pub caller: Option<String>,
}

impl AgentRequest {
    pub fn new(input: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            input,
            context: None,
            caller: None,
        }
    }
}
