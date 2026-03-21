# Spec: Write unit tests for Orchestrator dispatch and routing

> From: .claude/tasks/issue-15.md

## Objective

Create a comprehensive integration test suite (`crates/orchestrator/tests/orchestrator_test.rs`) that validates the `Orchestrator` struct's dispatch, routing, escalation, health aggregation, and `MicroAgent` trait implementation. These tests use mock HTTP servers (via `axum` + `tokio::net::TcpListener`) to simulate real downstream agents, verifying that the orchestrator correctly routes requests, handles failures, enforces escalation chain limits, and exposes itself as a `MicroAgent`.

## Current State

- **Orchestrator crate**: Currently a stub with only `src/main.rs` printing "Hello, world!". The task breakdown (issue-15) plans to convert it to a library crate with modules `agent_endpoint`, `error`, `orchestrator`, and `config`.
- **Orchestrator types (planned, not yet implemented)**:
  - `Orchestrator` struct with `registry: HashMap<String, AgentEndpoint>` and `manifest: SkillManifest`
  - Methods: `new()`, `register()`, `route()`, `dispatch()`, `from_config()`
  - `MicroAgent` trait implementation: `manifest()`, `invoke()` (delegates to `dispatch()`), `health()` (aggregates downstream health)
  - `AgentEndpoint` struct: wraps `reqwest::Client`, calls downstream agents via HTTP (`POST /invoke`, `GET /health`)
  - `OrchestratorError` enum: `NoRoute`, `AgentUnavailable`, `EscalationFailed`, `HttpError`
- **Mock test pattern** (`crates/agent-runtime/tests/constraint_enforcer_test.rs`): Uses a `MockAgent` struct implementing `MicroAgent` with configurable `response_confidence`, `error_mode`, and `health_status`. Helper function `make_manifest_with_threshold()` builds a `SkillManifest` with specific constraint values. Each test constructs the mock, wraps it in the system under test, and asserts on the output.
- **HTTP test pattern** (`crates/agent-runtime/tests/http_test.rs`): Uses `axum::Router` with `tower::ServiceExt::oneshot()` for in-process HTTP testing. Also uses `agent_runtime::http::build_router()` to create a real HTTP router from a `MockAgent`. Both patterns are available for this test file.
- **Key SDK types**: `AgentRequest` has `id: Uuid`, `input: String`, `context: Option<Value>`, `caller: Option<String>`. `AgentResponse` has `id`, `output`, `confidence`, `escalated: bool`, `escalate_to: Option<String>`, `tool_calls`. `HealthStatus` is an enum: `Healthy`, `Degraded(String)`, `Unhealthy(String)`.
- **Routing mechanism (planned)**: The orchestrator looks up agent by name from `request.context` (specifically a `"target_agent": "name"` field in the context JSON), falling back to substring matching of `request.input` against endpoint descriptions.

## Requirements

### Test Infrastructure
- Stand up mock HTTP servers using `axum` routers bound to `TcpListener` on `127.0.0.1:0` (OS-assigned port), each simulating a downstream agent with configurable behavior (success, escalation, error, health status).
- Each mock server must implement `POST /invoke` and `GET /health` endpoints matching the `agent-runtime` HTTP API contract (see `crates/agent-runtime/src/http.rs`).
- Mock servers must be configurable per-test to return specific `AgentResponse` values (including `escalated: true` and `escalate_to: Some(...)`) and specific `HealthStatus` values.
- Use `Arc<Mutex<...>>` or `Arc<AtomicBool>` for mock servers that need to track whether they were called (to verify routing correctness).

### Test Cases

1. **`dispatch_routes_to_correct_agent_based_on_context_target`**
   - Register two agents ("agent-a" and "agent-b") pointing to two different mock HTTP servers.
   - Create an `AgentRequest` with `context: Some(json!({"target_agent": "agent-b"}))`.
   - Call `dispatch()` and assert the response came from agent-b's mock (use a distinguishing output value like `{"source": "agent-b"}`).
   - Verify agent-a's mock was NOT called.

