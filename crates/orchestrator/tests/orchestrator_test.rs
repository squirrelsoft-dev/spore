use std::collections::HashMap;
use std::sync::Arc;

use agent_sdk::{
    AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent, ModelConfig, OutputSchema,
    SkillManifest,
};
use axum::routing::{get, post};
use axum::{Json, Router};
use orchestrator::agent_endpoint::AgentEndpoint;
use orchestrator::error::OrchestratorError;
use orchestrator::orchestrator::Orchestrator;
use orchestrator::semantic_router::SemanticRouter;
use rig::embeddings::{Embedding, EmbeddingError, EmbeddingModel};
use serde_json::json;
use tokio::net::TcpListener;

// ---------------------------------------------------------------------------
// Mock server helpers
// ---------------------------------------------------------------------------

/// Configuration for a mock agent HTTP server.
struct MockAgentConfig {
    health_status: HealthStatus,
    response: AgentResponse,
}

/// Starts a mock agent HTTP server on a random port and returns the base URL.
/// The spawned server task is dropped when the tokio runtime shuts down at
/// test exit, which is sufficient for test isolation.
async fn start_mock_agent(config: Arc<MockAgentConfig>) -> String {
    let health_config = Arc::clone(&config);
    let invoke_config = Arc::clone(&config);

    let app = Router::new()
        .route(
            "/health",
            get(move || {
                let cfg = Arc::clone(&health_config);
                async move { Json(json!({ "status": cfg.health_status })) }
            }),
        )
        .route(
            "/invoke",
            post(move |_body: Json<AgentRequest>| {
                let cfg = Arc::clone(&invoke_config);
                async move { Json(cfg.response.clone()) }
            }),
        );

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    format!("http://127.0.0.1:{}", addr.port())
}

/// Creates a mock agent server and returns an `AgentEndpoint` pointing at it.
async fn create_mock_endpoint(
    name: &str,
    description: &str,
    health_status: HealthStatus,
    response: AgentResponse,
) -> AgentEndpoint {
    let config = Arc::new(MockAgentConfig {
        health_status,
        response,
    });
    let url = start_mock_agent(config).await;
    AgentEndpoint::new(name, description, url, reqwest::Client::new())
}

