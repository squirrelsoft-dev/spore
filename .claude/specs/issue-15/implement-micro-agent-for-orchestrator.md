# Spec: Implement MicroAgent for Orchestrator

> From: .claude/tasks/issue-15.md

## Objective

Implement the `MicroAgent` trait (defined in `agent-sdk`) for the `Orchestrator` struct, making the orchestrator itself a first-class micro agent. This allows the orchestrator to be hosted on `agent-runtime` via the same HTTP interface (`POST /invoke`, `GET /health`) as any other agent -- the `AppState` in `agent-runtime::http` is `Arc<dyn MicroAgent>`, so any `MicroAgent` implementor is plug-compatible. The orchestrator becomes a meta-agent that receives requests, routes them to downstream agents, and reports its own health as an aggregate of its children.

## Current State

### MicroAgent trait (`crates/agent-sdk/src/micro_agent.rs`)

```rust
#[async_trait]
pub trait MicroAgent: Send + Sync {
    fn manifest(&self) -> &SkillManifest;
    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>;
    async fn health(&self) -> HealthStatus;
}
```

The trait uses `#[async_trait]` (not native async trait methods) because the orchestrator requires `Box<dyn MicroAgent>` / `Arc<dyn MicroAgent>`, and native async methods are not dyn-compatible.

### Existing MicroAgent implementation (`crates/agent-runtime/src/runtime_agent.rs`)

`RuntimeAgent` is the only existing implementor. Its patterns establish the conventions this implementation should follow:
- `manifest()` returns `&self.manifest` (a stored `SkillManifest` field).
- `invoke()` performs the agent's core work and maps domain errors to `AgentError` variants.
- `health()` returns `HealthStatus::Healthy` unconditionally (a leaf agent with no downstream dependencies).

### Orchestrator struct (not yet implemented -- defined in task spec)

Per the task breakdown, the `Orchestrator` will be defined in `crates/orchestrator/src/orchestrator.rs` with:
```rust
pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
}
```

It will have a `dispatch(&self, request: AgentRequest) -> Result<AgentResponse, OrchestratorError>` method that handles routing, health-checking, invocation, and escalation.

### HealthStatus (`crates/agent-sdk/src/health_status.rs`)

```rust
pub enum HealthStatus {
    Healthy,
    Degraded(String),
    Unhealthy(String),
}
```

`Degraded` and `Unhealthy` carry a reason string describing the issue.

### OrchestratorError (to be defined in sibling task)

Per the task breakdown, `OrchestratorError` will have variants `NoRoute`, `AgentUnavailable`, `EscalationFailed`, and `HttpError`, with a `From<OrchestratorError> for AgentError` conversion that maps all variants to `AgentError::Internal(err.to_string())`.

### AgentEndpoint (to be defined in sibling task)

`AgentEndpoint` will have a `health(&self) -> Result<HealthStatus, OrchestratorError>` method that calls `GET {url}/health` on the downstream agent and returns the parsed `HealthStatus`.

### HTTP integration (`crates/agent-runtime/src/http.rs`)

The runtime's HTTP layer uses `AppState = Arc<dyn MicroAgent>`. The `/invoke` handler calls `state.invoke(request)` and the `/health` handler calls `state.manifest()` and `state.health()`. Any `MicroAgent` implementor -- including `Orchestrator` -- can be wrapped in `Arc` and served through this same router.

### Dependency: `futures` crate

The `futures` crate (version 0.3) is already a direct dependency of both `agent-runtime` and `tool-registry`. It will need to be added to the orchestrator's `Cargo.toml` (or alternatively `futures-util` for a lighter footprint) for `futures::future::join_all`. Since it is already in the workspace lockfile, this does not introduce a new dependency to the dependency tree.

## Requirements

