use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input: Value,
    pub output: Value,
}
