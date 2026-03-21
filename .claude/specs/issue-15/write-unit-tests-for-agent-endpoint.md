# Spec: Write unit tests for AgentEndpoint

> From: .claude/tasks/issue-15.md

## Objective

Create integration-style unit tests for `AgentEndpoint` that verify its `invoke()` and `health()` methods correctly communicate with downstream agent HTTP servers, properly deserialize successful responses, and map error conditions to the appropriate `OrchestratorError` variants. These tests use a real axum HTTP server (bound to a random port on localhost) rather than mocking `reqwest` directly, ensuring the full HTTP round-trip is exercised.

## Current State

**`crates/agent-runtime/tests/http_test.rs`** establishes the project's axum test pattern. It uses `tower::ServiceExt::oneshot` to send requests directly into an axum `Router` without binding a TCP port. This approach works for testing axum handlers, but is **not suitable for `AgentEndpoint` tests** because `AgentEndpoint` uses `reqwest::Client` to make outbound HTTP calls -- it needs a real listening TCP server.

The test file builds a `MockAgent` implementing `MicroAgent`, wraps it in `Arc<dyn MicroAgent>`, passes it to `agent_runtime::http::build_router()`, and uses helper functions (`build_invoke_request`, `read_body`) for constructing requests and reading responses. The mock supports configurable error modes (`ErrorMode::None`, `ErrorMode::Internal`, `ErrorMode::ToolCallFailed`) and health statuses.

**`crates/agent-runtime/src/http.rs`** defines the server-side HTTP handlers. Key structures:
- `HealthResponse { name: String, version: String, status: HealthStatus }` -- the JSON shape returned by `GET /health`
- `POST /invoke` accepts `Json<AgentRequest>` and returns `Json<AgentResponse>` or an `AppError` (which maps `AgentError` variants to HTTP status codes: `Internal` -> 500, `ToolCallFailed` -> 502, etc.)

**`crates/agent-sdk/src/`** types used in the test:
- `AgentRequest { id: Uuid, input: String, context: Option<Value>, caller: Option<String> }` -- created via `AgentRequest::new(input)`
- `AgentResponse { id: Uuid, output: Value, confidence: f32, escalated: bool, escalate_to: Option<String>, tool_calls: Vec<ToolCallRecord> }`
- `HealthStatus` -- enum with `Healthy`, `Degraded(String)`, `Unhealthy(String)`

**`AgentEndpoint`** (to be created by a predecessor task) is expected to have:
- `new(name: String, description: String, url: String) -> Self` -- constructs with a `reqwest::Client`
- `invoke(&self, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>` -- POST to `{url}/invoke`, map errors to `OrchestratorError::HttpError`
- `health(&self) -> Result<HealthStatus, OrchestratorError>` -- GET `{url}/health`, extract `status` field, map errors to `OrchestratorError::AgentUnavailable`

**`OrchestratorError`** (to be created by a predecessor task) has variants:
- `HttpError { url: String, reason: String }` -- for network/HTTP failures
- `AgentUnavailable { name: String, reason: String }` -- for health check failures

**`crates/orchestrator/Cargo.toml`** currently has no dependencies. A predecessor task will add `axum = "0.8"` as a dev-dependency and `reqwest`, `agent-sdk`, `tokio`, `serde`, `serde_json` as regular dependencies.

## Requirements

1. **Test: `invoke_sends_correct_json_and_returns_agent_response`**
   - Start a mock axum server on `127.0.0.1:0` (OS-assigned port) that implements `POST /invoke` returning a valid `AgentResponse` JSON.
   - Create an `AgentEndpoint` pointing at the server's address.
   - Call `endpoint.invoke(&request)` and verify the returned `AgentResponse` has the correct `id`, `output`, `confidence`, `escalated`, and `tool_calls` fields.
   - Verify the server received the correct `AgentRequest` JSON (matching `id`, `input`, `context`, `caller` fields).

2. **Test: `invoke_maps_http_errors_to_orchestrator_error`**
   - Start a mock axum server that returns HTTP 500 (Internal Server Error) for `POST /invoke`.
   - Call `endpoint.invoke(&request)` and verify the result is `Err(OrchestratorError::HttpError { .. })`.
   - Verify the error's `url` field contains the endpoint URL and the `reason` field contains meaningful error information.

3. **Test: `health_returns_correct_health_status`**
   - Start a mock axum server that returns a valid `HealthResponse` JSON with `status: Healthy` on `GET /health`.
   - Call `endpoint.health()` and verify it returns `Ok(HealthStatus::Healthy)`.
   - Repeat with `Degraded("reason")` to verify non-trivial status deserialization.

