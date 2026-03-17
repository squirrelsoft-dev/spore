# Spec: Create HTTP handler module

> From: .claude/tasks/issue-12.md

## Objective

Create the HTTP handler module (`crates/agent-runtime/src/http.rs`) that exposes the `MicroAgent` trait over HTTP using axum. This module provides `POST /invoke` and `GET /health` endpoints, a router factory for testability, and a server entry point for production use. It bridges the domain-layer `MicroAgent` abstraction to HTTP request/response semantics.

## Current State

### MicroAgent trait (`crates/agent-sdk/src/micro_agent.rs`)

The `MicroAgent` trait is the core interface, marked `Send + Sync` (line 14), making `Arc<dyn MicroAgent>` compatible with axum's `State` extractor (which requires `Clone`). It exposes three methods:

- `fn manifest(&self) -> &SkillManifest` — returns skill metadata (name, version, description, etc.)
- `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>` — processes an agent request
- `async fn health(&self) -> HealthStatus` — returns current health status

### Domain types (`crates/agent-sdk/src/`)

- `AgentRequest` — `{ id: Uuid, input: String, context: Option<Value>, caller: Option<String> }`, derives `Serialize`, `Deserialize`
- `AgentResponse` — `{ id: Uuid, output: Value, confidence: f32, escalated: bool, tool_calls: Vec<ToolCallRecord> }`, derives `Serialize`, `Deserialize`
- `AgentError` — enum with variants `ToolCallFailed`, `ConfidenceTooLow`, `MaxTurnsExceeded`, `Internal`; derives `Serialize`, `Deserialize`
- `HealthStatus` — enum `Healthy | Degraded(String) | Unhealthy(String)`; derives `Serialize`
- `SkillManifest` — includes `name: String` and `version: String` fields; derives `Serialize`

### Runtime crate (`crates/agent-runtime/src/`)

- `lib.rs` currently exports: `pub mod config; pub mod provider; pub mod runtime_agent; pub mod tool_bridge;`
- `main.rs` already creates `Arc<dyn MicroAgent>` on line 67 (currently stored as `_micro_agent`, unused)
- `config.rs` defines `RuntimeConfig` with `bind_addr: SocketAddr` (default `0.0.0.0:8080`)
- `Cargo.toml` has `tokio` with `features = ["full"]` (includes `net` for `TcpListener`), `serde_json`, and `agent-sdk` as dependencies. Axum will be added by the prerequisite task "Add axum dependency to agent-runtime".

### Error handling pattern

The crate uses manual error types (no `thiserror`). See `ProviderError` in `provider.rs` and `AgentError` in `agent-sdk/src/agent_error.rs` for the established pattern: a `Display` impl with `match` arms and a bare `impl std::error::Error`.

## Requirements

- Define `AppState` as a type alias: `type AppState = Arc<dyn MicroAgent>`. This satisfies axum's `Clone` requirement directly since `Arc<T>` is `Clone` and `MicroAgent` is `Send + Sync`.
- Define `AppError` as a newtype wrapping `AgentError`. Implement `From<AgentError> for AppError` so handlers can use `?`. Implement `axum::response::IntoResponse` for `AppError` with the following HTTP status mapping:
  - `ToolCallFailed` -> 502 Bad Gateway
  - `ConfidenceTooLow` -> 200 OK (not a server error; represents a valid escalation response)
  - `MaxTurnsExceeded` -> 422 Unprocessable Entity
  - `Internal` -> 500 Internal Server Error
  - Response body is always the JSON-serialized `AgentError`
- Define `HealthResponse` struct with fields `name: String`, `version: String`, `status: HealthStatus`, deriving `Serialize`.
- Implement `invoke_handler` as an async function with signature `async fn invoke_handler(State(state): State<AppState>, Json(request): Json<AgentRequest>) -> Result<Json<AgentResponse>, AppError>`. Calls `state.invoke(request).await`, maps success to `Json(response)`, maps error via `?` (which converts `AgentError` to `AppError`).
- Implement `health_handler` as an async function with signature `async fn health_handler(State(state): State<AppState>) -> Json<HealthResponse>`. Calls `state.health().await` and `state.manifest()` to compose a `HealthResponse`.
- Implement `build_router(state: AppState) -> Router` that creates an axum `Router` with:
  - `POST /invoke` routed to `invoke_handler`
  - `GET /health` routed to `health_handler`
  - Shared state attached via `.with_state(state)`
- Implement `start_server(state: AppState, bind_addr: SocketAddr) -> Result<(), std::io::Error>` that binds a `tokio::net::TcpListener` to `bind_addr` and calls `axum::serve(listener, router).await`.
- Register the module in `crates/agent-runtime/src/lib.rs` by adding `pub mod http;`.
- All handler functions must be under 50 lines per project rules.
- No `thiserror` or other new dependencies beyond axum (already added by prerequisite task).

## Implementation Details

### Files to create

**`crates/agent-runtime/src/http.rs`**

This file contains all HTTP-layer types and functions:

