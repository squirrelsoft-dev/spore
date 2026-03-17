# Spec: Write handler integration tests

> From: .claude/tasks/issue-12.md

## Objective

Create integration tests that exercise the HTTP layer (`POST /invoke` and `GET /health`) of the `agent-runtime` crate without binding a real TCP port. These tests validate that the axum router correctly dispatches requests, serializes/deserializes JSON payloads, maps `AgentError` variants to the correct HTTP status codes, and returns well-formed `HealthResponse` bodies. By testing at the router level with `tower::ServiceExt::oneshot()`, we get fast, deterministic coverage of the HTTP contract without network flakiness.

## Current State

### Domain types (in `crates/agent-sdk/src/`)

- **`AgentRequest`** (`agent_request.rs`): `{ id: Uuid, input: String, context: Option<Value>, caller: Option<String> }`. Derives `Serialize`, `Deserialize`. Has `AgentRequest::new(input)` constructor that generates a random `id`.
- **`AgentResponse`** (`agent_response.rs`): `{ id: Uuid, output: Value, confidence: f32, escalated: bool, tool_calls: Vec<ToolCallRecord> }`. Derives `Serialize`, `Deserialize`. Has `AgentResponse::success(id, output)` convenience constructor.
- **`AgentError`** (`agent_error.rs`): enum with `ToolCallFailed { tool, reason }`, `ConfidenceTooLow { confidence, threshold }`, `MaxTurnsExceeded { turns }`, `Internal(String)`. Derives `Serialize`, `Deserialize`, `PartialEq`.
- **`HealthStatus`** (`health_status.rs`): enum `Healthy`, `Degraded(String)`, `Unhealthy(String)`. Derives `Serialize`, `Deserialize`, `PartialEq`, `Clone`.
- **`SkillManifest`** (`skill_manifest.rs`): `{ name, version, description, model: ModelConfig, preamble, tools, constraints: Constraints, output: OutputSchema }`.
- **`MicroAgent` trait** (`micro_agent.rs`): `Send + Sync`, uses `#[async_trait]`. Methods: `fn manifest() -> &SkillManifest`, `async fn invoke(AgentRequest) -> Result<AgentResponse, AgentError>`, `async fn health() -> HealthStatus`.

### Existing mock pattern (in `crates/agent-sdk/tests/micro_agent_test.rs`, lines 9-70)

A `MockAgent` struct with fields `manifest: SkillManifest`, `should_fail: bool`, `health_status: HealthStatus`. A `make_manifest()` helper builds a full `SkillManifest` with test values. A `make_mock(should_fail, health)` factory creates instances. The `#[async_trait] impl MicroAgent` returns `AgentError::Internal("mock failure")` when `should_fail` is true, and a valid `AgentResponse` otherwise. The `health()` method returns the stored `health_status`.

### HTTP module (to be created by predecessor tasks)

Per the task breakdown, `crates/agent-runtime/src/http.rs` will contain:
- `AppState` wrapping `Arc<dyn MicroAgent>`.
- `invoke_handler` accepting `State(state)` + `Json<AgentRequest>`, returning `Result<Json<AgentResponse>, AppError>`.
- `health_handler` accepting `State(state)`, returning JSON with a `HealthResponse` struct containing `name: String`, `version: String`, `status: HealthStatus`.
- `build_router(state: AppState) -> Router` wiring `/invoke` (POST) and `/health` (GET).
- `AppError` newtype wrapping `AgentError` with `IntoResponse` mapping: `Internal` -> 500, `ToolCallFailed` -> 502, `ConfidenceTooLow` -> 200 (valid escalation response), `MaxTurnsExceeded` -> 422.

### Current dev-dependencies (`crates/agent-runtime/Cargo.toml`)

None currently declared. The `[dev-dependencies]` section does not exist yet.

### Transitive dependency tree

`tower` 0.5 and `tower-service` 0.3 are already in `Cargo.lock` via `rig-core`. Adding `tower` as an explicit dev-dependency with `features = ["util"]` enables `ServiceExt::oneshot()`. `http-body-util` 0.1 is needed for `BodyExt::collect()` to read response bodies in tests.

## Requirements

1. **`POST /invoke` success (200)**: Send a valid `AgentRequest` JSON body to the router. Assert HTTP 200 status. Deserialize the response body as `AgentResponse` and verify the `id` matches the request, `output` contains the expected mock value, `confidence` is correct, `escalated` is false, and `tool_calls` is empty.

