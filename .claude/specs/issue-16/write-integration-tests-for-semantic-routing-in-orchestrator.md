# Spec: Write integration tests for semantic routing in `Orchestrator`

> From: .claude/tasks/issue-16.md

## Objective

Add integration tests to `crates/orchestrator/tests/orchestrator_test.rs` that exercise the semantic routing path through `dispatch()`. These tests verify that the `SemanticRouter`, once integrated into `Orchestrator`, correctly routes requests by intent context, by embedding-based cosine similarity, and that it returns `NoRoute` when no agent matches. They also confirm that the existing `target_agent` fast path still takes priority over semantic routing, and that orchestrators constructed without a `SemanticRouter` continue to work as before (backward compatibility).

## Current State

### Existing test file: `crates/orchestrator/tests/orchestrator_test.rs`

The file contains 8 integration tests exercising the orchestrator's dispatch, routing, escalation, health, and MicroAgent trait delegation. Key patterns used:

- **Mock HTTP servers** via `axum`: `start_mock_agent()` spins up a real HTTP server on `127.0.0.1:0` with `/health` and `/invoke` endpoints. The `MockAgentConfig` struct drives the canned responses.
- **`create_mock_endpoint()`**: Combines the mock server with an `AgentEndpoint`, returning a ready-to-register endpoint.
- **`build_test_manifest()`**: Returns a minimal `SkillManifest` for orchestrator construction.
- **`build_success_response()` / `build_escalation_response()`**: Factory helpers for `AgentResponse` instances.
- **Orchestrator construction**: `Orchestrator::new(manifest, vec![agent1, agent2])` -- currently takes a `SkillManifest` and a `Vec<AgentEndpoint>`. No semantic router is passed.
- **Routing via context**: All tests use `context: Some(json!({"target_agent": "agent-name"}))` to select the target agent.

### Planned changes from blocking tasks

The "Integrate `SemanticRouter` into `Orchestrator`" task will:
- Add an `Option<SemanticRouter>` field to `Orchestrator`.
- Modify `dispatch()` to: (1) try `route_by_target_agent()` exact match, (2) try `SemanticRouter` if present, (3) return `NoRoute`. The `route_by_description_match()` substring heuristic will be removed.
- The `Orchestrator::new()` signature will change to accept an optional `SemanticRouter`, or a new constructor/builder will be added.

The `SemanticRouter` (from the "Implement `SemanticRouter` struct" task):
- Constructor: `async fn new<M: EmbeddingModel>(model: &M, agents: Vec<(String, String)>, threshold: f64) -> Result<Self, OrchestratorError>` -- pre-computes description embeddings.
- Routing: `async fn route<M: EmbeddingModel>(&self, model: &M, request: &AgentRequest) -> Result<String, OrchestratorError>` -- Phase 1: intent exact match from `context.intent`, Phase 2: cosine similarity fallback.
- The embedding model is passed by reference (not stored), so `dispatch()` in the orchestrator will also need access to the model at call time.

The "Add embedding model configuration to `OrchestratorConfig`" task adds `embedding_provider`, `embedding_model`, and `similarity_threshold` fields to `OrchestratorConfig`. These are not directly used in the integration tests (tests construct the orchestrator manually, not via `from_config()`), but the overall API shape informs how the orchestrator will be assembled.

### `MockEmbeddingModel` strategy (from the unit test task spec)

The "Write unit tests for `SemanticRouter`" task specifies a `MockEmbeddingModel` that maps known strings to fixed embedding vectors:
- `"financial queries"` -> `[1.0, 0.0, 0.0]`
- `"weather forecasts"` -> `[0.0, 1.0, 0.0]`
- `"What are my expenses?"` -> `[0.9, 0.1, 0.0]` (high similarity to financial)
- `"random gibberish"` -> `[0.3, 0.3, 0.3]` (low similarity to everything)

This mock implements rig-core's `EmbeddingModel` trait, returning deterministic `Embedding` vectors without any network calls.

## Requirements

1. **Intent-based routing through dispatch**: A request with `context: {"intent": "finance-agent"}` dispatched through an orchestrator with a `SemanticRouter` must route to the agent named `"finance-agent"` and return that agent's response. No `target_agent` context key is set.

2. **Semantic similarity routing through dispatch**: A request with input `"What are my expenses?"` and no `intent` or `target_agent` context must be routed to the agent whose description has the highest cosine similarity above threshold (the "financial" agent). The mock HTTP server for that agent must receive the invocation and return its canned response.

3. **NoRoute when no agent matches semantically**: A request with input that produces a low-similarity embedding (e.g., `"random gibberish"`) and no `intent` or `target_agent` context must result in `OrchestratorError::NoRoute` when dispatched through an orchestrator with a `SemanticRouter`.

