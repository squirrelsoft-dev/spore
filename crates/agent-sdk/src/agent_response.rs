use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::tool_call_record::ToolCallRecord;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentResponse {
    pub id: Uuid,
    pub output: Value,
    pub confidence: f32,
    pub escalated: bool,
    pub tool_calls: Vec<ToolCallRecord>,
}

impl AgentResponse {
    pub fn success(id: uuid::Uuid, output: serde_json::Value) -> Self {
        Self {
            id,
            output,
            confidence: 1.0,
            escalated: false,
            tool_calls: vec![],
        }
    }
}
