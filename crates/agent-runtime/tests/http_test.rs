use std::collections::HashMap;
use std::sync::Arc;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent,
    ModelConfig, OutputSchema, SkillManifest,
};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::Router;
use http_body_util::BodyExt;
use serde_json::json;
use tower::ServiceExt;

#[derive(Debug, Clone, PartialEq)]
enum ErrorMode {
    None,
    Internal,
    ToolCallFailed,
}

struct MockAgent {
    manifest: SkillManifest,
    error_mode: ErrorMode,
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
            escalate_to: Some("human".to_string()),
            allowed_actions: vec!["test".to_string()],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::from([("result".to_string(), "string".to_string())]),
        },
    }
}

fn make_mock(error_mode: ErrorMode, health_status: HealthStatus) -> MockAgent {
    MockAgent {
        manifest: make_manifest(),
        error_mode,
        health_status,
    }
}

#[async_trait]
impl MicroAgent for MockAgent {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        match self.error_mode {
            ErrorMode::None => Ok(AgentResponse {
                id: request.id,
                output: json!({"result": "ok"}),
                confidence: 0.95,
                escalated: false,
                tool_calls: vec![],
            }),
            ErrorMode::Internal => Err(AgentError::Internal("mock failure".to_string())),
            ErrorMode::ToolCallFailed => Err(AgentError::ToolCallFailed {
                tool: "bad-tool".to_string(),
                reason: "connection refused".to_string(),
            }),
        }
    }

    async fn health(&self) -> HealthStatus {
        self.health_status.clone()
    }
}

fn build_test_router(error_mode: ErrorMode, health_status: HealthStatus) -> Router {
    let mock = make_mock(error_mode, health_status);
    let state: Arc<dyn MicroAgent> = Arc::new(mock);
    agent_runtime::http::build_router(state)
}

async fn read_body(response: axum::http::Response<Body>) -> String {
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("failed to read response body")
        .to_bytes();
    String::from_utf8(bytes.to_vec()).expect("response body is not valid UTF-8")
}

fn build_invoke_request(body: &str) -> Request<Body> {
    Request::builder()
        .method(Method::POST)
        .uri("/invoke")
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .expect("failed to build request")
}

#[tokio::test]
async fn invoke_valid_request_returns_200() {
    let router = build_test_router(ErrorMode::None, HealthStatus::Healthy);
    let request = AgentRequest::new("hello".to_string());
    let request_id = request.id;
    let body = serde_json::to_string(&request).unwrap();

    let response = router
        .oneshot(build_invoke_request(&body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = read_body(response).await;
    let parsed: AgentResponse = serde_json::from_str(&body_str).unwrap();
    assert_eq!(parsed.id, request_id);
    assert_eq!(parsed.output, json!({"result": "ok"}));
    assert!((parsed.confidence - 0.95).abs() < f32::EPSILON);
    assert!(!parsed.escalated);
    assert!(parsed.tool_calls.is_empty());
}

#[tokio::test]
async fn invoke_internal_error_returns_500() {
    let router = build_test_router(ErrorMode::Internal, HealthStatus::Healthy);
    let request = AgentRequest::new("trigger error".to_string());
    let body = serde_json::to_string(&request).unwrap();

    let response = router
        .oneshot(build_invoke_request(&body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body_str = read_body(response).await;
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    let inner = &json["Internal"];
    assert_eq!(inner.as_str().unwrap(), "mock failure");
}

#[tokio::test]
async fn invoke_tool_call_failed_returns_502() {
    let router = build_test_router(ErrorMode::ToolCallFailed, HealthStatus::Healthy);
    let request = AgentRequest::new("trigger tool error".to_string());
    let body = serde_json::to_string(&request).unwrap();

    let response = router
        .oneshot(build_invoke_request(&body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
    let body_str = read_body(response).await;
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    let inner = &json["ToolCallFailed"];
    assert_eq!(inner["tool"], "bad-tool");
    assert_eq!(inner["reason"], "connection refused");
}

#[tokio::test]
async fn invoke_invalid_json_returns_422() {
    let router = build_test_router(ErrorMode::None, HealthStatus::Healthy);

    let response = router
        .oneshot(build_invoke_request(r#"{"bad": true}"#))
        .await
        .unwrap();

    let status = response.status();
    let body_str = read_body(response).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(body_str.contains("Failed to deserialize the JSON body"));
}

#[tokio::test]
async fn health_returns_200_with_healthy_status() {
    let router = build_test_router(ErrorMode::None, HealthStatus::Healthy);
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = read_body(response).await;
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(json["name"], "test-agent");
    assert_eq!(json["version"], "1.0.0");
    assert_eq!(json["status"], "Healthy");
}

#[tokio::test]
async fn health_returns_200_with_degraded_status() {
    let router = build_test_router(
        ErrorMode::None,
        HealthStatus::Degraded("high latency".to_string()),
    );
    let request = Request::builder()
        .method(Method::GET)
        .uri("/health")
        .body(Body::empty())
        .unwrap();

    let response = router.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body_str = read_body(response).await;
    let json: serde_json::Value = serde_json::from_str(&body_str).unwrap();
    assert_eq!(json["name"], "test-agent");
    assert_eq!(json["version"], "1.0.0");
    assert_eq!(json["status"]["Degraded"], "high latency");
}
