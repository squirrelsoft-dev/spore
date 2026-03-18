use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use agent_sdk::{AgentError, AgentRequest, AgentResponse, HealthStatus, MicroAgent};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::Serialize;

/// A newtype wrapper around [`AgentError`] that implements [`IntoResponse`],
/// allowing HTTP handlers to propagate agent errors via `?` with automatic
/// conversion into appropriate HTTP responses.
pub struct AppError(pub AgentError);

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl fmt::Debug for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl std::error::Error for AppError {}

impl From<AgentError> for AppError {
    fn from(err: AgentError) -> Self {
        AppError(err)
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self.0 {
            AgentError::ToolCallFailed { .. } => StatusCode::BAD_GATEWAY,
            AgentError::ConfidenceTooLow { .. } => StatusCode::OK,
            AgentError::MaxTurnsExceeded { .. } => StatusCode::UNPROCESSABLE_ENTITY,
            AgentError::ActionDisallowed { .. } => StatusCode::FORBIDDEN,
            AgentError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, Json(self.0)).into_response()
    }
}

/// Shared application state holding a trait-object [`MicroAgent`].
pub type AppState = Arc<dyn MicroAgent>;

/// JSON-serializable response returned by the health endpoint.
#[derive(Serialize)]
pub struct HealthResponse {
    pub name: String,
    pub version: String,
    pub status: HealthStatus,
}

/// Handler for `POST /invoke` — forwards the request to the agent and returns
/// its response or an appropriate HTTP error.
pub async fn invoke_handler(
    State(state): State<AppState>,
    Json(request): Json<AgentRequest>,
) -> Result<Json<AgentResponse>, AppError> {
    let response = state.invoke(request).await?;
    Ok(Json(response))
}

/// Handler for `GET /health` — returns the agent's manifest metadata and
/// current health status.
pub async fn health_handler(
    State(state): State<AppState>,
) -> Json<HealthResponse> {
    let manifest = state.manifest();
    let status = state.health().await;
    Json(HealthResponse {
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        status,
    })
}

/// Constructs an axum [`Router`] wired to the invoke and health handlers.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/invoke", post(invoke_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}

/// Binds a TCP listener to `bind_addr` and serves the agent router.
pub async fn start_server(state: AppState, bind_addr: SocketAddr) -> Result<(), std::io::Error> {
    let router = build_router(state);
    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, router).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[test]
    fn app_error_from_agent_error() {
        let agent_err = AgentError::Internal("x".into());
        let app_err = AppError::from(agent_err.clone());
        assert_eq!(app_err.0, agent_err);
    }

    #[test]
    fn app_error_display_delegates() {
        let agent_err = AgentError::Internal("boom".into());
        let app_err = AppError(agent_err.clone());
        assert_eq!(app_err.to_string(), agent_err.to_string());
    }

    #[tokio::test]
    async fn tool_call_failed_returns_502() {
        let expected = AgentError::ToolCallFailed {
            tool: "search".into(),
            reason: "timeout".into(),
        };
        let response = AppError(expected.clone()).into_response();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: AgentError = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, expected);
    }

    #[tokio::test]
    async fn confidence_too_low_returns_200() {
        let expected = AgentError::ConfidenceTooLow {
            confidence: 0.3,
            threshold: 0.8,
        };
        let response = AppError(expected.clone()).into_response();
        assert_eq!(response.status(), StatusCode::OK);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: AgentError = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, expected);
    }

    #[tokio::test]
    async fn max_turns_exceeded_returns_422() {
        let expected = AgentError::MaxTurnsExceeded { turns: 10 };
        let response = AppError(expected.clone()).into_response();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: AgentError = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, expected);
    }

    #[tokio::test]
    async fn action_disallowed_returns_403() {
        let expected = AgentError::ActionDisallowed {
            action: "write".into(),
            allowed: vec!["read".into(), "query".into()],
        };
        let response = AppError(expected.clone()).into_response();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: AgentError = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, expected);
    }

    #[tokio::test]
    async fn internal_returns_500() {
        let expected = AgentError::Internal("something broke".into());
        let response = AppError(expected.clone()).into_response();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let parsed: AgentError = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed, expected);
    }
}
