use std::sync::Arc;

use agent_sdk::async_trait;
use agent_sdk::{
    AgentError, AgentRequest, AgentResponse, HealthStatus, MicroAgent, SkillManifest,
};

/// A decorator that wraps a `MicroAgent` and enforces confidence-threshold constraints.
///
/// After the inner agent returns a response, `ConstraintEnforcer` checks whether
/// the response confidence falls below the manifest's configured threshold. If so,
/// it marks the response as escalated and sets the escalation target from the manifest.
pub struct ConstraintEnforcer {
    inner: Arc<dyn MicroAgent>,
}

impl ConstraintEnforcer {
    /// Create a new `ConstraintEnforcer` wrapping the given agent.
    pub fn new(inner: Arc<dyn MicroAgent>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl MicroAgent for ConstraintEnforcer {
    fn manifest(&self) -> &SkillManifest {
        self.inner.manifest()
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        let mut response = self.inner.invoke(request).await?;
        let manifest = self.inner.manifest();
        let threshold = manifest.constraints.confidence_threshold;

        if (response.confidence as f64) < threshold {
            response.escalated = true;
            response.escalate_to = manifest.constraints.escalate_to.clone();
        }

        Ok(response)
    }

    async fn health(&self) -> HealthStatus {
        self.inner.health().await
    }
}