2. **`dispatch_returns_no_route_when_no_agent_matches`**
   - Register one agent named "agent-a".
   - Create an `AgentRequest` with `context: Some(json!({"target_agent": "nonexistent-agent"}))`.
   - Call `dispatch()` and assert it returns `Err(OrchestratorError::NoRoute { .. })`.
   - Verify the error's `input` field contains the request input.

3. **`dispatch_skips_unhealthy_agents`**
   - Register two agents: "agent-a" (mock returns `Unhealthy`) and "agent-b" (mock returns `Healthy`).
   - Create a request that could route to either (e.g., via description-based fallback routing, or test the specific behavior when the targeted agent is unhealthy).
   - Call `dispatch()` targeting "agent-a" and assert it returns `Err(OrchestratorError::AgentUnavailable { .. })` with the agent name and health reason.
   - Alternatively, if the orchestrator falls through to the next matching agent, assert the response came from agent-b.

4. **`dispatch_handles_escalation`**
   - Register two agents: "agent-primary" (mock returns `AgentResponse` with `escalated: true, escalate_to: Some("agent-fallback")`) and "agent-fallback" (mock returns a normal successful `AgentResponse`).
   - Create a request targeting "agent-primary".
   - Call `dispatch()` and assert the final response is from "agent-fallback" (the escalation target).
   - Verify both mock servers were called (primary first, then fallback).

5. **`dispatch_returns_escalation_failed_when_chain_exhausted`**
   - Register agents that form a circular or terminal escalation chain: "agent-a" escalates to "agent-b", "agent-b" escalates to "agent-c", "agent-c" escalates to a nonexistent agent (or back to "agent-a" to test cycle detection).
   - Call `dispatch()` targeting "agent-a".
   - Assert it returns `Err(OrchestratorError::EscalationFailed { chain, reason })`.
   - Assert the `chain` vector contains the names of all agents in the escalation path.
   - Also test that the recursion/depth limit (5 levels per the implementation notes) is respected.

6. **`register_adds_agents_that_can_be_dispatched_to`**
   - Create an `Orchestrator` with an empty agent list.
   - Call `register()` to add a new `AgentEndpoint`.
   - Create a request targeting that agent by name.
   - Call `dispatch()` and assert it succeeds, confirming the registered agent is reachable.

7. **`health_returns_healthy_when_at_least_one_downstream_agent_is_healthy`**
   - Register three agents: one `Healthy`, one `Degraded`, one `Unhealthy`.
   - Call `health()` on the `Orchestrator` (via the `MicroAgent` trait).
   - Assert it returns `HealthStatus::Healthy`.
   - Also test the edge cases:
     - All agents `Degraded` returns `Degraded`.
     - All agents `Unhealthy` returns `Unhealthy`.
     - Single healthy agent among many unhealthy returns `Healthy`.

8. **`micro_agent_invoke_delegates_to_dispatch_and_converts_errors`**
   - Cast the `Orchestrator` as `&dyn MicroAgent`.
   - Call `invoke()` with a request that would produce `OrchestratorError::NoRoute`.
   - Assert the result is `Err(AgentError::Internal(msg))` where `msg` contains the `NoRoute` error's display string.
   - Call `invoke()` with a valid request and assert success.

### Mock Server Design
- Create a helper function `start_mock_agent(name, response, health_status) -> (String, JoinHandle)` that:
  - Binds a `TcpListener` to `127.0.0.1:0`.
  - Extracts the assigned port to build the base URL `http://127.0.0.1:{port}`.
  - Spawns an `axum::serve()` task in the background.
  - Returns the base URL (for constructing `AgentEndpoint`) and the join handle (for cleanup).
- The mock server uses `Arc<dyn MicroAgent>` with a `MockAgent` struct (following the pattern from `constraint_enforcer_test.rs`) that returns the configured response/health.
- Reuse `agent_runtime::http::build_router()` to build the mock server's router -- this ensures the mock exactly matches the real HTTP API contract. This requires `agent-runtime` as a dev-dependency.
- Each test should shut down mock servers after completion (dropping the join handle or using `abort()`).

## Implementation Details

### Files to create
- `crates/orchestrator/tests/orchestrator_test.rs` -- the test file containing all 8 test cases and mock infrastructure.

