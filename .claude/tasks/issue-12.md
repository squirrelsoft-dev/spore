# Task Breakdown: Implement HTTP API with axum

> Add an HTTP server to `agent-runtime` using axum with `POST /invoke` and `GET /health` endpoints, wrapping the existing `MicroAgent` trait implementation as `Arc` shared state.

## Group 1 — Dependencies and error mapping

_Tasks in this group can be done in parallel._

- [ ] **Add axum dependency to agent-runtime** `[S]`
      Add `axum = "0.8"` to `crates/agent-runtime/Cargo.toml` under `[dependencies]`. The `tokio` dependency already has `features = ["full"]` which includes `net` and `rt-multi-thread`, so TCP listener support is available. `hyper` and `tower` are already in the dependency tree via `rig-core`, so axum will share them. No other new dependencies are needed — `serde_json` is already present.
      Files: `crates/agent-runtime/Cargo.toml`
      Blocking: "Create HTTP handler module", "Wire router into main.rs"

- [ ] **Create AppError wrapper with HTTP status mapping** `[S]`
      Create an `AppError` newtype wrapping `AgentError` in `crates/agent-runtime/src/http.rs` (or a dedicated `error.rs` if preferred). Implement `axum::response::IntoResponse` for `AppError` to map each `AgentError` variant to an HTTP status code: `ToolCallFailed` to 502 Bad Gateway, `ConfidenceTooLow` to 200 OK (successful but escalated — return a valid response body with the confidence info), `MaxTurnsExceeded` to 422 Unprocessable Entity, `Internal` to 500 Internal Server Error. The response body should be JSON-serialized `AgentError` (it already derives `Serialize`). Implement `From<AgentError> for AppError` so handlers can use `?` directly. Follow the project's manual error impl pattern (no `thiserror`) as seen in `crates/agent-sdk/src/agent_error.rs` and `crates/agent-runtime/src/provider.rs`.
      Files: `crates/agent-runtime/src/http.rs`
      Blocking: "Create HTTP handler module"

## Group 2 — Handler module and router

_Depends on: Group 1._