4. **target_agent takes priority over intent**: A request with both `context: {"target_agent": "agent-a", "intent": "agent-b"}` must route to `"agent-a"` (the `target_agent` fast path in the orchestrator), not `"agent-b"` (the intent path in the `SemanticRouter`). This confirms the orchestrator's routing priority order: target_agent > intent > semantic similarity.

5. **Backward compatibility**: All 8 existing tests must continue to pass without modification. An orchestrator constructed without a `SemanticRouter` (i.e., `None`) must still work with exact `target_agent` matching. This requirement is verified by running the existing tests, not by writing new ones.

## Implementation Details

### File to modify: `crates/orchestrator/tests/orchestrator_test.rs`

### New imports

Add the following imports at the top of the file (exact imports depend on the final `SemanticRouter` and rig-core API):

```rust
use orchestrator::semantic_router::SemanticRouter;
use rig_core::embeddings::{Embedding, EmbeddingModel};
```

### `MockEmbeddingModel` struct

Define a `MockEmbeddingModel` in the test file (or import from a shared test utilities module if the unit test task creates one). The mock should:

- Store a `HashMap<String, Vec<f64>>` mapping known input strings to fixed embedding vectors.
- Implement rig-core's `EmbeddingModel` trait:
  - `embed_text(text)` looks up the text in the map; if not found, returns a zero vector (or a low-magnitude default).
  - `embed_texts(texts)` calls `embed_text` for each.
  - `ndims()` returns the dimensionality (e.g., 3).
- Use the same vector assignments from the unit test task:
  - Agent descriptions: `"Handles financial queries"` -> `[1.0, 0.0, 0.0]`, `"Provides weather forecasts"` -> `[0.0, 1.0, 0.0]`.
  - Request inputs: `"What are my expenses?"` -> `[0.9, 0.1, 0.0]`, `"random gibberish"` -> `[0.3, 0.3, 0.3]`.

### Helper: `create_semantic_orchestrator()`

Add a helper function that constructs an `Orchestrator` wired with a `SemanticRouter` and mock HTTP agent backends:

```rust
async fn create_semantic_orchestrator(
    mock_model: &MockEmbeddingModel,
    agents: Vec<(&str, &str, HealthStatus, AgentResponse)>,
) -> Orchestrator {
    // 1. Create mock HTTP endpoints for each agent
    // 2. Build (name, description) pairs for SemanticRouter::new()
    // 3. Construct SemanticRouter with mock_model and threshold 0.7
    // 4. Construct Orchestrator with the SemanticRouter and AgentEndpoints
}
```

The exact signature will adapt to the final `Orchestrator` constructor API. If the orchestrator stores the `SemanticRouter` but needs the model passed to `dispatch()`, the helper should return both the orchestrator and model, or the tests should keep the model in scope.

### Test 1: `dispatch_with_semantic_router_routes_by_intent`

```
Setup:
  - Two agents: "finance-agent" (desc: "Handles financial queries") and
    "weather-agent" (desc: "Provides weather forecasts")
  - Both healthy, each returns a distinct success response
  - Orchestrator constructed with SemanticRouter
Request:
  - input: "something unrelated"
  - context: {"intent": "finance-agent"}
  - No target_agent
Assert:
  - dispatch() returns Ok with finance-agent's response
```

### Test 2: `dispatch_with_semantic_router_routes_by_similarity`

```
Setup:
  - Same two agents as Test 1
  - Orchestrator constructed with SemanticRouter
Request:
  - input: "What are my expenses?"
  - context: None (or empty, no target_agent, no intent)
Assert:
  - dispatch() returns Ok with finance-agent's response
  - (The mock embedding for this input has highest cosine similarity to
    the financial agent's description embedding)
```

### Test 3: `dispatch_with_semantic_router_returns_no_route`

```
Setup:
  - Same two agents as Test 1
  - Orchestrator constructed with SemanticRouter (threshold 0.7)
Request:
  - input: "random gibberish"
  - context: None
Assert:
  - dispatch() returns Err(OrchestratorError::NoRoute { .. })
  - (The mock embedding [0.3, 0.3, 0.3] has cosine similarity below 0.7
    against both [1.0, 0.0, 0.0] and [0.0, 1.0, 0.0])
```

### Test 4: `dispatch_with_semantic_router_prefers_target_agent_over_intent`

```
Setup:
  - Two agents: "agent-a" and "agent-b", both healthy, distinct responses
  - Orchestrator constructed with SemanticRouter
Request:
  - input: "test"
  - context: {"target_agent": "agent-a", "intent": "agent-b"}
Assert:
  - dispatch() returns Ok with agent-a's response
  - (target_agent takes priority in the orchestrator's routing chain,
    before the SemanticRouter is even consulted)
```

