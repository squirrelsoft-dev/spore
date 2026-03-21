use std::collections::HashMap;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, HealthStatus, MicroAgent, SkillManifest,
};
use futures::future::join_all;
use serde_json::json;

use crate::agent_endpoint::AgentEndpoint;
use crate::config::OrchestratorConfig;
use crate::error::OrchestratorError;
use crate::semantic_router::SemanticRouter;
use rig::embeddings::EmbeddingModel;

const MAX_ESCALATION_DEPTH: usize = 5;

pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
    semantic_router: Option<SemanticRouter>,
}

impl Orchestrator {
    pub fn new(
        manifest: SkillManifest,
        agents: Vec<AgentEndpoint>,
        semantic_router: Option<SemanticRouter>,
    ) -> Self {
        let registry = agents
            .into_iter()
            .map(|agent| (agent.name.clone(), agent))
            .collect();
        Self {
            registry,
            manifest,
            semantic_router,
        }
    }

    pub fn register(&mut self, endpoint: AgentEndpoint) {
        self.registry.insert(endpoint.name.clone(), endpoint);
    }

    pub fn route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError> {
        if let Some(endpoint) = self.route_by_target_agent(request) {
            return Ok(endpoint);
        }

        Err(OrchestratorError::NoRoute {
            input: request.input.clone(),
        })
    }

    pub async fn dispatch(
        &self,
        request: AgentRequest,
    ) -> Result<AgentResponse, OrchestratorError> {
        let endpoint = self.route(&request)?;
        let response = self.try_invoke(endpoint, &request).await?;
        let chain = vec![endpoint.name.clone()];
        self.handle_escalation(response, &request, chain).await
    }

    pub fn from_config(
        config: OrchestratorConfig,
        manifest: SkillManifest,
    ) -> Result<Self, OrchestratorError> {
        let client = build_shared_client();
        let agents: Vec<AgentEndpoint> = config
            .agents
            .into_iter()
            .map(|ac| AgentEndpoint::new(ac.name, ac.description, ac.url, client.clone()))
            .collect();

        Ok(Self::new(manifest, agents, None))
    }

    /// Extracts `target_agent` from request context and looks up the registry.
    /// Returns `None` if context is absent, `target_agent` is missing or not a
    /// string, or the named agent is not registered.
    fn route_by_target_agent(&self, request: &AgentRequest) -> Option<&AgentEndpoint> {
        let context = request.context.as_ref()?;
        let target = context.get("target_agent")?.as_str()?;
        self.registry.get(target)
    }

    /// Routes using the semantic router to find the best matching agent.
    /// Returns `NoRoute` if no semantic router is configured or no match found.
    async fn route_semantic<M: EmbeddingModel>(
        &self,
        model: &M,
        request: &AgentRequest,
    ) -> Result<&AgentEndpoint, OrchestratorError> {
        let router = self.semantic_router.as_ref().ok_or_else(|| {
            OrchestratorError::NoRoute {
                input: request.input.clone(),
            }
        })?;

        let agent_name = router.route(model, request).await?;

        self.registry.get(&agent_name).ok_or_else(|| {
            OrchestratorError::NoRoute {
                input: request.input.clone(),
            }
        })
    }

    /// Three-phase routing: (1) target_agent context, (2) semantic similarity,
    /// (3) NoRoute error.
    async fn route_with_model<M: EmbeddingModel>(
        &self,
        request: &AgentRequest,
        model: &M,
    ) -> Result<&AgentEndpoint, OrchestratorError> {
        if let Some(endpoint) = self.route_by_target_agent(request) {
            return Ok(endpoint);
        }

        self.route_semantic(model, request).await
    }

    /// Dispatches a request using three-phase routing with an embedding model.
    /// Falls back through target_agent, semantic similarity, then NoRoute.
    pub async fn dispatch_with_model<M: EmbeddingModel>(
        &self,
        request: AgentRequest,
        model: &M,
    ) -> Result<AgentResponse, OrchestratorError> {
        let endpoint = self.route_with_model(&request, model).await?;
        let response = self.try_invoke(endpoint, &request).await?;
        let chain = vec![endpoint.name.clone()];
        self.handle_escalation(response, &request, chain).await
    }

    /// Builds an orchestrator from config with a semantic router powered by an
    /// embedding model. Pre-computes embeddings for all configured agent
    /// descriptions.
    pub async fn from_config_with_model<M: EmbeddingModel>(
        config: OrchestratorConfig,
        manifest: SkillManifest,
        model: &M,
        similarity_threshold: f64,
    ) -> Result<Self, OrchestratorError> {
        let client = build_shared_client();
        let agent_pairs: Vec<(String, String)> = config
            .agents
            .iter()
            .map(|ac| (ac.name.clone(), ac.description.clone()))
            .collect();

        let router =
            SemanticRouter::new(model, agent_pairs, similarity_threshold).await?;

        let agents: Vec<AgentEndpoint> = config
            .agents
            .into_iter()
            .map(|ac| AgentEndpoint::new(ac.name, ac.description, ac.url, client.clone()))
            .collect();

        Ok(Self::new(manifest, agents, Some(router)))
    }