### Files to modify
- `crates/orchestrator/Cargo.toml` -- ensure dev-dependencies include: `tokio = { version = "1", features = ["macros", "rt-multi-thread"] }`, `axum = "0.8"`, `serde_json = "1"`, `agent-runtime = { path = "../agent-runtime" }` (dev-dependency only, for `build_router`), `agent-sdk = { path = "../agent-sdk" }`.

### Key types and functions to add in the test file

```rust
// Mock agent for spawning HTTP servers
struct MockAgent {
    name: String,
    manifest: SkillManifest,
    response: AgentResponse,       // preconfigured response to return
    health_status: HealthStatus,
    invoked: Arc<AtomicBool>,      // tracks whether invoke was called
}

// Helper to build a SkillManifest with a given name
fn make_manifest(name: &str) -> SkillManifest { ... }

// Helper to build a mock AgentResponse with a distinguishing source marker
fn make_response(request_id: Uuid, source: &str, escalated: bool, escalate_to: Option<String>) -> AgentResponse { ... }

// Helper to start a mock HTTP server, returns (base_url, join_handle)
async fn start_mock_agent(
    name: &str,
    response: AgentResponse,
    health_status: HealthStatus,
) -> (String, tokio::task::JoinHandle<()>, Arc<AtomicBool>) { ... }

// Helper to build an AgentRequest with a target_agent context
fn make_targeted_request(target: &str) -> AgentRequest { ... }
```

### Integration points
- Imports from `orchestrator`: `Orchestrator`, `AgentEndpoint`, `OrchestratorError`
- Imports from `agent-sdk`: `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `MicroAgent`, `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`, `async_trait`
- Imports from `agent-runtime::http`: `build_router` (for mock servers)
- The tests validate the contract between `Orchestrator` and `AgentEndpoint` over HTTP, ensuring the orchestrator correctly calls downstream agents and processes their responses.

## Dependencies

- **Blocked by**: "Implement MicroAgent for Orchestrator" -- the `Orchestrator` struct, `AgentEndpoint`, `OrchestratorError`, and `MicroAgent` impl must all exist before these tests can compile.
- **Blocking**: None (non-blocking). This is a leaf task in the dependency graph.

## Risks & Edge Cases

- **Port conflicts**: Using `127.0.0.1:0` for OS-assigned ports eliminates port conflicts. Tests must extract the actual port from the bound listener before spawning the server.
- **Test isolation**: Each test spawns its own mock servers and orchestrator instance. No shared mutable state between tests. Tests can run in parallel.
- **Mock server lifecycle**: Background `tokio::spawn` tasks must be aborted after each test to prevent resource leaks. Use `JoinHandle::abort()` in cleanup or rely on the tokio runtime dropping tasks at the end of each `#[tokio::test]`.
- **Circular escalation**: The escalation chain test (test 5) must verify that the recursion limit prevents infinite loops. If agents A -> B -> A, the orchestrator should detect the cycle or hit the depth limit (5) and return `EscalationFailed`.
- **Race conditions in health checks**: The orchestrator checks agent health before dispatching. If a mock server is slow to start, the health check might fail. Mitigate by awaiting the `TcpListener::bind()` and only returning the URL after the server is ready.
- **`agent-runtime` as dev-dependency**: Adding `agent-runtime` as a dev-dependency of `orchestrator` is safe because dev-dependencies do not affect the production dependency graph. This avoids the circular dependency concern mentioned in the implementation notes (the production dependency is `orchestrator -> agent-sdk`, not `orchestrator -> agent-runtime`).
- **Escalation target not registered**: Test 5 should cover the case where `escalate_to` names an agent that does not exist in the registry, which should produce `EscalationFailed` rather than a panic.
- **Empty registry**: Ensure `dispatch()` on an orchestrator with no registered agents returns `NoRoute`, not a panic.

## Verification

- `cargo check -p orchestrator` compiles the crate and test file without errors.
- `cargo test -p orchestrator --test orchestrator_test` runs all 8 tests and they pass.
- `cargo clippy -p orchestrator` reports no warnings in the test file.
- `cargo test` (workspace-wide) shows no regressions in other crates.
- Each test assertion is specific and tests exactly one behavior (no multi-purpose tests that could mask failures).