- [ ] **Create HTTP handler module** `[M]`
      Create `crates/agent-runtime/src/http.rs` with:
      1. An `AppState` type alias or struct wrapping `Arc<dyn MicroAgent>` (the trait already has `Send + Sync` bounds on line 14 of `micro_agent.rs`, and `Arc<dyn MicroAgent>` is `Clone`, satisfying axum's `State` extractor requirements).
      2. `invoke_handler` — async function accepting `State(state)` and `Json(request): Json<AgentRequest>`, calling `state.invoke(request).await`, returning `Result<Json<AgentResponse>, AppError>`. On success return 200 with JSON body. On error, `AppError`'s `IntoResponse` handles status mapping.
      3. `health_handler` — async function accepting `State(state)`, calling `state.health().await`, returning a JSON response. The issue says the endpoint should return skill name, version, and readiness. Define a `HealthResponse` struct with `name: String`, `version: String`, `status: HealthStatus` fields (all derive `Serialize`). Populate from `state.manifest().name`, `state.manifest().version`, and the health check result.
      4. `build_router(state: AppState) -> Router` — constructs the axum `Router` with `/invoke` (POST) and `/health` (GET) routes, attaching the shared state.
      5. `start_server(state: AppState, bind_addr: SocketAddr) -> Result<(), std::io::Error>` — binds `TcpListener` and calls `axum::serve()`. Keep this as a separate function so tests can use `build_router` without binding a port.
      Register the module in `crates/agent-runtime/src/lib.rs` by adding `pub mod http;`.
      Files: `crates/agent-runtime/src/http.rs`, `crates/agent-runtime/src/lib.rs`
      Blocked by: "Add axum dependency to agent-runtime", "Create AppError wrapper with HTTP status mapping"
      Blocking: "Wire router into main.rs"

## Group 3 — Main integration

_Depends on: Group 2._

- [ ] **Wire router into main.rs** `[S]`
      Modify `crates/agent-runtime/src/main.rs` to start the HTTP server after agent construction. Currently line 67 creates `_micro_agent: Arc<dyn MicroAgent>` (unused). Change this to: (1) remove the underscore prefix (use `micro_agent`), (2) wrap it as the `AppState` type, (3) read `config.bind_addr` (already parsed from `BIND_ADDR` env var with default `0.0.0.0:8080` in `config.rs`), (4) call `http::start_server(state, config.bind_addr).await?`. Update the step comments from "[6/6]" to "[6/7]" and add "[7/7] Starting HTTP server". Log the bind address before starting. The function signature already returns `Result<(), Box<dyn std::error::Error>>`, so `std::io::Error` from the listener will propagate.
      Files: `crates/agent-runtime/src/main.rs`
      Blocked by: "Create HTTP handler module"
      Blocking: "Write handler integration tests"

## Group 4 — Tests and verification

_Depends on: Group 3._

- [ ] **Write handler integration tests** `[M]`
      Create `crates/agent-runtime/tests/http_test.rs` with tests that exercise the HTTP layer without a real TCP listener. Use `tower::ServiceExt` (already in the dep tree) to call the router directly via `oneshot()`. Create a `MockAgent` implementing `MicroAgent` (follow the pattern from `crates/agent-sdk/tests/micro_agent_test.rs` lines 9-70, which has `MockAgent` with `should_fail` flag and configurable `health_status`). Tests:
      1. `POST /invoke` with valid `AgentRequest` JSON returns 200 and valid `AgentResponse` JSON.
      2. `POST /invoke` with a mock that returns `AgentError::Internal` returns 500 with JSON error body.
      3. `POST /invoke` with a mock that returns `AgentError::ToolCallFailed` returns 502.
      4. `POST /invoke` with invalid JSON body returns 400 (axum's default deserialization error).
      5. `GET /health` returns 200 with `HealthResponse` JSON containing name, version, and `Healthy` status.
      6. `GET /health` with `Degraded` status returns 200 with correct status in body.
      Add `tower` as a dev-dependency in `crates/agent-runtime/Cargo.toml` (`tower = { version = "0.5", features = ["util"] }`) and `http-body-util = "0.1"` for reading response bodies in tests.
      Files: `crates/agent-runtime/tests/http_test.rs`, `crates/agent-runtime/Cargo.toml`
      Blocked by: "Wire router into main.rs"
      Blocking: "Run verification suite"

- [ ] **Run verification suite** `[S]`
      Run `cargo check -p agent-runtime`, `cargo clippy -p agent-runtime`, and `cargo test -p agent-runtime` to verify the HTTP module compiles, has no warnings, and tests pass. Also run `cargo check` and `cargo test` across the full workspace to ensure no regressions in other crates.
      Files: (none — command-line only)
      Blocked by: "Write handler integration tests"

## Notes for implementers

1. **`Arc<dyn MicroAgent>` as axum state**: Axum's `State` extractor requires `Clone`. `Arc<dyn MicroAgent>` is `Clone` and the trait already has `Send + Sync` bounds (line 14 of `micro_agent.rs`), so this works directly.
2. **`ConfidenceTooLow` ambiguity**: The triage comments note that low confidence might come back as `Ok(response)` with `escalated: true` or as `Err(ConfidenceTooLow)`. The `AppError` mapping should handle the error case by returning 200 with a JSON body that includes the confidence values — it is not a server error, it is a valid response indicating escalation. If `invoke()` returns `Ok` with `escalated: true`, the handler naturally returns 200.
3. **Health endpoint enrichment**: The issue says `/health` should return skill name, version, and readiness. `HealthStatus` is just an enum, so the handler should compose a richer `HealthResponse` struct using data from `manifest()` and `health()`.
4. **Graceful shutdown**: Not in scope for this issue. Issue #14 is noted in the triage as the place for `tokio::signal`-based graceful shutdown.
5. **`tower` test dependency**: `tower` is already in the lockfile (pulled by `rig-core`). Adding it as a dev-dependency with `features = ["util"]` enables `ServiceExt::oneshot()` for testing the router without binding a TCP port.
6. **Functions under 50 lines**: Per project rules, keep `invoke_handler` and `health_handler` short. The `AppError` mapping should be its own `impl` block, not inline in a handler.