4. **Test: `health_maps_connection_failures_to_agent_unavailable`**
   - Create an `AgentEndpoint` pointing at a URL where no server is listening (e.g., `http://127.0.0.1:1` or a port known to be closed).
   - Call `endpoint.health()` and verify the result is `Err(OrchestratorError::AgentUnavailable { .. })`.
   - Verify the error's `name` field matches the endpoint's name and the `reason` field contains connection failure information.

5. All tests must be `#[tokio::test]` async tests.

6. The mock server must use `agent_runtime::http::build_router()` with a `MockAgent` implementing `MicroAgent`, following the same pattern as `crates/agent-runtime/tests/http_test.rs`. This reuses the real HTTP handler logic rather than hand-coding axum routes.

7. The mock server must be spawned on a background tokio task using `tokio::net::TcpListener::bind("127.0.0.1:0")` and `axum::serve()`, so that `reqwest` can make real HTTP connections to it.

## Implementation Details

### File to create

**`crates/orchestrator/tests/agent_endpoint_test.rs`**

### Helper: `MockAgent` struct

Reuse the same pattern from `crates/agent-runtime/tests/http_test.rs`:
- Define an `ErrorMode` enum with variants `None` and `Internal` (only need enough to test success and error paths).
- Define a `MockAgent` struct with fields: `manifest: SkillManifest`, `error_mode: ErrorMode`, `health_status: HealthStatus`.
- Implement `MicroAgent` for `MockAgent`:
  - `manifest()` returns `&self.manifest`
  - `invoke()` returns `Ok(AgentResponse { ... })` in normal mode or `Err(AgentError::Internal(...))` in error mode
  - `health()` returns `self.health_status.clone()`
- Define `make_manifest() -> SkillManifest` helper (same as in `http_test.rs`).

### Helper: `start_mock_server`

```
async fn start_mock_server(error_mode: ErrorMode, health_status: HealthStatus) -> String
```

- Creates a `MockAgent` and wraps it in `Arc<dyn MicroAgent>`.
- Calls `agent_runtime::http::build_router(state)` to get the axum `Router`.
- Binds a `tokio::net::TcpListener` to `127.0.0.1:0`.
- Captures `listener.local_addr()` to determine the assigned port.
- Spawns `axum::serve(listener, router)` on a background tokio task.
- Returns the base URL as `format!("http://127.0.0.1:{}", port)`.

### Test functions

1. **`invoke_sends_correct_json_and_returns_agent_response`**
   - `let url = start_mock_server(ErrorMode::None, HealthStatus::Healthy).await;`
   - `let endpoint = AgentEndpoint::new("test-agent".into(), "desc".into(), url);`
   - `let request = AgentRequest::new("hello".into());`
   - `let response = endpoint.invoke(&request).await.unwrap();`
   - Assert `response.id == request.id`, `response.output == json!({"result": "ok"})`, `response.confidence` is approximately 0.95, `response.escalated == false`, `response.tool_calls.is_empty()`.

2. **`invoke_maps_http_errors_to_orchestrator_error`**
   - `let url = start_mock_server(ErrorMode::Internal, HealthStatus::Healthy).await;`
   - `let endpoint = AgentEndpoint::new("test-agent".into(), "desc".into(), url);`
   - `let request = AgentRequest::new("trigger error".into());`
   - `let result = endpoint.invoke(&request).await;`
   - Assert `result.is_err()`.
   - Match on the error and verify it is `OrchestratorError::HttpError { url, reason }` where `url` contains "/invoke" and `reason` is non-empty.

3. **`health_returns_correct_health_status`**
   - `let url = start_mock_server(ErrorMode::None, HealthStatus::Healthy).await;`
   - `let endpoint = AgentEndpoint::new("test-agent".into(), "desc".into(), url);`
   - `let status = endpoint.health().await.unwrap();`
   - Assert `status == HealthStatus::Healthy`.
   - Optionally repeat with `HealthStatus::Degraded("high latency".into())` in a separate test or as a second assertion block within the same test.

4. **`health_maps_connection_failures_to_agent_unavailable`**
   - `let endpoint = AgentEndpoint::new("test-agent".into(), "desc".into(), "http://127.0.0.1:1".into());` (no server listening)
   - `let result = endpoint.health().await;`
   - Assert `result.is_err()`.
   - Match on the error and verify it is `OrchestratorError::AgentUnavailable { name, reason }` where `name == "test-agent"` and `reason` contains connection-related text.

