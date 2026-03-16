use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolEntry {
    pub name: String,
    pub version: String,
    pub endpoint: String,
}