1. **Imports** — `std::net::SocketAddr`, `std::sync::Arc`, axum extractors (`State`, `Json`), axum routing (`get`, `post`, `Router`), `axum::response::IntoResponse`, `axum::http::StatusCode`, `serde::Serialize`, and SDK types (`AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `MicroAgent`).

2. **`AppState` type alias** — `pub type AppState = Arc<dyn MicroAgent>;`. No wrapper struct needed since `Arc<dyn MicroAgent>` already satisfies `Clone + Send + Sync`.

3. **`AppError` newtype** — `pub struct AppError(AgentError);` with:
   - `impl From<AgentError> for AppError` — trivial wrapping
   - `impl IntoResponse for AppError` — matches on the inner `AgentError` variant to select status code, serializes the `AgentError` to JSON for the response body. Uses `(status_code, Json(self.0)).into_response()`.

4. **`HealthResponse` struct** — `pub struct HealthResponse { pub name: String, pub version: String, pub status: HealthStatus }` with `#[derive(Serialize)]`.

5. **`invoke_handler`** — Extracts `State(state)` and `Json(request)`, calls `state.invoke(request).await?`, returns `Ok(Json(response))`. The `?` operator converts `AgentError` to `AppError` via the `From` impl.

6. **`health_handler`** — Extracts `State(state)`, calls `state.health().await` and `state.manifest()`, constructs and returns `Json(HealthResponse { ... })`.

7. **`build_router`** — Creates `Router::new()` with `.route("/invoke", post(invoke_handler))` and `.route("/health", get(health_handler))`, then `.with_state(state)`.

8. **`start_server`** — Binds `TcpListener::bind(bind_addr).await?`, calls `axum::serve(listener, router).await?`, returns `Ok(())`.

### Files to modify

**`crates/agent-runtime/src/lib.rs`**

Add `pub mod http;` to the module declarations. Place it alphabetically (after `config`, before `provider`).

### Integration points

- `AppState` wraps `Arc<dyn MicroAgent>`, which is already constructed in `main.rs` line 67.
- `start_server` accepts a `SocketAddr` matching the type of `RuntimeConfig::bind_addr` from `config.rs`.
- The downstream task "Wire router into main.rs" will call `http::start_server(micro_agent, config.bind_addr).await?`.
- The downstream task "Write handler integration tests" will use `build_router` with `tower::ServiceExt::oneshot()` to test handlers without binding a port.

## Dependencies

- Blocked by: "Add axum dependency to agent-runtime" (provides the `axum` crate), "Create AppError wrapper with HTTP status mapping" (provides `AppError` and its `IntoResponse` impl — note: the task description places `AppError` in this same file, so in practice both tasks contribute to `http.rs` and should be implemented together or sequentially with the error type first)
- Blocking: "Wire router into main.rs" (needs `build_router` and `start_server`), "Write handler integration tests" (needs `build_router` and `AppState`)

## Risks & Edge Cases

- **`ConfidenceTooLow` as error vs. success**: The `invoke` method may return `Err(AgentError::ConfidenceTooLow { .. })` or `Ok(AgentResponse { escalated: true, .. })` for low-confidence results. The `AppError` mapping handles the error case by returning 200 with the confidence info in the JSON body. The success case with `escalated: true` naturally returns 200 via the normal `Json(response)` path. Both paths produce a 200 — no special handler logic needed beyond the `AppError::IntoResponse` impl.
- **Request body deserialization failures**: If the client sends invalid JSON or JSON that does not match `AgentRequest`, axum returns a 400 Bad Request automatically via its `Json` extractor rejection. No custom handling is needed, but this behavior should be covered in integration tests.
- **Large request bodies**: Axum has a default body size limit (2MB). If agent requests could exceed this, a custom limit layer would be needed. This is out of scope for this task but worth noting.
- **Graceful shutdown**: Not in scope (tracked by issue #14). `start_server` currently runs `axum::serve` without a shutdown signal, meaning it runs until the process is killed.
- **Concurrent access**: `Arc<dyn MicroAgent>` is shared across all handler invocations. The `MicroAgent` trait requires `Send + Sync`, so this is safe. However, implementors (like `RuntimeAgent`) must ensure their `invoke` method is safe for concurrent calls. `RuntimeAgent` holds an `Arc<ToolRegistry>` and a `BuiltAgent` enum, both of which should be concurrency-safe.
- **`http` module name conflicts**: The module is named `http`, which shadows the `http` crate. Within `http.rs`, references to the `http` crate (e.g., `axum::http::StatusCode`) must use the fully qualified path via axum's re-export rather than `use http::StatusCode` directly.

## Verification

- `cargo check -p agent-runtime` compiles without errors after both this task and its prerequisites are complete.
- `cargo clippy -p agent-runtime` produces no warnings.
- `cargo test -p agent-runtime` passes (existing tests remain green; new handler tests are a separate task).
- `cargo build` across the full workspace succeeds with no regressions.
- Manual review confirms:
  - `http.rs` exports `AppState`, `AppError`, `HealthResponse`, `invoke_handler`, `health_handler`, `build_router`, and `start_server`.
  - `lib.rs` includes `pub mod http;`.
  - All functions in `http.rs` are under 50 lines.
  - No `thiserror` or additional dependencies beyond `axum`.
  - Error pattern follows the manual `Display` + `impl std::error::Error` style used in `provider.rs`.
