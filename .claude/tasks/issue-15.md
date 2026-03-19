# Task Breakdown: Create orchestrator crate with agent registry

> Build the `orchestrator` crate as a library implementing an agent registry that maps agent names to HTTP endpoints, with dispatch logic and `MicroAgent` trait implementation, so the orchestrator itself runs on `agent-runtime` as a homogeneous micro agent.

## Group 1 — Crate scaffold and error types

_Tasks in this group can be done in parallel._

- [x] **Convert orchestrator from binary to library crate** `[S]`
      Replace `crates/orchestrator/src/main.rs` (currently a 3-line `println!` stub) with `crates/orchestrator/src/lib.rs` that declares the crate's public modules. The orchestrator is not a standalone binary -- it runs inside `agent-runtime` -- so it must be a library crate. Declare modules: `pub mod agent_endpoint;`, `pub mod error;`, `pub mod orchestrator;`, `pub mod config;`. Delete `src/main.rs`.
      Files: `crates/orchestrator/src/lib.rs` (new), `crates/orchestrator/src/main.rs` (delete)
      Blocking: All Group 2 and Group 3 tasks

- [x] **Update orchestrator Cargo.toml with dependencies** `[S]`
      Add required dependencies to `crates/orchestrator/Cargo.toml`. Required: `agent-sdk = { path = "../agent-sdk" }`, `reqwest = { version = "0.13", features = ["json"] }`, `tokio = { version = "1", features = ["full"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `serde_yaml = "0.9"`, `async-trait = "0.1"`. All of these already exist in the workspace lockfile as transitive dependencies of other crates, so no new dependencies are introduced to the dependency tree. Dev-dependencies: `tokio = { version = "1", features = ["macros", "rt"] }`, `axum = "0.8"` (for mock HTTP test server).
      Files: `crates/orchestrator/Cargo.toml`
      Blocking: All Group 2 and Group 3 tasks

- [x] **Define OrchestratorError enum** `[S]`
      Create `crates/orchestrator/src/error.rs` with the error type following the manual `Display + Error` pattern from `crates/agent-sdk/src/agent_error.rs`. Variants: `NoRoute { input: String }` (no agent matches the request), `AgentUnavailable { name: String, reason: String }` (target agent is unhealthy or unreachable), `EscalationFailed { chain: Vec<String>, reason: String }` (escalation chain exhausted without resolution), `HttpError { url: String, reason: String }` (network/HTTP failure calling a downstream agent). Implement `Display` and `Error` for it. Add a `From<OrchestratorError>` impl that converts to `AgentError::Internal(String)` for use in the `MicroAgent::invoke()` implementation.
      Files: `crates/orchestrator/src/error.rs`
      Blocking: "Implement AgentEndpoint struct", "Implement Orchestrator struct with dispatch logic"

## Group 2 — Core types

_Depends on: Group 1._

- [x] **Implement AgentEndpoint struct** `[M]`
      Create `crates/orchestrator/src/agent_endpoint.rs`. Define:
      ```rust
      pub struct AgentEndpoint {
          pub name: String,
          pub description: String,
          pub url: String,
          client: reqwest::Client,
      }
      ```
      Implement methods:
      - `new(name, description, url) -> Self` -- constructs with a shared `reqwest::Client`
      - `invoke(&self, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>` -- POST JSON to `{url}/invoke`, deserialize response; map `reqwest` errors to `OrchestratorError::HttpError`
      - `health(&self) -> Result<HealthStatus, OrchestratorError>` -- GET `{url}/health`, deserialize the `HealthResponse` from `agent-runtime::http`, extract the `status` field; map errors to `OrchestratorError::AgentUnavailable`

      Note: The `/health` endpoint returns a `HealthResponse { name, version, status }` (defined in `crates/agent-runtime/src/http.rs`). The `AgentEndpoint::health()` should deserialize a minimal struct containing just the `status: HealthStatus` field (use `serde(default)` or a local DTO) to avoid a dependency on `agent-runtime`.
      Files: `crates/orchestrator/src/agent_endpoint.rs`
      Blocked by: "Convert orchestrator from binary to library crate", "Update orchestrator Cargo.toml with dependencies", "Define OrchestratorError enum"
      Blocking: "Implement Orchestrator struct with dispatch logic", "Implement MicroAgent for Orchestrator"

- [x] **Define registry config format and loader** `[M]`
      Create `crates/orchestrator/src/config.rs`. Define a YAML-deserializable config structure:
      ```rust
      #[derive(Deserialize)]
      pub struct OrchestratorConfig {
          pub agents: Vec<AgentConfig>,
      }
      #[derive(Deserialize)]
      pub struct AgentConfig {
          pub name: String,
          pub description: String,
          pub url: String,
      }
      ```
      Implement `OrchestratorConfig::from_file(path: &str) -> Result<Self, OrchestratorError>` that reads and parses a YAML file. Also implement `OrchestratorConfig::from_env() -> Result<Self, OrchestratorError>` that reads an `AGENT_ENDPOINTS` env var (format: `name=url,name2=url2`, similar to the `TOOL_ENDPOINTS` pattern in `crates/agent-runtime/src/main.rs` lines 86-112) with optional `AGENT_DESCRIPTIONS` env var for descriptions. The env-based approach is the primary path; YAML is secondary. Follow the config patterns from `crates/agent-runtime/src/config.rs`.
      Files: `crates/orchestrator/src/config.rs`
      Blocked by: "Convert orchestrator from binary to library crate", "Update orchestrator Cargo.toml with dependencies"
      Blocking: "Implement Orchestrator struct with dispatch logic"

## Group 3 — Orchestrator core

_Depends on: Group 2._

- [x] **Implement Orchestrator struct with dispatch logic** `[L]`
      Create `crates/orchestrator/src/orchestrator.rs`. Define:
      ```rust
      pub struct Orchestrator {
          registry: HashMap<String, AgentEndpoint>,
          manifest: SkillManifest,
      }
      ```
      Note: The `SemanticRouter` is a separate issue (#16), so the `router` field is deferred. For now, routing uses exact name matching from `request.context` or a simple keyword-in-description heuristic as a placeholder.

      Methods:
      - `new(manifest: SkillManifest, agents: Vec<AgentEndpoint>) -> Self` -- populate the registry HashMap keyed by agent name
      - `register(&mut self, endpoint: AgentEndpoint)` -- add a single agent to the registry
      - `route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError>` -- look up agent by name from `request.context` (if it contains a `"target_agent": "name"` field), or iterate registry entries and do basic substring matching of `request.input` against each endpoint's description; return `OrchestratorError::NoRoute` if none match
      - `dispatch(&self, request: AgentRequest) -> Result<AgentResponse, OrchestratorError>` -- call `route()`, check agent health (skip unhealthy agents), call `agent.invoke()`, if response has `escalated: true` and `escalate_to: Some(name)` then look up the escalation target and re-dispatch (with a recursion limit to prevent infinite escalation chains)
      - `from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError>` -- build from parsed config, constructing `AgentEndpoint` instances and a default `SkillManifest` for the orchestrator itself

      Keep each method under 50 lines per the project rule. Break `dispatch` into helpers: `try_invoke`, `handle_escalation`.
      Files: `crates/orchestrator/src/orchestrator.rs`
      Blocked by: "Implement AgentEndpoint struct", "Define registry config format and loader", "Define OrchestratorError enum"
      Blocking: "Implement MicroAgent for Orchestrator"

## Group 4 — MicroAgent implementation

_Depends on: Group 3._

- [x] **Implement MicroAgent for Orchestrator** `[M]`
      In `crates/orchestrator/src/orchestrator.rs`, implement the `MicroAgent` trait for `Orchestrator`:
      - `manifest()` -- return `&self.manifest` (the orchestrator's own `SkillManifest`)
      - `invoke(request)` -- delegate to `self.dispatch(request)`, converting `OrchestratorError` to `AgentError::Internal(err.to_string())`
      - `health()` -- iterate all registered agents, call `health()` on each; return `Healthy` if at least one downstream agent is `Healthy`, `Degraded` if all are `Degraded`, `Unhealthy` if all are `Unhealthy`. Use `futures::future::join_all` for concurrent health checks.

      This is what makes the orchestrator itself a micro agent that can run on `agent-runtime` via the same HTTP interface as any other agent.
      Files: `crates/orchestrator/src/orchestrator.rs`
      Blocked by: "Implement Orchestrator struct with dispatch logic"
      Blocking: "Write unit tests for AgentEndpoint"

## Group 5 — Tests and verification

_Depends on: Group 4._

- [x] **Write unit tests for OrchestratorError** `[S]`
      Create `crates/orchestrator/tests/error_test.rs`. Test `Display` output for all four variants. Test `From<OrchestratorError>` conversion to `AgentError::Internal`. Follow the pattern from `crates/agent-sdk/tests/envelope_types_test.rs` and the inline tests in `crates/agent-runtime/src/provider.rs`.
      Files: `crates/orchestrator/tests/error_test.rs`
      Blocked by: "Define OrchestratorError enum"
      Non-blocking

- [x] **Write unit tests for AgentEndpoint** `[M]`
      Create `crates/orchestrator/tests/agent_endpoint_test.rs`. Use an `axum` test server (similar to how `crates/agent-runtime/tests/http_test.rs` works) to stand up a mock `/invoke` and `/health` endpoint. Tests:
      1. `invoke()` sends correct JSON and returns deserialized `AgentResponse`
      2. `invoke()` maps HTTP errors to `OrchestratorError::HttpError`
      3. `health()` returns correct `HealthStatus`
      4. `health()` maps connection failures to `OrchestratorError::AgentUnavailable`
      Files: `crates/orchestrator/tests/agent_endpoint_test.rs`
      Blocked by: "Implement MicroAgent for Orchestrator"
      Non-blocking

- [x] **Write unit tests for Orchestrator dispatch and routing** `[M]`
      Create `crates/orchestrator/tests/orchestrator_test.rs`. Use mock HTTP servers for downstream agents. Tests:
      1. `dispatch()` routes to correct agent based on `request.context` target
      2. `dispatch()` returns `NoRoute` when no agent matches
      3. `dispatch()` skips unhealthy agents
      4. `dispatch()` handles escalation (agent returns `escalated: true`, orchestrator re-routes to `escalate_to` agent)
      5. `dispatch()` returns `EscalationFailed` when escalation chain is exhausted
      6. `register()` adds agents that can be dispatched to
      7. `health()` returns `Healthy` when at least one downstream agent is healthy
      8. `MicroAgent::invoke()` delegates to `dispatch()` and converts errors
      Follow the mock pattern from `crates/agent-runtime/tests/constraint_enforcer_test.rs`.
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Implement MicroAgent for Orchestrator"
      Non-blocking

- [x] **Write unit tests for config loading** `[S]`
      Create `crates/orchestrator/tests/config_test.rs`. Tests:
      1. YAML config parses correctly with multiple agents
      2. Empty agents list is valid
      3. Malformed YAML returns appropriate error
      4. Env-based config parses `AGENT_ENDPOINTS` format correctly
      5. Missing env var returns error
      Follow the env-testing pattern from `crates/agent-runtime/src/config.rs` (using a mutex to serialize env-modifying tests).
      Files: `crates/orchestrator/tests/config_test.rs`
      Blocked by: "Define registry config format and loader"
      Non-blocking

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify no regressions in existing crates (`agent-sdk`, `agent-runtime`, `skill-loader`, `tool-registry`). Verify all new orchestrator tests pass.
      Files: (none -- command-line verification only)
      Blocked by: All other tasks in this group

## Implementation Notes

1. **Library, not binary**: The orchestrator runs inside `agent-runtime` as a `MicroAgent` (same as `RuntimeAgent`). The current `src/main.rs` stub must be replaced with `src/lib.rs`. The `agent-runtime` binary will eventually import and instantiate the orchestrator, but wiring it into `agent-runtime/src/main.rs` is out of scope for this issue.

2. **SemanticRouter is deferred to issue #16**: The triage comment mentions a `router: SemanticRouter` field, but issue #16 is a separate issue for semantic routing. This issue should implement a simple placeholder routing strategy (exact name match from context, then basic description matching). The `Orchestrator` struct should be designed so the `SemanticRouter` can be plugged in later without major refactoring.

3. **`reqwest` is already a transitive dependency**: Version 0.13.2 is in the lockfile via `rig-core`. Adding it as a direct dependency of `orchestrator` does not introduce a new crate to the dependency tree, which aligns with the project rule to avoid unnecessary new dependencies.

4. **Health response DTO**: The `/health` endpoint returns `HealthResponse { name, version, status }` defined in `agent-runtime::http`. Rather than making `orchestrator` depend on `agent-runtime` (which would create a circular dependency since `agent-runtime` will eventually depend on `orchestrator`), define a minimal local deserialization struct in `agent_endpoint.rs` that only extracts the `status` field.

5. **Escalation recursion limit**: The `dispatch()` method must cap escalation depth (e.g., 5 levels) to prevent infinite loops when agents escalate to each other. Track the chain in `EscalationFailed { chain: Vec<String> }`.

6. **No dependency on `agent-runtime`**: The orchestrator depends on `agent-sdk` (for `MicroAgent`, `AgentRequest`, `AgentResponse`, etc.) but must NOT depend on `agent-runtime` to avoid circular dependencies. HTTP communication with downstream agents goes through `reqwest`, not through Rust-level imports.