- Implement `MicroAgent for Orchestrator` in `crates/orchestrator/src/orchestrator.rs`.
- `manifest()` must return a reference to the orchestrator's own `SkillManifest` (the `manifest` field on the struct).
- `invoke(request)` must delegate to `self.dispatch(request)` and convert any `OrchestratorError` into `AgentError` using the `From` impl (i.e., `AgentError::Internal(err.to_string())`). The conversion can use the `?` operator with `.map_err()` or rely on the `From` impl if it is in scope.
- `health()` must concurrently query the health of all registered agents in the registry using `futures::future::join_all`.
- `health()` must aggregate downstream health statuses using these rules:
  - If the registry is empty (no downstream agents), return `HealthStatus::Healthy` (the orchestrator itself is healthy even if it has nobody to route to).
  - If at least one downstream agent reports `HealthStatus::Healthy`, the orchestrator reports `HealthStatus::Healthy`.
  - If no agent is `Healthy` but at least one is `Degraded`, the orchestrator reports `HealthStatus::Degraded` with a message listing the degraded/unhealthy agents.
  - If all agents are `Unhealthy` (or all health checks fail), the orchestrator reports `HealthStatus::Unhealthy` with a message summarizing the failures.
- Health check failures (agents that return `Err(OrchestratorError)` from `AgentEndpoint::health()`) must be treated as `Unhealthy` for aggregation purposes -- they should not cause `health()` to panic or return an error.
- The `#[async_trait]` attribute must be applied to the `impl MicroAgent for Orchestrator` block, matching the trait definition.
- Each method must stay within the 50-line function limit per project rules.

## Implementation Details

### Files to modify

1. **`crates/orchestrator/src/orchestrator.rs`** -- Add the `impl MicroAgent for Orchestrator` block after the existing `impl Orchestrator` block.

2. **`crates/orchestrator/Cargo.toml`** -- Ensure `futures = "0.3"` is listed in `[dependencies]`. This may already be handled by the "Update orchestrator Cargo.toml with dependencies" task; if not, it must be added here. The `futures` crate is already in the workspace lockfile as a transitive dependency.

### Key code structure

The implementation goes in `crates/orchestrator/src/orchestrator.rs`:

```rust
#[async_trait]
impl MicroAgent for Orchestrator {
    fn manifest(&self) -> &SkillManifest {
        &self.manifest
    }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        self.dispatch(request)
            .await
            .map_err(|e| AgentError::Internal(e.to_string()))
    }

    async fn health(&self) -> HealthStatus {
        // described below
    }
}
```

### Health aggregation logic

The `health()` method should:

1. Collect futures for all registered agents by iterating `self.registry.values()` and calling `.health()` on each `AgentEndpoint`.
2. Execute all futures concurrently with `futures::future::join_all`.
3. Convert each `Result<HealthStatus, OrchestratorError>` to a `HealthStatus`, treating `Err` as `HealthStatus::Unhealthy(err.to_string())`.
4. Classify results into three buckets: healthy count, degraded list, unhealthy list.
5. Apply the aggregation rules:
   - Empty registry -> `Healthy`
   - Any healthy -> `Healthy`
   - No healthy, some degraded -> `Degraded("N of M agents degraded: [names/reasons]")`
   - All unhealthy -> `Unhealthy("All N agents unhealthy: [names/reasons]")`

For clarity and the 50-line rule, extract the aggregation into a helper function:

```rust
fn aggregate_health(statuses: Vec<HealthStatus>) -> HealthStatus
```

This helper is a pure function (no async, no `&self`) that takes the resolved statuses and returns the aggregate. It can be a free function or an associated function on `Orchestrator`. Making it a standalone function also makes it independently unit-testable.

### Imports required in `orchestrator.rs`

```rust
use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse,
    HealthStatus, MicroAgent, SkillManifest,
};
use futures::future::join_all;
```

### Integration points

- **With `agent-runtime`:** The orchestrator can be instantiated and wrapped in `Arc<dyn MicroAgent>`, then passed to `agent_runtime::http::build_router()` or `start_server()`. This wiring is out of scope for this task but is the reason this implementation exists.
- **With `Orchestrator::dispatch()`:** The `invoke()` method is a thin adapter that calls `dispatch()` and converts the error type. All routing, health-gating, invocation, and escalation logic lives in `dispatch()`.
- **With `AgentEndpoint::health()`:** The `health()` method calls `health()` on each `AgentEndpoint` in the registry. It must handle the `Result` return type gracefully.

## Dependencies