2. **`POST /invoke` with `AgentError::Internal` (500)**: Use a `MockAgent` configured with `should_fail: true` (which returns `AgentError::Internal`). Assert HTTP 500 status. Verify the response body is a JSON-serialized `AgentError::Internal` variant.

3. **`POST /invoke` with `AgentError::ToolCallFailed` (502)**: Extend the mock to support returning `ToolCallFailed` errors (the existing `should_fail` flag only produces `Internal`; the test mock needs a configurable error variant or a second flag). Assert HTTP 502 status. Verify the JSON error body contains the `ToolCallFailed` variant with tool name and reason.

4. **`POST /invoke` with invalid JSON (400)**: Send a request with a body that is not valid `AgentRequest` JSON (e.g., `{"bad": true}`). Assert HTTP 400 or 422 status (axum's default `Json` rejection). The response body should contain a deserialization error message.

5. **`GET /health` with `Healthy` status (200)**: Send a GET request to `/health` using a mock with `HealthStatus::Healthy`. Assert HTTP 200. Deserialize response as `HealthResponse` and verify `name` matches `manifest.name`, `version` matches `manifest.version`, and `status` is `Healthy`.

6. **`GET /health` with `Degraded` status (200)**: Send a GET request to `/health` using a mock with `HealthStatus::Degraded("high latency")`. Assert HTTP 200. Verify `status` in the response body is `Degraded` with the expected reason string.

## Implementation Details

### Files to create

**`crates/agent-runtime/tests/http_test.rs`**

- **Imports**: `agent_sdk` types (`async_trait`, `AgentError`, `AgentRequest`, `AgentResponse`, `HealthStatus`, `MicroAgent`, `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`), `agent_runtime::http::{build_router, AppState, HealthResponse}`, `axum::body::Body`, `axum::http::{Request, StatusCode, Method}`, `http_body_util::BodyExt`, `tower::ServiceExt`, `serde_json`, `std::sync::Arc`, `std::collections::HashMap`.

- **`MockAgent` struct**: Fields: `manifest: SkillManifest`, `error_mode: ErrorMode`, `health_status: HealthStatus`.

- **`ErrorMode` enum**: `None`, `Internal`, `ToolCallFailed`. This extends the original `should_fail: bool` pattern to support multiple error variants needed by tests 2 and 3.

- **`make_manifest()` helper**: Returns a `SkillManifest` with `name: "test-agent"`, `version: "1.0.0"`, and other plausible test values (replicating the pattern from `crates/agent-sdk/tests/micro_agent_test.rs` lines 15-38).

- **`make_mock(error_mode, health_status)` factory**: Creates a `MockAgent` with the given error mode and health status.

- **`#[async_trait] impl MicroAgent for MockAgent`**:
  - `manifest()`: returns `&self.manifest`.
  - `invoke()`: matches on `self.error_mode`:
    - `ErrorMode::None` -> `Ok(AgentResponse { id: request.id, output: json!({"result": "ok"}), confidence: 0.95, escalated: false, tool_calls: vec![] })`.
    - `ErrorMode::Internal` -> `Err(AgentError::Internal("mock failure".to_string()))`.
    - `ErrorMode::ToolCallFailed` -> `Err(AgentError::ToolCallFailed { tool: "bad-tool".to_string(), reason: "connection refused".to_string() })`.
  - `health()`: returns `self.health_status.clone()`.

- **`build_test_router(error_mode, health_status)` helper**: Creates a `MockAgent`, wraps in `Arc<dyn MicroAgent>` as `AppState`, calls `build_router(state)` and returns the `Router`.

- **`read_body(response)` async helper**: Takes an `axum::http::Response<Body>`, uses `BodyExt::collect().await` and `to_bytes()` to extract the full body as `Bytes`, then converts to `String`. Keeps test functions under 50 lines.

- **Test functions** (all `#[tokio::test] async`):

  1. `async fn invoke_valid_request_returns_200()`: Build router with `ErrorMode::None`. Create `AgentRequest::new("hello")`, serialize to JSON. Build `Request::builder().method(Method::POST).uri("/invoke").header("content-type", "application/json").body(Body::from(json_bytes))`. Call `router.oneshot(request).await`. Assert status 200. Deserialize body as `AgentResponse`. Assert `id` matches, `output` is `{"result": "ok"}`, confidence ~0.95, not escalated, empty tool_calls.

  2. `async fn invoke_internal_error_returns_500()`: Build router with `ErrorMode::Internal`. Send valid `AgentRequest` to `POST /invoke`. Assert status 500. Deserialize body and verify it contains the `Internal` error variant.

  3. `async fn invoke_tool_call_failed_returns_502()`: Build router with `ErrorMode::ToolCallFailed`. Send valid `AgentRequest` to `POST /invoke`. Assert status 502. Verify JSON body contains `ToolCallFailed` with `tool: "bad-tool"`.

  4. `async fn invoke_invalid_json_returns_400()`: Build router with `ErrorMode::None`. Send `POST /invoke` with body `{"bad": true}`. Assert status is 400 (or 422, depending on axum version -- the test should accept either as both indicate a client error from deserialization rejection).

  5. `async fn health_returns_200_with_healthy_status()`: Build router with `HealthStatus::Healthy`. Send `GET /health`. Assert status 200. Deserialize as `HealthResponse`. Assert `name == "test-agent"`, `version == "1.0.0"`, `status == HealthStatus::Healthy`.

  6. `async fn health_returns_200_with_degraded_status()`: Build router with `HealthStatus::Degraded("high latency".to_string())`. Send `GET /health`. Assert status 200. Deserialize as `HealthResponse`. Assert `status == HealthStatus::Degraded("high latency".to_string())`.

### Files to modify

**`crates/agent-runtime/Cargo.toml`**

Add a `[dev-dependencies]` section with:
```toml
[dev-dependencies]
tower = { version = "0.5", features = ["util"] }
http-body-util = "0.1"
```

The `tokio` dependency (with `features = ["full"]`) is already in `[dependencies]` and available for `#[tokio::test]`. `serde_json` is also already in `[dependencies]`. `agent-sdk` is in `[dependencies]` and provides all domain types.

### Key integration points

- The tests import `agent_runtime::http::{build_router, AppState, HealthResponse}` -- these are the public items from the HTTP handler module created by the "Create HTTP handler module" task.
- The `MockAgent` pattern is lifted from `crates/agent-sdk/tests/micro_agent_test.rs` and extended with `ErrorMode` to cover the additional error variant (`ToolCallFailed`) required by test case 3.
- `tower::ServiceExt::oneshot()` is used instead of a TCP listener, making tests fast and parallel-safe with no port conflicts.

## Dependencies

- Blocked by: "Wire router into main.rs" (which depends on "Create HTTP handler module", which produces `http.rs` with `build_router`, `AppState`, `HealthResponse`, and `AppError`)
- Blocking: "Run verification suite"

## Risks & Edge Cases

- **`HealthResponse` type visibility**: The tests import `HealthResponse` from `agent_runtime::http`. If the handler module makes `HealthResponse` private or names it differently, the test imports will break. Mitigation: the task spec for "Create HTTP handler module" explicitly calls for a `HealthResponse` struct with `name`, `version`, `status` fields. The implementer should ensure it is `pub`.
- **Axum JSON rejection status code**: Axum 0.8 returns 422 for `Json` deserialization failures by default (changed from 400 in earlier versions). The test for invalid JSON (test 4) should check for 422 specifically if targeting axum 0.8, or accept either 400/422. The spec recommends asserting `status.is_client_error()` as a safe guard, then asserting the specific code if the axum version is known.
- **`AppState` type**: If `AppState` is a type alias (`type AppState = Arc<dyn MicroAgent>`) rather than a newtype struct, the test code simply passes an `Arc<dyn MicroAgent>` directly to `build_router()`. If it is a newtype, the test must wrap accordingly. The task description says "type alias or struct" -- the test should follow whatever the handler module defines.
- **Body reading in tests**: `axum::body::Body` does not implement `Into<Bytes>` directly. The `http-body-util` crate's `BodyExt::collect()` is needed. This is why `http-body-util` is added as a dev-dependency.
- **Mock does not cover `ConfidenceTooLow` or `MaxTurnsExceeded`**: The task description specifies only 6 tests. Additional error variants can be tested in follow-up work if needed. The `ErrorMode` enum is extensible.
- **`serde_json` for `AgentRequest` serialization**: Tests need to serialize `AgentRequest` into a JSON body string. Since `AgentRequest` derives `Serialize`, `serde_json::to_vec(&request)` works directly.

## Verification

1. `cargo check -p agent-runtime --tests` compiles the test file without errors (requires predecessor tasks to be complete).
2. `cargo test -p agent-runtime --test http_test` runs all 6 tests and they pass.
3. `cargo clippy -p agent-runtime --tests` reports no warnings in the test file.
4. Each test function is under 50 lines (per project rules), with shared logic extracted into `build_test_router()` and `read_body()` helpers.
5. No new runtime dependencies are added; `tower` and `http-body-util` are dev-only.
6. The `MockAgent` in the test file is self-contained and does not depend on any real provider, network, or file system resources.
