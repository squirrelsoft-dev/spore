use async_trait::async_trait;

use crate::agent_error::AgentError;
use crate::agent_request::AgentRequest;
use crate::agent_response::AgentResponse;
use crate::health_status::HealthStatus;
use crate::skill_manifest::SkillManifest;

/// Core trait for all micro-agents.
///
/// Uses `#[async_trait]` instead of native async trait methods because native
/// async methods are not dyn-compatible — the orchestrator requires `Box<dyn MicroAgent>`.
#[async_trait]
pub trait MicroAgent: Send + Sync {
    fn manifest(&self) -> &SkillManifest;
    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>;
    async fn health(&self) -> HealthStatus;
}