### Test 5: Backward compatibility (no new test needed)

The existing 8 tests already verify that an orchestrator without a `SemanticRouter` works correctly. After the "Integrate `SemanticRouter`" task modifies `Orchestrator::new()`, those tests must continue to pass by constructing the orchestrator without a semantic router (passing `None` or using a backward-compatible constructor). If the constructor signature changes, the existing tests will need a minimal update (e.g., adding `None` for the semantic router parameter), but the test logic and assertions remain unchanged.

**Important**: If the `Orchestrator::new()` signature changes to require an `Option<SemanticRouter>`, the 8 existing test calls to `Orchestrator::new(manifest, agents)` will need to become `Orchestrator::new(manifest, agents, None)` (or equivalent). This is a mechanical change, not a logic change. Document this as part of the integration task, not this test task.

### Embedding model lifetime consideration

The `SemanticRouter::route()` method takes `&M` where `M: EmbeddingModel`. The `Orchestrator::dispatch()` method is `async fn dispatch(&self, request)`. The integration task will need to decide how the model is threaded through. Two likely patterns:

- **Option A**: The `Orchestrator` stores a `Box<dyn EmbeddingModel>` (not possible -- the trait is not dyn-compatible per implementation notes).
- **Option B**: The `Orchestrator` stores an `Option<SemanticRouter>` and `dispatch()` gains a generic model parameter or the model is stored via type erasure.
- **Option C**: A new `dispatch_with_model<M>(&self, model: &M, request)` method is added, and `dispatch()` only does exact matching.

The tests must align with whichever pattern the integration task chooses. The spec describes the test intent; exact method signatures will be adapted during implementation.

## Dependencies

- **Blocked by**:
  - "Integrate `SemanticRouter` into `Orchestrator`" -- the `Orchestrator` must accept and use a `SemanticRouter` for the new tests to compile
  - "Add embedding model configuration to `OrchestratorConfig`" -- while tests construct manually (not via config), the overall API design from this task influences constructor signatures
- **Blocking**: Nothing (non-blocking task)

## Risks & Edge Cases

- **rig-core `EmbeddingModel` trait instability**: The mock must implement rig-core 0.32's `EmbeddingModel` trait exactly. If the trait has associated types, default methods, or async-fn-in-trait constraints that are hard to mock, the `MockEmbeddingModel` may need careful construction. Mitigation: review the exact trait definition in rig-core 0.32 before implementing.
- **Cosine similarity edge cases**: The mock vectors are chosen so that similarity results are deterministic and clearly above or below threshold. For example, `cosine_similarity([0.9, 0.1, 0.0], [1.0, 0.0, 0.0])` is approximately 0.994 (well above 0.7), and `cosine_similarity([0.3, 0.3, 0.3], [1.0, 0.0, 0.0])` is approximately 0.577 (below 0.7). These values should be verified during implementation.
- **Constructor signature changes**: If `Orchestrator::new()` changes its signature (e.g., adding an `Option<SemanticRouter>` parameter), the 8 existing tests will need a mechanical update. This is expected and documented above. The key risk is that the integration task's constructor design is not finalized when this test task begins. Mitigation: this task is blocked by the integration task.
- **Async test ordering**: Tests that share no state are independent. The mock HTTP servers bind to random ports, so there is no port conflict risk. No shared mutable state exists between tests.
- **Model lifetime across async boundaries**: The `MockEmbeddingModel` must live long enough for `dispatch()` to complete. Since it is created at the start of each test and the test awaits `dispatch()`, this is straightforward. If the orchestrator stores a reference to the model, the test must ensure the model outlives the orchestrator. Likely the model is passed per-call, making this a non-issue.
- **Removing `route_by_description_match()`**: The integration task removes the substring heuristic. The existing `dispatch_returns_no_route` test currently triggers `NoRoute` by specifying a nonexistent `target_agent`. With the heuristic removed, an orchestrator without a `SemanticRouter` will always return `NoRoute` when `target_agent` doesn't match -- this is fine since the existing test already uses a nonexistent target. However, if any existing test relied on the substring heuristic (none currently do), it would break.

## Verification

- `cargo test -p orchestrator` passes with all 8 existing tests and all 4 new tests green.
- `cargo clippy -p orchestrator` produces no warnings in the test file.
- Each new test is independent and can be run individually via `cargo test -p orchestrator <test_name>`.
- The 4 new tests cover the 4 specified routing scenarios: intent routing, similarity routing, no-match, and priority ordering.
- Backward compatibility is confirmed by the 8 existing tests continuing to pass unchanged (or with only mechanical constructor signature updates).
