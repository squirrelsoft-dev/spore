# Spec: Add escalation-via-semantic-routing test

> From: .claude/tasks/issue-17.md

## Objective

Add a test `dispatch_with_model_handles_escalation` to `crates/orchestrator/tests/orchestrator_test.rs` that verifies the escalation path works correctly when dispatch is initiated via semantic routing (`dispatch_with_model()`). This confirms that escalation behavior is identical regardless of whether the initial agent was selected by context-based routing (`dispatch()`) or by semantic similarity routing (`dispatch_with_model()`).

## Current State

### Existing escalation test: `dispatch_handles_escalation`

The file already contains a test at line 289 that exercises escalation through the context-based `dispatch()` path:
- Agent A is routed to via `context: {"target_agent": "agent-a"}`.
- Agent A returns an escalation response pointing to `"agent-b"`.
- Agent B returns a success response.
- The test asserts that `dispatch()` returns Agent B's success response.

This test only covers escalation when the initial route was determined by `target_agent` context. It does not cover the case where the initial agent was selected via semantic similarity.

### Existing semantic routing tests

Four semantic routing tests exist (lines 477-645), all using `dispatch_with_model()`:
- `dispatch_with_semantic_router_routes_by_intent` -- routes via `context.intent`.
- `dispatch_with_semantic_router_routes_by_similarity` -- routes via cosine similarity of input embedding.
- `dispatch_with_semantic_router_returns_no_route` -- verifies `NoRoute` when similarity is below threshold.
- `dispatch_with_semantic_router_prefers_target_agent_over_intent` -- verifies `target_agent` priority.

None of these tests exercise escalation after semantic routing. They all use `build_success_response()`.

### Relevant test infrastructure already in place

- `MockEmbeddingModel` (line 134): Maps known strings to fixed 3D vectors. Key mappings: `"Handles financial queries"` -> `[1.0, 0.0, 0.0]`, `"What are my expenses?"` -> `[0.9, 0.1, 0.0]`.
- `build_semantic_router()` (line 179): Builds a `SemanticRouter` with `"finance-agent"` and `"weather-agent"` using threshold `0.7`.
- `build_escalation_response()` (line 114): Creates an `AgentResponse` with `escalated: true` and a named `escalate_to` target.
- `create_mock_endpoint()` (line 61): Spins up a mock HTTP agent with canned responses.

### How `dispatch_with_model()` handles escalation

In `orchestrator.rs` (line 124), `dispatch_with_model()` calls `route_with_model()` to find the initial endpoint, invokes it via `try_invoke()`, then delegates to `handle_escalation()` -- the exact same escalation handler used by `dispatch()`. The escalation path itself does not use the embedding model; it resolves targets by name from the registry. This means the code path is shared, but it has never been tested end-to-end through `dispatch_with_model()`.

## Requirements

1. **Test name**: `dispatch_with_model_handles_escalation`.

2. **Semantic routing to initial agent**: The test must route the request to the initial agent via semantic similarity (not `target_agent` context), confirming that the entry point is through the semantic routing path. The input must produce an embedding with cosine similarity above 0.7 against the initial agent's description embedding.

3. **Escalation to second agent**: The initial agent must return an escalation response (using `build_escalation_response()`) pointing to a second agent registered in the orchestrator. The second agent must return a success response (using `build_success_response()`).

4. **Assertion**: `dispatch_with_model()` must return `Ok` with the second agent's success response, confirming that the full chain (semantic route -> invoke -> escalate -> invoke) works end-to-end.

5. **Follow existing patterns**: Use `MockEmbeddingModel`, `build_semantic_router()`, `create_mock_endpoint()`, and `Orchestrator::new()` with `Some(router)` exactly as the existing semantic routing tests do.

## Implementation Details

### File to modify: `crates/orchestrator/tests/orchestrator_test.rs`

### MockEmbeddingModel vector addition

The `MockEmbeddingModel::vector_for()` method (line 137) needs a new entry for the escalation target agent's description. The escalation target does not need to be in the `SemanticRouter` (escalation resolves by name, not by embedding), but it does need to be registered in the orchestrator's `AgentEndpoint` registry. No new vectors are required unless the escalation target's description happens to be one of the strings passed to `embed_texts` during `SemanticRouter::new()`. Since `build_semantic_router()` only registers `"finance-agent"` and `"weather-agent"`, the escalation target can be a third agent (e.g., `"escalation-handler"`) whose description does not need an embedding mapping.

