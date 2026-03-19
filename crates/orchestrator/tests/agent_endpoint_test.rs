use agent_sdk::{AgentRequest, AgentResponse, HealthStatus};
use axum::extract::Json;
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::Router;
use orchestrator::agent_endpoint::AgentEndpoint;
use orchestrator::error::OrchestratorError;
use serde_json::json;
use tokio::net::TcpListener;

/// Starts a mock agent server on an ephemeral port and returns its base URL.
///
/// The server responds to `POST /invoke` with a valid `AgentResponse` and
/// `GET /health` with a JSON object containing the given `HealthStatus`.
/// The spawned server task is dropped when the tokio runtime shuts down at
/// test exit, which is sufficient for test isolation.
async fn start_mock_server(health_status: HealthStatus) -> String {
    let router = build_mock_router(health_status);
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock server failed");
    });
    format!("http://{}", addr)
}

/// Starts a mock server that always returns a 500 status on `POST /invoke`.
/// The spawned server task is dropped when the tokio runtime shuts down at
/// test exit, which is sufficient for test isolation.
async fn start_error_mock_server() -> String {
    let router = build_error_mock_router();
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router)
            .await
            .expect("mock server failed");
    });
    format!("http://{}", addr)
}

fn build_mock_router(health_status: HealthStatus) -> Router {
    Router::new()
        .route("/invoke", post(mock_invoke_handler))
        .route("/health", get(mock_health_handler))
        .with_state(health_status)
}

fn build_error_mock_router() -> Router {
    Router::new()
        .route("/invoke", post(mock_invoke_error_handler))
}

async fn mock_invoke_handler(
    Json(request): Json<AgentRequest>,
) -> Json<AgentResponse> {
    Json(AgentResponse {
        id: request.id,
        output: json!({"result": "ok"}),
        confidence: 0.95,
        escalated: false,
        escalate_to: None,
        tool_calls: vec![],
    })
}

async fn mock_invoke_error_handler() -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

async fn mock_health_handler(
    axum::extract::State(status): axum::extract::State<HealthStatus>,
) -> Json<serde_json::Value> {
    Json(json!({
        "name": "test",
        "version": "1.0",
        "status": status,
    }))
}

#[tokio::test]
async fn invoke_success() {
    let url = start_mock_server(HealthStatus::Healthy).await;
    let endpoint = AgentEndpoint::new("test-agent", "A test agent", &url);
    let request = AgentRequest::new("hello".to_string());
    let request_id = request.id;

    let result = endpoint.invoke(&request).await;

    let response = result.expect("invoke should succeed");
    assert_eq!(response.id, request_id);
    assert_eq!(response.output, json!({"result": "ok"}));
    assert!((response.confidence - 0.95).abs() < f32::EPSILON);
    assert!(!response.escalated);
    assert!(response.escalate_to.is_none());
    assert!(response.tool_calls.is_empty());
}

#[tokio::test]
async fn invoke_http_error() {
    let url = start_error_mock_server().await;
    let endpoint = AgentEndpoint::new("test-agent", "A test agent", &url);
    let request = AgentRequest::new("trigger error".to_string());

    let result = endpoint.invoke(&request).await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        OrchestratorError::HttpError { .. }
    ));
}

#[tokio::test]
async fn health_success() {
    let url = start_mock_server(HealthStatus::Healthy).await;
    let endpoint = AgentEndpoint::new("test-agent", "A test agent", &url);

    let result = endpoint.health().await;

    let status = result.expect("health should succeed");
    assert_eq!(status, HealthStatus::Healthy);
}

#[tokio::test]
async fn health_connection_refused() {
    // Bind a port and immediately drop the listener so nothing is serving
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind ephemeral port");
    let addr = listener.local_addr().unwrap();
    drop(listener);

    let url = format!("http://{}", addr);
    let endpoint = AgentEndpoint::new("test-agent", "A test agent", &url);

    let result = endpoint.health().await;

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        OrchestratorError::AgentUnavailable { .. }
    ));
}
