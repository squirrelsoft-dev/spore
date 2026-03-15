use std::collections::HashMap;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent,
    ModelConfig, OutputSchema, SkillManifest,
};
use serde_json::json;

struct MockAgent {
    manifest: SkillManifest,
    should_fail: bool,
    health_status: HealthStatus,
}

fn make_manifest() -> SkillManifest {
    SkillManifest {
        name: "test-agent".to_string(),
        description: "A mock agent for testing".to_string(),
        version: "1.0.0".to_string(),
        model: ModelConfig {
            provider: "test-provider".to_string(),
            name: "test-model".to_string(),
            temperature: 0.7,
        },
        preamble: "You are a test agent.".to_string(),
        tools: vec!["test_tool".to_string()],
        constraints: Constraints {
            max_turns: 10,
            confidence_threshold: 0.85,
            escalate_to: "human".to_string(),
            allowed_actions: vec!["test".to_string()],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::from([("result".to_string(), "string".to_string())]),
        },
    }
}

fn make_mock(should_fail: bool, health: HealthStatus) -> MockAgent {
    MockAgent {
        manifest: make_manifest(),
        should_fail,
        health_status: health,
    }
}

#[async_trait]
impl MicroAgent for MockAgent {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        if self.should_fail {
            return Err(AgentError::Internal("mock failure".to_string()));
        }
        Ok(AgentResponse {
            id: request.id,
            output: json!({"result": "ok"}),
            confidence: 0.95,
            escalated: false,
            tool_calls: vec![],
        })
    }

    async fn health(&self) -> HealthStatus {
        self.health_status.clone()
    }
}

#[tokio::test]
async fn mock_agent_implements_trait() {
    let agent = make_mock(false, HealthStatus::Healthy);
    let manifest = agent.manifest();
    assert_eq!(manifest.name, "test-agent");

    let request = AgentRequest::new("test input".to_string());
    let response = agent.invoke(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "ok"}));

    let health = agent.health().await;
    assert_eq!(health, HealthStatus::Healthy);
}

#[tokio::test]
async fn trait_object_is_dyn_compatible() {
    let mock = make_mock(false, HealthStatus::Healthy);
    let agent: Box<dyn MicroAgent> = Box::new(mock);

    let manifest = agent.manifest();
    assert_eq!(manifest.name, "test-agent");

    let request = AgentRequest::new("boxed test".to_string());
    let req_id = request.id;
    let response = agent.invoke(request).await.unwrap();
    assert_eq!(response.id, req_id);

    let health = agent.health().await;
    assert_eq!(health, HealthStatus::Healthy);
}

#[tokio::test]
async fn invoke_returns_ok() {
    let agent = make_mock(false, HealthStatus::Healthy);
    let request = AgentRequest::new("hello".to_string());
    let req_id = request.id;
    let result = agent.invoke(request).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.id, req_id);
    assert_eq!(response.output, json!({"result": "ok"}));
    assert!((response.confidence - 0.95).abs() < f32::EPSILON);
    assert!(!response.escalated);
    assert!(response.tool_calls.is_empty());
}

#[tokio::test]
async fn invoke_returns_err() {
    let agent = make_mock(true, HealthStatus::Healthy);
    let request = AgentRequest::new("will fail".to_string());
    let result = agent.invoke(request).await;

    assert!(result.is_err());
    assert_eq!(
        result.unwrap_err(),
        AgentError::Internal("mock failure".to_string())
    );
}

#[tokio::test]
async fn health_status_healthy() {
    let agent = make_mock(false, HealthStatus::Healthy);
    let health = agent.health().await;
    assert_eq!(health, HealthStatus::Healthy);
}

#[tokio::test]
async fn health_status_degraded() {
    let agent = make_mock(false, HealthStatus::Degraded("high latency".to_string()));
    let health = agent.health().await;
    assert_eq!(health, HealthStatus::Degraded("high latency".to_string()));
}

#[tokio::test]
async fn health_status_unhealthy() {
    let agent = make_mock(false, HealthStatus::Unhealthy("database down".to_string()));
    let health = agent.health().await;
    assert_eq!(health, HealthStatus::Unhealthy("database down".to_string()));
}
