# Spec: Integrate `SemanticRouter` into `Orchestrator`

> From: .claude/tasks/issue-16.md

## Objective

Replace the orchestrator's placeholder substring-matching heuristic (`route_by_description_match`) with the `SemanticRouter` for embedding-based routing. The orchestrator must continue to work without a `SemanticRouter` (backward compatibility for tests and deployments without an embedding model), while gaining the ability to perform cosine-similarity routing when a `SemanticRouter` is present.

## Current State

**`crates/orchestrator/src/orchestrator.rs`** defines the `Orchestrator` struct with two fields:

```rust
pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
}
```

Routing is performed by `route()`, which calls two private helpers in sequence:

1. `route_by_target_agent(&self, request: &AgentRequest) -> Option<&AgentEndpoint>` -- Checks `request.context` for a `"target_agent"` string key and does a direct registry lookup. This is the fast path.
2. `route_by_description_match(&self, request: &AgentRequest) -> Option<&AgentEndpoint>` -- Substring heuristic: lowercases both the input and each endpoint's description, then checks if any description word (3+ chars) appears in the input. Returns the first match found (nondeterministic due to HashMap iteration order). **This method is the placeholder that the `SemanticRouter` replaces.**

`dispatch()` is already `async`. It calls `self.route(&request)` synchronously, then proceeds to `try_invoke()` and `handle_escalation()`.

**`crates/orchestrator/src/lib.rs`** currently exports four modules:

```rust
pub mod agent_endpoint;
pub mod config;
pub mod error;
pub mod orchestrator;
```

**`crates/orchestrator/src/config.rs`** defines `OrchestratorConfig` with a single `agents: Vec<AgentConfig>` field. A sibling task ("Add embedding model configuration to `OrchestratorConfig`") will extend this with optional embedding provider/model/threshold fields.

**`crates/orchestrator/src/error.rs`** defines `OrchestratorError` with variants: `NoRoute`, `AgentUnavailable`, `EscalationFailed`, `HttpError`, `Config`. A prerequisite task adds `EmbeddingError { reason: String }`.

**Existing tests** in `crates/orchestrator/tests/orchestrator_test.rs` construct `Orchestrator` via `Orchestrator::new(manifest, agents)` and use `target_agent` context for routing. None of the existing tests rely on `route_by_description_match`. The `dispatch()` tests call `dispatch(request).await`.

**`SemanticRouter`** (created by a prerequisite task in `crates/orchestrator/src/semantic_router.rs`):

```rust
pub struct SemanticRouter {
    agents: Vec<AgentProfile>,
    similarity_threshold: f64,
}
```

Key API surface:
- `async fn new<M: EmbeddingModel>(model: &M, agents: Vec<(String, String)>, threshold: f64) -> Result<Self, OrchestratorError>` -- Pre-computes description embeddings.
- `async fn route<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<String, OrchestratorError>` -- Returns the agent **name** (not a reference to `AgentEndpoint`). Two-phase: (1) check `request.context` for `"intent"` key and exact-match agent names, (2) embed `request.input` and cosine-similarity match.
- `async fn register<M: EmbeddingModel>(&mut self, model: &M, name: String, description: String) -> Result<(), OrchestratorError>` -- Adds a new agent profile.

The `EmbeddingModel` trait (from rig-core) is not dyn-compatible, so it cannot be stored as `Box<dyn EmbeddingModel>`. It must be passed as a generic parameter at call sites.

## Requirements

- **R1**: Add a `semantic_router: Option<SemanticRouter>` field to the `Orchestrator` struct.
- **R2**: Update `Orchestrator::new()` to accept an optional `SemanticRouter`. Signature becomes: `pub fn new(manifest: SkillManifest, agents: Vec<AgentEndpoint>, semantic_router: Option<SemanticRouter>) -> Self`.
- **R3**: Keep the synchronous `route()` method for backward compatibility. It must continue to perform `route_by_target_agent()` as its only lookup. Remove the call to `route_by_description_match()` from `route()`, making it return `NoRoute` when no `target_agent` context is present. This preserves the existing synchronous API without losing functionality, because the semantic path is invoked from `dispatch()`.
- **R4**: Add a new `async fn route_semantic<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError>` method. This method calls `self.semantic_router.as_ref()` to get the router, calls `router.route(model, request).await` to get an agent name, then looks up the name in `self.registry`. Returns `OrchestratorError::NoRoute` if the router is absent, returns no match, or the returned name is not in the registry.
- **R5**: Modify `dispatch()` to use a three-phase routing strategy:
  1. Try `route_by_target_agent()` -- if the request has `context.target_agent`, use it. This is the fast synchronous path.
  2. If no `target_agent`, and a `SemanticRouter` is present, try `route_semantic()`. Since `dispatch()` is already async, this does not change its signature. **Note**: `dispatch()` currently does not have access to an embedding model. A new method `dispatch_with_model()` (see R6) handles this.
  3. Fall through to `OrchestratorError::NoRoute`.
