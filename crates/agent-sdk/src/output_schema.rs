use std::collections::HashMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const ALLOWED_OUTPUT_FORMATS: &[&str] = &["json", "structured_json", "text"];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct OutputSchema {
    pub format: String,
    pub schema: HashMap<String, String>,
}
