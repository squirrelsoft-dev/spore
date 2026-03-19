use std::collections::HashMap;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent,
    ModelConfig, OutputSchema, SkillManifest,
};
use futures::future::join_all;
use serde_json::json;

use crate::agent_endpoint::AgentEndpoint;
use crate::config::OrchestratorConfig;
use crate::error::OrchestratorError;

const MAX_ESCALATION_DEPTH: usize = 5;

pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
}

impl Orchestrator {
    pub fn new(manifest: SkillManifest, agents: Vec<AgentEndpoint>) -> Self {
        let registry = agents
            .into_iter()
            .map(|agent| (agent.name.clone(), agent))
            .collect();
        Self { registry, manifest }
    }

    pub fn register(&mut self, endpoint: AgentEndpoint) {
        self.registry.insert(endpoint.name.clone(), endpoint);
    }

    pub fn route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError> {
        if let Some(endpoint) = self.route_by_target_agent(request) {
            return Ok(endpoint);
        }

        if let Some(endpoint) = self.route_by_description_match(request) {
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

    pub fn from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
        let agents = config
            .agents
            .into_iter()
            .map(|ac| AgentEndpoint::new(ac.name, ac.description, ac.url))
            .collect();

        let manifest = build_default_manifest();
        Ok(Self::new(manifest, agents))
    }

    /// Phase 1: Extract target agent name from request context.
    /// If `context` contains `"target_agent"` but the value is not a string
    /// (e.g. a number or object), `as_str()` returns `None` and we fall
    /// through to the description-match heuristic.
    fn route_by_target_agent(&self, request: &AgentRequest) -> Option<&AgentEndpoint> {
        let context = request.context.as_ref()?;
        let target = context.get("target_agent")?.as_str()?;
        self.registry.get(target)
    }

    /// Phase 2 (fallback): Substring heuristic matching against endpoint descriptions.
    /// This is a placeholder until `SemanticRouter` (issue #16) replaces it with
    /// ranked scoring. Note: HashMap iteration order is nondeterministic, so when
    /// multiple endpoints match, the one returned is arbitrary.
    fn route_by_description_match(&self, request: &AgentRequest) -> Option<&AgentEndpoint> {
        let input_lower = request.input.to_lowercase();
        self.registry.values().find(|endpoint| {
            let desc_lower = endpoint.description.to_lowercase();
            desc_lower
                .split_whitespace()
                .filter(|word| word.len() >= 3)
                .any(|word| input_lower.contains(word))
        })
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
        if !response.escalated {
            return Ok(response);
        }

        let target_name = match response.escalate_to {
            Some(ref name) => name.clone(),
            None => return Ok(response),
        };

        self.validate_escalation_depth(&chain)?;
        self.validate_no_cycle(&chain, &target_name)?;

        let endpoint = self.lookup_escalation_target(&target_name, &chain)?;
        let new_request = build_escalation_request(original_request, &target_name, &chain);

        let escalated_response = self.try_invoke(endpoint, &new_request).await?;
        let mut updated_chain = chain;
        updated_chain.push(target_name);

        Box::pin(self.handle_escalation(escalated_response, original_request, updated_chain))
            .await
    }

    fn validate_escalation_depth(
        &self,
        chain: &[String],
    ) -> Result<(), OrchestratorError> {
        if chain.len() >= MAX_ESCALATION_DEPTH {
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

fn build_default_manifest() -> SkillManifest {
    SkillManifest {
        name: "orchestrator".to_string(),
        version: "0.1.0".to_string(),
        description: "Routes requests to specialized agents".to_string(),
        model: ModelConfig {
            provider: "none".into(),
            name: "none".into(),
            temperature: 0.0,
        },
        preamble: String::new(),
        tools: vec![],
        constraints: Constraints {
            max_turns: 1,
            confidence_threshold: 0.0,
            escalate_to: None,
            allowed_actions: vec![],
        },
        output: OutputSchema {
            format: "json".into(),
            schema: HashMap::new(),
        },
    }
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
