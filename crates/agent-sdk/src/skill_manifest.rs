use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::constraints::Constraints;
use crate::model_config::ModelConfig;
use crate::output_schema::OutputSchema;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SkillManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub model: ModelConfig,
    pub preamble: String,
    pub tools: Vec<String>,
    pub constraints: Constraints,
    pub output: OutputSchema,
}
