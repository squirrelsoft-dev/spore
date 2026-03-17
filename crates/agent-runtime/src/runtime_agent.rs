use std::sync::Arc;

use agent_sdk::async_trait;
use agent_sdk::{
    AgentError, AgentRequest, AgentResponse, HealthStatus, MicroAgent, SkillManifest,
};
use serde_json::Value;
use tool_registry::ToolRegistry;

use crate::provider::BuiltAgent;

/// A runtime agent that bridges a rig-core `Agent` with the spore `MicroAgent` trait.
///
/// Holds a skill manifest describing the agent's capabilities, a provider-backed
/// `BuiltAgent` for LLM interaction, and a shared `ToolRegistry` for tool access.
pub struct RuntimeAgent {
    manifest: SkillManifest,
    agent: BuiltAgent,
    #[allow(dead_code)]
    registry: Arc<ToolRegistry>,
}

impl RuntimeAgent {
    /// Create a new `RuntimeAgent` with the given manifest, agent, and tool registry.
    pub fn new(manifest: SkillManifest, agent: BuiltAgent, registry: Arc<ToolRegistry>) -> Self {
        Self {
            manifest,
            agent,
            registry,
        }
    }
}

#[async_trait]
impl MicroAgent for RuntimeAgent {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        let output = self
            .agent
            .prompt(&request.input)
            .await
            .map_err(|e| AgentError::Internal(e.to_string()))?;
        Ok(AgentResponse::success(request.id, Value::String(output)))
    }

    async fn health(&self) -> HealthStatus {
        HealthStatus::Healthy
    }
}