- **R6**: Add `pub async fn dispatch_with_model<M: EmbeddingModel>(&self, request: AgentRequest, model: &M) -> Result<AgentResponse, OrchestratorError>`. This method performs the full three-phase routing: (1) `route_by_target_agent`, (2) `route_semantic` via the `SemanticRouter`, (3) `NoRoute`. After routing, it delegates to `try_invoke` and `handle_escalation` identically to the existing `dispatch()`.
- **R7**: The existing `dispatch()` must continue to work without an embedding model. When no `SemanticRouter` is present (or when `dispatch()` is called without a model), routing falls through to only `route_by_target_agent`, then `NoRoute`. This preserves backward compatibility with all existing tests.
- **R8**: Remove `route_by_description_match()` entirely. It is the substring heuristic that the `SemanticRouter` replaces.
- **R9**: Update `Orchestrator::from_config()` to accept and wire through a `SemanticRouter` when available. Since constructing a `SemanticRouter` requires an async embedding call, `from_config` cannot build it synchronously. Two options: (a) `from_config` passes `None` and the caller sets the router later, or (b) add `async fn from_config_with_model<M: EmbeddingModel>(config: OrchestratorConfig, model: &M) -> Result<Self, OrchestratorError>` that builds the `SemanticRouter` from the agent descriptions in config. Implement option (b) and keep option (a) as the existing `from_config` path (which passes `semantic_router: None`).
- **R10**: Update `crates/orchestrator/src/lib.rs` to add `pub mod semantic_router;`.
- **R11**: All public methods must remain under 50 lines per project rules.
- **R12**: No new crate dependencies. The `rig-core` dependency is added by a prerequisite task.

## Implementation Details

### Files to modify

**`crates/orchestrator/src/lib.rs`**

Add the `semantic_router` module export:

```rust
pub mod agent_endpoint;
pub mod config;
pub mod error;
pub mod orchestrator;
pub mod semantic_router;
```

**`crates/orchestrator/src/orchestrator.rs`**

#### Import additions

Add to the existing imports:

```rust
use crate::semantic_router::SemanticRouter;
```

Add a conditional import for `EmbeddingModel` from rig-core (used in generic method bounds):

```rust
use rig::embeddings::EmbeddingModel;
```

#### Struct change

```rust
pub struct Orchestrator {
    registry: HashMap<String, AgentEndpoint>,
    manifest: SkillManifest,
    semantic_router: Option<SemanticRouter>,
}
```

#### `new()` signature change

```rust
pub fn new(
    manifest: SkillManifest,
    agents: Vec<AgentEndpoint>,
    semantic_router: Option<SemanticRouter>,
) -> Self {
    let registry = agents
        .into_iter()
        .map(|agent| (agent.name.clone(), agent))
        .collect();
    Self { registry, manifest, semantic_router }
}
```

#### Remove `route_by_description_match()`

Delete the entire `route_by_description_match` method (lines 84-93 of the current file).

#### Simplify `route()`

The synchronous `route()` now only does the `target_agent` exact match:

```rust
pub fn route(&self, request: &AgentRequest) -> Result<&AgentEndpoint, OrchestratorError> {
    if let Some(endpoint) = self.route_by_target_agent(request) {
        return Ok(endpoint);
    }
    Err(OrchestratorError::NoRoute {
        input: request.input.clone(),
    })
}
```

#### Add `route_semantic()`

New private async method:

```rust
async fn route_semantic<M: EmbeddingModel>(
    &self,
    model: &M,
    request: &AgentRequest,
) -> Result<&AgentEndpoint, OrchestratorError> {
    let router = self.semantic_router.as_ref().ok_or_else(|| {
        OrchestratorError::NoRoute {
            input: request.input.clone(),
        }
    })?;

    let agent_name = router.route(model, request).await?;

    self.registry.get(&agent_name).ok_or_else(|| {
        OrchestratorError::NoRoute {
            input: request.input.clone(),
        }
    })
}
```