/// Builds a test `SkillManifest` suitable for orchestrator construction.
fn build_test_manifest() -> SkillManifest {
    SkillManifest {
        name: "test-orchestrator".to_string(),
        version: "0.1.0".to_string(),
        description: "Test orchestrator manifest".to_string(),
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

/// Builds a standard successful `AgentResponse` with a given output message.
fn build_success_response(request_id: uuid::Uuid, output_msg: &str) -> AgentResponse {
    AgentResponse {
        id: request_id,
        output: json!({"result": output_msg}),
        confidence: 1.0,
        escalated: false,
        escalate_to: None,
        tool_calls: vec![],
    }
}

/// Builds an `AgentResponse` that triggers escalation to the named target.
fn build_escalation_response(
    request_id: uuid::Uuid,
    escalate_to: &str,
) -> AgentResponse {
    AgentResponse {
        id: request_id,
        output: json!({"result": "needs escalation"}),
        confidence: 0.3,
        escalated: true,
        escalate_to: Some(escalate_to.to_string()),
        tool_calls: vec![],
    }
}

/// Builds an `AgentResponse` with `escalated: true` but no escalation target.
fn build_escalated_no_target_response(request_id: uuid::Uuid, output_msg: &str) -> AgentResponse {
    AgentResponse {
        id: request_id,
        output: json!({"result": output_msg}),
        confidence: 0.5,
        escalated: true,
        escalate_to: None,
        tool_calls: vec![],
    }
}

// ---------------------------------------------------------------------------
// Mock embedding model
// ---------------------------------------------------------------------------

/// A deterministic embedding model for testing semantic routing.
/// Maps known strings to fixed 3D vectors; unknown strings get a zero vector.
struct MockEmbeddingModel;

impl MockEmbeddingModel {
    fn vector_for(text: &str) -> Vec<f64> {
        match text {
            "Handles financial queries" => vec![1.0, 0.0, 0.0],
            "Handles weather forecasts" => vec![0.0, 1.0, 0.0],
            "What are my expenses?" => vec![0.9, 0.1, 0.0],
            "random gibberish" => vec![0.33, 0.33, 0.33],
            _ => vec![0.0, 0.0, 0.0],
        }
    }
}

impl EmbeddingModel for MockEmbeddingModel {
    const MAX_DOCUMENTS: usize = 1;
    type Client = ();

    fn make(_client: &Self::Client, _model: impl Into<String>, _dims: Option<usize>) -> Self {
        MockEmbeddingModel
    }

    fn ndims(&self) -> usize {
        3
    }

    fn embed_texts(
        &self,
        texts: impl IntoIterator<Item = String> + Send,
    ) -> impl std::future::Future<Output = Result<Vec<Embedding>, EmbeddingError>> + Send {
        let results: Vec<Embedding> = texts
            .into_iter()
            .map(|text| {
                let vec = Self::vector_for(&text);
                Embedding {
                    document: text,
                    vec,
                }
            })
            .collect();
        async move { Ok(results) }
    }
}

/// Builds a `SemanticRouter` with finance and weather agents using the mock model.
async fn build_semantic_router(model: &MockEmbeddingModel) -> SemanticRouter {
    let agent_pairs = vec![
        ("finance-agent".to_string(), "Handles financial queries".to_string()),
        ("weather-agent".to_string(), "Handles weather forecasts".to_string()),
    ];
    SemanticRouter::new(model, agent_pairs, 0.7)
        .await
        .expect("failed to build SemanticRouter")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_routes_by_context_target() {
    let request_id = uuid::Uuid::new_v4();

    let agent1 = create_mock_endpoint(
        "agent-1",
        "first agent",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-agent-1"),
    )
    .await;

    let agent2 = create_mock_endpoint(
        "agent-2",
        "second agent",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-agent-2"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent1, agent2], None);

    let request = AgentRequest {
        id: request_id,
        input: "hello".to_string(),
        context: Some(json!({"target_agent": "agent-1"})),
        caller: None,
    };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "from-agent-1"}));
}

#[tokio::test]
async fn dispatch_returns_no_route() {
    let request_id = uuid::Uuid::new_v4();

    let agent = create_mock_endpoint(
        "agent-a",
        "alpha agent",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-a"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent], None);

    // No target_agent in context and input does not match description keywords.
    let request = AgentRequest {
        id: request_id,
        input: "zzzzz".to_string(),
        context: Some(json!({"target_agent": "nonexistent-agent"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::NoRoute { .. } => {}
        other => panic!("expected NoRoute, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_skips_unhealthy_agent() {
    let request_id = uuid::Uuid::new_v4();

    let agent = create_mock_endpoint(
        "sick-agent",
        "unhealthy agent",
        HealthStatus::Unhealthy("down for maintenance".to_string()),
        build_success_response(request_id, "should not reach"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent], None);

    let request = AgentRequest {
        id: request_id,
        input: "test".to_string(),
        context: Some(json!({"target_agent": "sick-agent"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::AgentUnavailable { name, reason } => {
            assert_eq!(name, "sick-agent");
            assert!(reason.contains("down for maintenance"));
        }
        other => panic!("expected AgentUnavailable, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_handles_escalation() {
    let request_id = uuid::Uuid::new_v4();

    // Agent A escalates to agent-b
    let agent_a = create_mock_endpoint(
        "agent-a",
        "first responder",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-b"),
    )
    .await;

    // Agent B returns a successful response
    let agent_b = create_mock_endpoint(
        "agent-b",
        "escalation handler",
        HealthStatus::Healthy,
        build_success_response(request_id, "handled-by-b"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b], None);

    let request = AgentRequest {
        id: request_id,
        input: "need help".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "handled-by-b"}));
}

#[tokio::test]
async fn dispatch_handles_multi_hop_escalation() {
    let request_id = uuid::Uuid::new_v4();

    // Agent A escalates to agent-b
    let agent_a = create_mock_endpoint(
        "agent-a",
        "first responder",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-b"),
    )
    .await;

    // Agent B escalates to agent-c
    let agent_b = create_mock_endpoint(
        "agent-b",
        "second responder",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-c"),
    )
    .await;

    // Agent C returns a successful response
    let agent_c = create_mock_endpoint(
        "agent-c",
        "final handler",
        HealthStatus::Healthy,
        build_success_response(request_id, "handled-by-c"),
    )
    .await;

    let orchestrator =
        Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b, agent_c], None);

    let request = AgentRequest {
        id: request_id,
        input: "need help".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "handled-by-c"}));
    assert!(!response.escalated);
}

#[tokio::test]
async fn dispatch_returns_escalation_failed_on_depth() {
    let request_id = uuid::Uuid::new_v4();

    // Create a chain of 6 agents where each escalates to the next.
    // With MAX_ESCALATION_DEPTH = 5, the chain [agent-0] has length 1,
    // then after escalation to agent-1 chain becomes [agent-0, agent-1] (len 2),
    // and so on. At chain length 5 the next escalation should fail.
    let mut agents = Vec::new();
    for i in 0..6 {
        let next_name = format!("agent-{}", i + 1);
        let agent = create_mock_endpoint(
            &format!("agent-{}", i),
            &format!("chain agent {}", i),
            HealthStatus::Healthy,
            build_escalation_response(request_id, &next_name),
        )
        .await;
        agents.push(agent);
    }

    // Add a final agent that would succeed (but should never be reached)
    let final_agent = create_mock_endpoint(
        "agent-6",
        "chain agent 6",
        HealthStatus::Healthy,
        build_success_response(request_id, "should-not-reach"),
    )
    .await;
    agents.push(final_agent);

    let orchestrator = Orchestrator::new(build_test_manifest(), agents, None);

    let request = AgentRequest {
        id: request_id,
        input: "deep escalation".to_string(),
        context: Some(json!({"target_agent": "agent-0"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::EscalationFailed { chain, reason } => {
            assert!(
                reason.contains("max escalation depth"),
                "reason was: {}",
                reason
            );
            assert!(
                chain.len() <= 6,
                "chain length was: {}",
                chain.len()
            );
        }
        other => panic!("expected EscalationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_returns_escalation_failed_on_cycle() {
    let request_id = uuid::Uuid::new_v4();

    // Agent A escalates to agent-b
    let agent_a = create_mock_endpoint(
        "agent-a",
        "alpha agent",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-b"),
    )
    .await;

    // Agent B escalates back to agent-a, creating a cycle
    let agent_b = create_mock_endpoint(
        "agent-b",
        "beta agent",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-a"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b], None);

    let request = AgentRequest {
        id: request_id,
        input: "trigger cycle".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::EscalationFailed { chain, reason } => {
            assert!(
                reason.contains("cycle detected"),
                "reason was: {}",
                reason
            );
            assert!(
                chain.contains(&"agent-a".to_string()),
                "chain was: {:?}",
                chain
            );
        }
        other => panic!("expected EscalationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_returns_escalation_failed_on_missing_target() {
    let request_id = uuid::Uuid::new_v4();

    // Agent A escalates to "nonexistent-agent" which is NOT registered
    let agent_a = create_mock_endpoint(
        "agent-a",
        "first responder",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "nonexistent-agent"),
    )
    .await;

    // Only register agent-a; "nonexistent-agent" is deliberately absent
    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a], None);

    let request = AgentRequest {
        id: request_id,
        input: "escalate to missing".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::EscalationFailed { chain, reason } => {
            assert!(reason.contains("not found in registry"), "reason was: {}", reason);
            assert!(chain.contains(&"agent-a".to_string()), "chain was: {:?}", chain);
        }
        other => panic!("expected EscalationFailed, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_returns_response_when_escalated_without_target() {
    let request_id = uuid::Uuid::new_v4();

    let agent_a = create_mock_endpoint(
        "agent-a",
        "agent that escalates without target",
        HealthStatus::Healthy,
        build_escalated_no_target_response(request_id, "no-target-escalation"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a], None);

    let request = AgentRequest {
        id: request_id,
        input: "test escalation without target".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "no-target-escalation"}));
    assert!(response.escalated);
    assert!(response.escalate_to.is_none());
}

#[tokio::test]
async fn register_adds_dispatchable_agent() {
    let request_id = uuid::Uuid::new_v4();

    let agent = create_mock_endpoint(
        "late-agent",
        "added later",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-late-agent"),
    )
    .await;

    let mut orchestrator = Orchestrator::new(build_test_manifest(), vec![], None);

    // Before registration, routing should fail
    let request = AgentRequest {
        id: request_id,
        input: "test".to_string(),
        context: Some(json!({"target_agent": "late-agent"})),
        caller: None,
    };
    assert!(orchestrator.dispatch(request).await.is_err());

    // Register and verify dispatch works
    orchestrator.register(agent);

    let request = AgentRequest {
        id: request_id,
        input: "test".to_string(),
        context: Some(json!({"target_agent": "late-agent"})),
        caller: None,
    };
    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "from-late-agent"}));
}

#[tokio::test]
async fn health_returns_healthy_with_one_healthy() {
    let request_id = uuid::Uuid::new_v4();

    let healthy_agent = create_mock_endpoint(
        "good-agent",
        "healthy one",
        HealthStatus::Healthy,
        build_success_response(request_id, "ok"),
    )
    .await;

    let unhealthy_agent = create_mock_endpoint(
        "bad-agent",
        "unhealthy one",
        HealthStatus::Unhealthy("broken".to_string()),
        build_success_response(request_id, "unused"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![healthy_agent, unhealthy_agent], None);

    // The MicroAgent::health impl aggregates: if any agent is healthy, result is Healthy.
    let status = MicroAgent::health(&orchestrator).await;
    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn micro_agent_invoke_delegates_to_dispatch() {
    let request_id = uuid::Uuid::new_v4();

    let agent = create_mock_endpoint(
        "delegate-agent",
        "delegation target",
        HealthStatus::Healthy,
        build_success_response(request_id, "via-trait"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent], None);

    // Use the MicroAgent trait interface
    let trait_ref: &dyn MicroAgent = &orchestrator;

    let request = AgentRequest {
        id: request_id,
        input: "invoke via trait".to_string(),
        context: Some(json!({"target_agent": "delegate-agent"})),
        caller: None,
    };

    let response = trait_ref.invoke(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "via-trait"}));
}

// ---------------------------------------------------------------------------
// Semantic routing tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn dispatch_with_semantic_router_routes_by_intent() {
    let request_id = uuid::Uuid::new_v4();
    let model = MockEmbeddingModel;

    let finance_agent = create_mock_endpoint(
        "finance-agent",
        "Handles financial queries",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-finance"),
    )
    .await;

    let weather_agent = create_mock_endpoint(
        "weather-agent",
        "Handles weather forecasts",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-weather"),
    )
    .await;

    let router = build_semantic_router(&model).await;
    let orchestrator = Orchestrator::new(
        build_test_manifest(),
        vec![finance_agent, weather_agent],
        Some(router),
    );

    // Route via context.intent matching the agent name
    let request = AgentRequest {
        id: request_id,
        input: "anything".to_string(),
        context: Some(json!({"intent": "finance-agent"})),
        caller: None,
    };

    let response = orchestrator.dispatch_with_model(request, &model).await.unwrap();
    assert_eq!(response.output, json!({"result": "from-finance"}));
}

#[tokio::test]
async fn dispatch_with_semantic_router_routes_by_similarity() {
    let request_id = uuid::Uuid::new_v4();
    let model = MockEmbeddingModel;

    let finance_agent = create_mock_endpoint(
        "finance-agent",
        "Handles financial queries",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-finance"),
    )
    .await;

    let weather_agent = create_mock_endpoint(
        "weather-agent",
        "Handles weather forecasts",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-weather"),
    )
    .await;

    let router = build_semantic_router(&model).await;
    let orchestrator = Orchestrator::new(
        build_test_manifest(),
        vec![finance_agent, weather_agent],
        Some(router),
    );

    // No target_agent, no intent -- routes via cosine similarity.
    // "What are my expenses?" [0.9, 0.1, 0.0] is close to finance [1.0, 0.0, 0.0].
    let request = AgentRequest {
        id: request_id,
        input: "What are my expenses?".to_string(),
        context: None,
        caller: None,
    };

    let response = orchestrator.dispatch_with_model(request, &model).await.unwrap();
    assert_eq!(response.output, json!({"result": "from-finance"}));
}

#[tokio::test]
async fn dispatch_with_semantic_router_returns_no_route() {
    let request_id = uuid::Uuid::new_v4();
    let model = MockEmbeddingModel;

    let finance_agent = create_mock_endpoint(
        "finance-agent",
        "Handles financial queries",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-finance"),
    )
    .await;

    let weather_agent = create_mock_endpoint(
        "weather-agent",
        "Handles weather forecasts",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-weather"),
    )
    .await;

    let router = build_semantic_router(&model).await;
    let orchestrator = Orchestrator::new(
        build_test_manifest(),
        vec![finance_agent, weather_agent],
        Some(router),
    );

    // "random gibberish" [0.33, 0.33, 0.33] has low similarity to both agents
    // and should not exceed the 0.7 threshold.
    let request = AgentRequest {
        id: request_id,
        input: "random gibberish".to_string(),
        context: None,
        caller: None,
    };

    let result = orchestrator.dispatch_with_model(request, &model).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::NoRoute { .. } => {}
        other => panic!("expected NoRoute, got: {:?}", other),
    }
}

#[tokio::test]
async fn dispatch_with_semantic_router_prefers_target_agent_over_intent() {
    let request_id = uuid::Uuid::new_v4();
    let model = MockEmbeddingModel;

    let finance_agent = create_mock_endpoint(
        "finance-agent",
        "Handles financial queries",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-finance"),
    )
    .await;

    let weather_agent = create_mock_endpoint(
        "weather-agent",
        "Handles weather forecasts",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-weather"),
    )
    .await;

    let router = build_semantic_router(&model).await;
    let orchestrator = Orchestrator::new(
        build_test_manifest(),
        vec![finance_agent, weather_agent],
        Some(router),
    );

    // target_agent points to weather-agent, but intent says finance-agent.
    // target_agent should take priority (phase 1 of three-phase routing).
    let request = AgentRequest {
        id: request_id,
        input: "anything".to_string(),
        context: Some(json!({
            "target_agent": "weather-agent",
            "intent": "finance-agent"
        })),
        caller: None,
    };

    let response = orchestrator.dispatch_with_model(request, &model).await.unwrap();
    assert_eq!(response.output, json!({"result": "from-weather"}));
}

#[tokio::test]
async fn dispatch_with_model_handles_escalation() {
    let request_id = uuid::Uuid::new_v4();
    let model = MockEmbeddingModel;

    // finance-agent escalates to escalation-handler
    let finance_agent = create_mock_endpoint(
        "finance-agent",
        "Handles financial queries",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "escalation-handler"),
    )
    .await;

    let weather_agent = create_mock_endpoint(
        "weather-agent",
        "Handles weather forecasts",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-weather"),
    )
    .await;

    // escalation-handler is registered in the orchestrator but NOT in the
    // semantic router -- it is only reachable via escalation.
    let escalation_handler = create_mock_endpoint(
        "escalation-handler",
        "Handles escalated requests",
        HealthStatus::Healthy,
        build_success_response(request_id, "handled-by-escalation"),
    )
    .await;

    let router = build_semantic_router(&model).await;
    let orchestrator = Orchestrator::new(
        build_test_manifest(),
        vec![finance_agent, weather_agent, escalation_handler],
        Some(router),
    );

    // No target_agent, no intent -- routes via cosine similarity.
    // "What are my expenses?" [0.9, 0.1, 0.0] is close to finance [1.0, 0.0, 0.0].
    // finance-agent escalates to escalation-handler, which returns success.
    let request = AgentRequest {
        id: request_id,
        input: "What are my expenses?".to_string(),
        context: None,
        caller: None,
    };

    let response = orchestrator
        .dispatch_with_model(request, &model)
        .await
        .unwrap();
    assert_eq!(
        response.output,
        json!({"result": "handled-by-escalation"})
    );
}