    /// Checks agent health before invoking. This adds an HTTP round-trip per
    /// dispatch. A future optimization could cache health status with a TTL.
    async fn try_invoke(
        &self,
        endpoint: &AgentEndpoint,
        request: &AgentRequest,
    ) -> Result<AgentResponse, OrchestratorError> {
        let status = endpoint.health().await?;
        match status {
            HealthStatus::Unhealthy(reason) => Err(OrchestratorError::AgentUnavailable {
                name: endpoint.name.clone(),
                reason,
            }),
            HealthStatus::Healthy | HealthStatus::Degraded(_) => endpoint.invoke(request).await,
        }
    }

    async fn handle_escalation(
        &self,
        response: AgentResponse,
        original_request: &AgentRequest,
        chain: Vec<String>,
    ) -> Result<AgentResponse, OrchestratorError> {
        let mut current_response = response;
        let mut current_chain = chain;

        loop {
            if !current_response.escalated {
                return Ok(current_response);
            }

            let target_name = match current_response.escalate_to {
                Some(ref name) => name.clone(),
                None => {
                    tracing::warn!(
                        source_agent = %current_chain.last().unwrap_or(&"unknown".to_string()),
                        confidence = current_response.confidence,
                        chain = ?current_chain,
                        "agent signaled escalation but provided no target"
                    );
                    return Ok(current_response);
                }
            };

            self.validate_escalation_depth(&current_chain)?;
            self.validate_no_cycle(&current_chain, &target_name)?;

            let endpoint =
                self.lookup_escalation_target(&target_name, &current_chain)?;
            let new_request = build_escalation_request(
                original_request,
                &target_name,
                &current_chain,
            );

            tracing::info!(
                source_agent = %current_chain.last().unwrap_or(&"unknown".to_string()),
                target_agent = %target_name,
                confidence = current_response.confidence,
                depth = current_chain.len(),
                chain = ?current_chain,
                "escalating request to next agent"
            );

            current_response = self.try_invoke(endpoint, &new_request).await?;
            current_chain.push(target_name);
        }
    }

    fn validate_escalation_depth(
        &self,
        chain: &[String],
    ) -> Result<(), OrchestratorError> {
        if chain.len() >= MAX_ESCALATION_DEPTH {
            tracing::error!(
                depth = chain.len(),
                max_depth = MAX_ESCALATION_DEPTH,
                chain = ?chain,
                "escalation depth exceeded"
            );
            return Err(OrchestratorError::EscalationFailed {
                chain: chain.to_vec(),
                reason: format!(
                    "max escalation depth of {} exceeded",
                    MAX_ESCALATION_DEPTH
                ),
            });
        }
        Ok(())
    }

    fn validate_no_cycle(
        &self,
        chain: &[String],
        target_name: &str,
    ) -> Result<(), OrchestratorError> {
        if chain.contains(&target_name.to_string()) {
            tracing::error!(
                target_agent = %target_name,
                chain = ?chain,
                "escalation cycle detected"
            );
            return Err(OrchestratorError::EscalationFailed {
                chain: chain.to_vec(),
                reason: format!("cycle detected: '{}' already in chain", target_name),
            });
        }
        Ok(())
    }

    fn lookup_escalation_target<'a>(
        &'a self,
        target_name: &str,
        chain: &[String],
    ) -> Result<&'a AgentEndpoint, OrchestratorError> {
        self.registry.get(target_name).ok_or_else(|| {
            OrchestratorError::EscalationFailed {
                chain: chain.to_vec(),
                reason: format!("escalation target '{}' not found in registry", target_name),
            }
        })
    }
}

#[async_trait]
impl MicroAgent for Orchestrator {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        self.dispatch(request)
            .await
            .map_err(|e| AgentError::Internal(e.to_string()))
    }

    async fn health(&self) -> HealthStatus {
        let futures: Vec<_> = self
            .registry
            .values()
            .map(|agent| async { agent.health().await })
            .collect();

        let results = join_all(futures).await;

        let statuses: Vec<HealthStatus> = results
            .into_iter()
            .map(|r| r.unwrap_or_else(|e| HealthStatus::Unhealthy(e.to_string())))
            .collect();

        aggregate_health(statuses)
    }
}

fn aggregate_health(statuses: Vec<HealthStatus>) -> HealthStatus {
    if statuses.is_empty() {
        return HealthStatus::Healthy;
    }

    let total = statuses.len();
    let mut healthy_count = 0;
    let mut degraded_count = 0;

    for status in &statuses {
        match status {
            HealthStatus::Healthy => healthy_count += 1,
            HealthStatus::Degraded(_) => degraded_count += 1,
            HealthStatus::Unhealthy(_) => {}
        }
    }

    if healthy_count > 0 {
        HealthStatus::Healthy
    } else if degraded_count > 0 {
        HealthStatus::Degraded(format!("{} of {} agents degraded", degraded_count, total))
    } else {
        HealthStatus::Unhealthy(format!("All {} agents unhealthy", total))
    }
}

fn build_shared_client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .expect("failed to build HTTP client")
}

fn build_escalation_request(
    original_request: &AgentRequest,
    target_name: &str,
    chain: &[String],
) -> AgentRequest {
    AgentRequest {
        id: original_request.id,
        input: original_request.input.clone(),
        context: Some(json!({"target_agent": target_name})),
        caller: chain.last().cloned(),
    }
}