#### Add `dispatch_with_model()`

New public async method that performs the full three-phase routing:

```rust
pub async fn dispatch_with_model<M: EmbeddingModel>(
    &self,
    request: AgentRequest,
    model: &M,
) -> Result<AgentResponse, OrchestratorError> {
    let endpoint = self.route_with_model(&request, model).await?;
    let response = self.try_invoke(endpoint, &request).await?;
    let chain = vec![endpoint.name.clone()];
    self.handle_escalation(response, &request, chain).await
}
```

Add a private helper for the three-phase routing:

```rust
async fn route_with_model<M: EmbeddingModel>(
    &self,
    request: &AgentRequest,
    model: &M,
) -> Result<&AgentEndpoint, OrchestratorError> {
    // Phase 1: exact target_agent match (fast path)
    if let Some(endpoint) = self.route_by_target_agent(request) {
        return Ok(endpoint);
    }

    // Phase 2: semantic routing via embedding similarity
    if self.semantic_router.is_some() {
        return self.route_semantic(model, request).await;
    }

    // Phase 3: no route found
    Err(OrchestratorError::NoRoute {
        input: request.input.clone(),
    })
}
```

#### Modify existing `dispatch()`

The existing `dispatch()` retains its current signature for backward compatibility. Without an embedding model it can only do exact matching:

```rust
pub async fn dispatch(
    &self,
    request: AgentRequest,
) -> Result<AgentResponse, OrchestratorError> {
    let endpoint = self.route(&request)?;
    let response = self.try_invoke(endpoint, &request).await?;
    let chain = vec![endpoint.name.clone()];
    self.handle_escalation(response, &request, chain).await
}
```

This is unchanged from the current implementation (just calls `route()` which now only does `target_agent` matching).

#### Update `from_config()`

Keep the existing synchronous `from_config` but pass `None` for the semantic router:

```rust
pub fn from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError> {
    let client = build_shared_client();
    let agents: Vec<AgentEndpoint> = config
        .agents
        .into_iter()
        .map(|ac| AgentEndpoint::new(ac.name, ac.description, ac.url, client.clone()))
        .collect();

    let manifest = build_default_manifest();
    Ok(Self::new(manifest, agents, None))
}
```

#### Add `from_config_with_model()`

New async constructor that builds a `SemanticRouter` from config:

```rust
pub async fn from_config_with_model<M: EmbeddingModel>(
    config: OrchestratorConfig,
    model: &M,
    similarity_threshold: f64,
) -> Result<Self, OrchestratorError> {
    let client = build_shared_client();
    let agent_pairs: Vec<(String, String)> = config
        .agents
        .iter()
        .map(|ac| (ac.name.clone(), ac.description.clone()))
        .collect();

    let agents: Vec<AgentEndpoint> = config
        .agents
        .into_iter()
        .map(|ac| AgentEndpoint::new(ac.name, ac.description, ac.url, client.clone()))
        .collect();

    let semantic_router = SemanticRouter::new(model, agent_pairs, similarity_threshold).await?;
    let manifest = build_default_manifest();
    Ok(Self::new(manifest, agents, Some(semantic_router)))
}
```

#### Routing priority order (summary)

The complete routing priority for `dispatch_with_model`:

1. `context.target_agent` exact match (existing fast path, synchronous, in `route_by_target_agent`)
2. `context.intent` exact match (inside `SemanticRouter::route`, phase 1)
3. Embedding cosine similarity fallback (inside `SemanticRouter::route`, phase 2)
4. `OrchestratorError::NoRoute`

For the existing `dispatch()` (no model available):

1. `context.target_agent` exact match
2. `OrchestratorError::NoRoute`

### Key integration points

- `SemanticRouter::route()` returns a `String` (agent name), not an `&AgentEndpoint`. The orchestrator bridges this by looking up the name in `self.registry`.
- `EmbeddingModel` is a generic parameter on methods, not stored in the struct. This avoids making `Orchestrator` generic, which would complicate the `MicroAgent` trait implementation.
- The `MicroAgent::invoke()` implementation continues to call `dispatch()` (not `dispatch_with_model`), so the trait-based interface does not use semantic routing. A future enhancement could store a model reference, but that is out of scope.
- `route_by_target_agent()` is kept as-is since it is the fast path that both `route()` and `route_with_model()` check first.