- **Blocked by:** "Implement Orchestrator struct with dispatch logic" (Group 3) -- the `Orchestrator` struct, its `dispatch()` method, and the `registry` field must exist before the trait can be implemented.
- **Blocked by (transitively):** "Implement AgentEndpoint struct", "Define OrchestratorError enum", "Define registry config format and loader", "Convert orchestrator from binary to library crate", "Update orchestrator Cargo.toml with dependencies".
- **Blocking:** "Write unit tests for AgentEndpoint" (Group 5) and "Write unit tests for Orchestrator dispatch and routing" (Group 5). These test suites exercise the full orchestrator including its `MicroAgent` behavior.

## Risks & Edge Cases

- **Empty registry:** An orchestrator with no registered agents is a valid state (e.g., during startup before agents are discovered). `health()` should return `Healthy` in this case, not `Unhealthy`, since the orchestrator itself is functioning -- it just has no downstream agents yet. `invoke()` will return an error via `dispatch()` -> `OrchestratorError::NoRoute`, which is correct behavior.

- **All health checks fail with network errors:** If every `AgentEndpoint::health()` returns `Err`, all are treated as `Unhealthy` and the orchestrator reports `Unhealthy`. This is correct -- the orchestrator cannot serve requests if it cannot reach any downstream agent.

- **Health check latency:** `join_all` runs all checks concurrently, but a single slow agent can still delay the overall health response. Consider whether `AgentEndpoint::health()` has a built-in timeout (via `reqwest` client timeout). If not, the orchestrator's `/health` endpoint could be slow. This is a concern for the `AgentEndpoint` implementation, not for this task, but worth noting.

- **Large number of agents:** `join_all` spawns all futures at once. For a very large registry (hundreds of agents), this could create many simultaneous HTTP connections. In practice, orchestrators are expected to manage tens of agents at most, so this is not a concern for the initial implementation. If needed, `futures::stream::FuturesUnordered` with a concurrency limit could be used later.

- **`HealthStatus` matching with strings:** `Degraded(String)` and `Unhealthy(String)` carry reason strings. The aggregation must compose meaningful messages from the individual agent statuses. Include agent names in the aggregated message so operators can identify which downstream agent is causing issues.

- **Thread safety:** The `MicroAgent` trait requires `Send + Sync`. `Orchestrator` holds a `HashMap<String, AgentEndpoint>` and a `SkillManifest`, both of which are `Send + Sync` (assuming `AgentEndpoint` contains a `reqwest::Client`, which is `Send + Sync`). The `&self` receiver on all trait methods means no interior mutability is needed.

- **Error conversion in `invoke()`:** The task description says to convert `OrchestratorError` to `AgentError::Internal(err.to_string())`. This discards the structured error information (variant, fields). This is acceptable because the HTTP layer serializes `AgentError` as JSON, and the `Internal` variant's string message is sufficient for debugging. If more granular error mapping is needed later (e.g., mapping `NoRoute` to a 404-like error), the conversion can be refined.

## Verification

1. **Compilation:**
   ```bash
   cargo check -p orchestrator
   ```
   Must succeed with no errors. The `MicroAgent` impl must satisfy all trait requirements.

2. **Lint:**
   ```bash
   cargo clippy -p orchestrator
   ```
   Must succeed with no warnings.

3. **Trait object compatibility:**
   Verify that `Orchestrator` can be used as `Arc<dyn MicroAgent>`:
   ```rust
   let orch: Arc<dyn MicroAgent> = Arc::new(orchestrator);
   ```
   This must compile, confirming dyn-compatibility (object safety).

4. **Unit tests (in the subsequent test task):**
   - `manifest()` returns the correct `SkillManifest` that was provided at construction.
   - `invoke()` delegates to `dispatch()` and returns `AgentResponse` on success.
   - `invoke()` converts `OrchestratorError` to `AgentError::Internal` on failure.
   - `health()` returns `Healthy` when the registry is empty.
   - `health()` returns `Healthy` when at least one downstream agent is `Healthy`.
   - `health()` returns `Degraded` when no agent is `Healthy` but at least one is `Degraded`.
   - `health()` returns `Unhealthy` when all agents are `Unhealthy` or unreachable.
   - `health()` treats agents returning `Err` from health checks as `Unhealthy`.
   - `aggregate_health()` (if extracted as a helper) can be tested independently with synthetic `HealthStatus` vectors.

5. **Workspace integrity:**
   ```bash
   cargo test
   ```
   Full workspace tests must still pass with no regressions.