### Dev-dependencies required in `crates/orchestrator/Cargo.toml`

The predecessor task "Update orchestrator Cargo.toml with dependencies" must include these dev-dependencies:
- `tokio = { version = "1", features = ["macros", "rt"] }` (for `#[tokio::test]`)
- `axum = "0.8"` (for `axum::serve` in the mock server)
- `agent-runtime = { path = "../agent-runtime" }` (for `agent_runtime::http::build_router`)
- `agent-sdk = { path = "../agent-sdk" }` (for `AgentRequest`, `AgentResponse`, `HealthStatus`, `MicroAgent`, etc.)
- `serde_json = "1"` (for `json!()` macro in assertions)

Note: `agent-runtime` as a dev-dependency does NOT create a circular dependency problem. Circular dependencies are only an issue for regular `[dependencies]`. Dev-dependencies form a separate graph and cycles are allowed by Cargo.

### Integration points

- Depends on `orchestrator::agent_endpoint::AgentEndpoint` (the struct under test).
- Depends on `orchestrator::error::OrchestratorError` (to match error variants).
- Depends on `agent_runtime::http::build_router` (to create the mock server router).
- Depends on `agent_sdk::{MicroAgent, AgentRequest, AgentResponse, HealthStatus, AgentError, SkillManifest, ...}` (for mock agent and type assertions).

## Dependencies

- **Blocked by**: "Implement MicroAgent for Orchestrator" (which transitively depends on all Group 1-3 tasks including AgentEndpoint, OrchestratorError, the Cargo.toml updates, and the lib.rs conversion)
- **Blocking**: Nothing (non-blocking; this is a leaf task in Group 5)

## Risks & Edge Cases

- **Port conflicts**: Using `127.0.0.1:0` for OS-assigned ports eliminates port conflict risks. Each test gets its own port.
- **Server shutdown**: The mock server is spawned on a background task and will be dropped when the test completes. Since `axum::serve` runs until the listener is dropped, and the tokio runtime shuts down after the test, cleanup is automatic. No explicit shutdown signal is needed.
- **Race condition on server readiness**: After `tokio::spawn(axum::serve(...))`, the server may not be immediately ready to accept connections. In practice, binding the `TcpListener` before spawning means the port is already listening; `axum::serve` just starts accepting. If flakiness occurs, a small `tokio::time::sleep` can be added, but this is unlikely to be needed.
- **HTTP 500 handling in `invoke()`**: The `AgentEndpoint::invoke()` method must decide how to handle non-2xx HTTP responses. If the server returns 500, `reqwest` will still return `Ok(Response)` (not an error). The implementation must check the status code and map non-2xx responses to `OrchestratorError::HttpError`. The test should verify this behavior explicitly.
- **`reqwest` connection refused**: When connecting to a port with no listener, `reqwest` returns a connection error (not an HTTP error). The `AgentEndpoint::health()` implementation must catch this and convert it to `OrchestratorError::AgentUnavailable`. Port 1 is used in the test because it is a privileged port that no test server will be listening on.
- **Circular dependency concern**: Adding `agent-runtime` as a dev-dependency is safe because Cargo allows cycles in the dev-dependency graph. However, if `agent-runtime` ever adds `orchestrator` as a regular dependency, the dev-dependency from `orchestrator` tests back to `agent-runtime` is still fine. Only regular dependency cycles are forbidden.
- **`HealthResponse` deserialization**: `AgentEndpoint::health()` deserializes a local DTO (not `agent_runtime::http::HealthResponse`) to avoid a regular dependency on `agent-runtime`. The test implicitly validates that the local DTO is compatible with the actual `HealthResponse` format because the mock server uses the real `agent-runtime` handler.

## Verification

1. `cargo test -p orchestrator --test agent_endpoint_test` passes all four tests
2. `cargo clippy -p orchestrator --tests` produces no warnings on the test file
3. Each test exercises a distinct code path in `AgentEndpoint`: successful invoke, error invoke, successful health, connection-failure health
4. The mock server pattern is consistent with `crates/agent-runtime/tests/http_test.rs` (reusing `build_router`, `MockAgent`, `MicroAgent` trait impl)
5. No test depends on external services, network connectivity, or hardcoded ports
6. Tests complete within a reasonable time (< 5 seconds total) since they use localhost connections only
