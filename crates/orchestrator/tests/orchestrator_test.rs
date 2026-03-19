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
    AgentEndpoint::new(name, description, url)
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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent1, agent2]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), agents);

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
async fn register_adds_dispatchable_agent() {
    let request_id = uuid::Uuid::new_v4();

    let agent = create_mock_endpoint(
        "late-agent",
        "added later",
        HealthStatus::Healthy,
        build_success_response(request_id, "from-late-agent"),
    )
    .await;

    let mut orchestrator = Orchestrator::new(build_test_manifest(), vec![]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![healthy_agent, unhealthy_agent]);

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

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent]);

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