## Dependencies

- **Blocked by**:
  - "Implement `SemanticRouter` struct with two-phase routing" -- `crates/orchestrator/src/semantic_router.rs` must exist with the `SemanticRouter` struct and its `new()`, `route()`, and `register()` methods.
  - "Add `EmbeddingError` variant to `OrchestratorError`" -- `OrchestratorError::EmbeddingError` must exist for `SemanticRouter::route` error propagation.
  - "Add `rig-core` dependency to orchestrator `Cargo.toml`" -- `rig-core` must be in dependencies for the `EmbeddingModel` trait import.

- **Blocking**:
  - "Write integration tests for semantic routing in `Orchestrator`" -- tests that exercise `dispatch_with_model` with a mock `EmbeddingModel`.

## Risks & Edge Cases

- **`SemanticRouter` returns a name not in registry**: This can happen if agents are registered in the `SemanticRouter` but not in the orchestrator's `registry`, or vice versa. The `route_semantic` method handles this by returning `NoRoute` when the registry lookup fails. Mitigation: document that `SemanticRouter` and `registry` must be kept in sync. The constructors (`new`, `from_config_with_model`) handle this by building both from the same agent list.
- **Embedding API failure during routing**: If the embedding model call fails in `route_semantic`, the error propagates as `OrchestratorError::EmbeddingError`. The caller sees a clear error rather than a silent fallback. This is intentional -- a transient embedding failure should not silently degrade to "no route found" since that would be confusing to debug.
- **Backward compatibility of `new()` signature**: Changing `new()` from 2 parameters to 3 is a breaking change for all call sites. All existing tests construct `Orchestrator::new(manifest, agents)` and must be updated to `Orchestrator::new(manifest, agents, None)`. This is a mechanical change across test files.
- **`MicroAgent::invoke()` does not use semantic routing**: The `MicroAgent` trait implementation calls `dispatch()` which has no access to an embedding model. Users who want semantic routing must call `dispatch_with_model()` directly. This is acceptable because the `MicroAgent` trait interface is fixed and cannot carry a generic model parameter.
- **Thread safety**: `SemanticRouter` stores pre-computed `Vec<AgentProfile>` and a `f64` threshold. These are read-only after construction, so `&self` access in `route()` is safe. The `Orchestrator` does not need additional synchronization.
- **`register()` does not update the `SemanticRouter`**: Calling `orchestrator.register(endpoint)` adds an agent to the `registry` but not to the `SemanticRouter`. To register an agent in both, the caller must also call `semantic_router.register(model, name, description).await`. This asymmetry is a known limitation. Documenting this in a code comment is sufficient for now.
- **Empty `SemanticRouter` (no agents)**: If the `SemanticRouter` is constructed with an empty agent list, `route()` will always return `NoRoute` from the semantic path. This is correct behavior.

## Verification

- **Compilation**: `cargo check -p orchestrator` succeeds with no errors after all prerequisite tasks are complete.
- **Lint**: `cargo clippy -p orchestrator` produces no warnings.
- **Existing tests pass**: All tests in `crates/orchestrator/tests/orchestrator_test.rs` continue to pass after updating `Orchestrator::new()` calls to include the `None` semantic router parameter. No test behavior changes.
- **Backward compatibility**: `dispatch()` without a model still works with `target_agent` context routing.
- **New API surface**: `dispatch_with_model()` compiles with a generic `EmbeddingModel` parameter and performs three-phase routing.
- **`route_by_description_match` removed**: The substring heuristic method no longer exists in the source.
- **Module export**: `use orchestrator::semantic_router::SemanticRouter;` compiles from external crates and tests.
- **Integration tests** (defined in a downstream task) will verify:
  1. `dispatch_with_model()` routes by `target_agent` context (priority 1).
  2. `dispatch_with_model()` routes by `intent` context via `SemanticRouter` (priority 2).
  3. `dispatch_with_model()` routes by embedding similarity (priority 3).
  4. `dispatch_with_model()` returns `NoRoute` when no match is found.
  5. `dispatch()` without a model returns `NoRoute` when no `target_agent` is set (does not panic or use stale heuristic).
  6. `from_config_with_model()` constructs a fully wired orchestrator with semantic routing.