**Option A (recommended)**: Register the escalation target only in the orchestrator's endpoint registry (via `Orchestrator::new()`), not in the `SemanticRouter`. This mirrors the real-world pattern where an escalation target may not be semantically routable. No changes to `MockEmbeddingModel` are needed.

**Option B**: Add the escalation target to both the `SemanticRouter` and the orchestrator registry. This would require adding a vector mapping for the target's description in `MockEmbeddingModel::vector_for()`. This is unnecessary complexity for this test.

### Test structure

```
Test: dispatch_with_model_handles_escalation

Setup:
  - model = MockEmbeddingModel
  - router = build_semantic_router(&model) -- registers finance-agent and weather-agent
  - finance_agent = create_mock_endpoint(
      "finance-agent",
      "Handles financial queries",
      Healthy,
      build_escalation_response(request_id, "escalation-handler")
    )
  - weather_agent = create_mock_endpoint(
      "weather-agent",
      "Handles weather forecasts",
      Healthy,
      build_success_response(request_id, "from-weather")  -- not expected to be called
    )
  - escalation_handler = create_mock_endpoint(
      "escalation-handler",
      "Handles escalated requests",
      Healthy,
      build_success_response(request_id, "handled-by-escalation")
    )
  - orchestrator = Orchestrator::new(
      build_test_manifest(),
      vec![finance_agent, weather_agent, escalation_handler],
      Some(router)
    )

Request:
  - input: "What are my expenses?"  -- embeds to [0.9, 0.1, 0.0], similar to finance [1.0, 0.0, 0.0]
  - context: None  -- no target_agent, no intent; forces semantic similarity routing
  - caller: None

Execution:
  - orchestrator.dispatch_with_model(request, &model).await

Assertions:
  - Result is Ok
  - response.output == json!({"result": "handled-by-escalation"})
```

### Placement in the test file

Add the test after the existing semantic routing tests (after line 645), within the "Semantic routing tests" section. The test logically belongs with the semantic routing tests because it validates behavior initiated through `dispatch_with_model()`.

### No helper function changes needed

The existing `build_semantic_router()` helper builds a router with `finance-agent` and `weather-agent`. The test can reuse this directly. The escalation target (`escalation-handler`) is only added to the orchestrator's endpoint registry, not to the semantic router.

## Dependencies

- **Blocked by**: "Add structured tracing to escalation path" -- the task description explicitly states this dependency. The tracing additions to `handle_escalation()` must land first so that the test exercises the instrumented code path.
- **Blocking**: Nothing (non-blocking task).
- **Requires**: The existing `MockEmbeddingModel`, `build_semantic_router()`, `create_mock_endpoint()`, `build_escalation_response()`, `build_success_response()`, and `build_test_manifest()` helpers. All are already present in the test file.

## Risks & Edge Cases

- **Escalation target description embedding**: The escalation target's description (`"Handles escalated requests"`) will be passed to `MockEmbeddingModel::vector_for()` only if it is registered in the `SemanticRouter`. Since the recommended approach registers it only in the orchestrator's endpoint registry (not the semantic router), no embedding lookup occurs for this description. If for some reason the `SemanticRouter` constructor or the escalation path triggers an embedding for the target's description, the `MockEmbeddingModel` will return `[0.0, 0.0, 0.0]` (the default for unknown strings), which is safe.
- **Shared escalation handler**: The `handle_escalation()` method (line 180 of `orchestrator.rs`) is the same code path for both `dispatch()` and `dispatch_with_model()`. This test's value is confirming the end-to-end integration, not testing new escalation logic. If `handle_escalation()` is refactored, this test still provides a valid integration check.
- **Mock HTTP server response is static**: The mock agent always returns the same canned response regardless of the request. The escalation handler's response includes `escalated: false`, so the escalation loop terminates after one hop. No risk of infinite loops.
- **Test independence**: The test creates its own mock servers on random ports, its own `MockEmbeddingModel` (zero-allocation struct), and its own `Orchestrator`. No shared state with other tests.

## Verification

- `cargo test -p orchestrator dispatch_with_model_handles_escalation` passes.
- `cargo test -p orchestrator` passes with all existing tests and the new test green.
- `cargo clippy -p orchestrator` produces no new warnings.
- The test confirms that `dispatch_with_model()` returns the escalation target's response (not the initial agent's escalation response), verifying the full semantic-route-then-escalate path.
