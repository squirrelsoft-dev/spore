use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::tool_call_record::ToolCallRecord;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AgentResponse {
    pub id: Uuid,
    pub output: Value,
    pub confidence: f32,
    pub escalated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub escalate_to: Option<String>,
    pub tool_calls: Vec<ToolCallRecord>,
}

impl AgentResponse {
    pub fn success(id: Uuid, output: Value) -> Self {
        Self {
            id,
            output,
            confidence: 1.0,
            escalated: false,
            escalate_to: None,
            tool_calls: vec![],
        }
    }
}
