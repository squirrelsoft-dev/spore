use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::mcp_handle::McpHandle;

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct ToolEntry {
    pub name: String,
    pub version: String,
    pub endpoint: String,
    #[serde(skip)]
    #[schemars(skip)]
    pub handle: Option<McpHandle>,
}

impl Clone for ToolEntry {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            version: self.version.clone(),
            endpoint: self.endpoint.clone(),
            handle: None,
        }
    }
}

impl PartialEq for ToolEntry {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.version == other.version && self.endpoint == other.endpoint
    }
}
