# Spec: Add escalated-with-no-target test
> From: .claude/tasks/issue-17.md

## Objective
Add a test `dispatch_returns_response_when_escalated_without_target` that verifies the orchestrator gracefully handles the case where an agent returns `escalated: true` but `escalate_to: None`. The test must assert that `dispatch()` returns `Ok(response)` containing the original response as-is, rather than erroring or attempting further routing.

## Current State
The `handle_escalation` method in `crates/orchestrator/src/orchestrator.rs` (lines 180-213) already handles this case correctly. When `current_response.escalated` is `true` but `escalate_to` is `None`, the loop at line 194-197 matches `None` and returns `Ok(current_response)` immediately. However, there is no test exercising this specific branch.

Existing escalation tests cover:
- `dispatch_handles_escalation` -- agent A escalates to agent B with an explicit target (happy path)
- `dispatch_returns_escalation_failed_on_depth` -- chain exceeds `MAX_ESCALATION_DEPTH`

Neither test covers the `escalated: true` + `escalate_to: None` combination.

## Requirements
1. Add a new `#[tokio::test]` named `dispatch_returns_response_when_escalated_without_target` to `crates/orchestrator/tests/orchestrator_test.rs`.
2. The test must register a single agent ("agent-a") that returns an `AgentResponse` with `escalated: true` and `escalate_to: None`.
3. Route to "agent-a" via `context: Some(json!({"target_agent": "agent-a"}))`.
4. Assert `dispatch()` returns `Ok(response)`.
5. Assert the returned response matches the original response from agent-a (same `id`, `output`, `confidence`, `escalated`, `escalate_to`, `tool_calls`).
6. Follow existing test conventions: use `create_mock_endpoint`, `build_test_manifest`, `Orchestrator::new`, and the same assertion style already in the file.

## Implementation Details

### New helper function
Add a builder alongside `build_success_response` and `build_escalation_response`:

```rust
/// Builds an `AgentResponse` with `escalated: true` but no escalation target.
fn build_escalated_no_target_response(request_id: uuid::Uuid, output_msg: &str) -> AgentResponse {
    AgentResponse {
        id: request_id,
        output: json!({"result": output_msg}),
        confidence: 0.5,
        escalated: true,
        escalate_to: None,
        tool_calls: vec![],
    }
}
```

### Test function
```rust
#[tokio::test]
async fn dispatch_returns_response_when_escalated_without_target() {
    let request_id = uuid::Uuid::new_v4();

    let agent_a = create_mock_endpoint(
        "agent-a",
        "agent that escalates without target",
        HealthStatus::Healthy,
        build_escalated_no_target_response(request_id, "no-target-escalation"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a], None);

    let request = AgentRequest {
        id: request_id,
        input: "test escalation without target".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "no-target-escalation"}));
    assert!(response.escalated);
    assert!(response.escalate_to.is_none());
}
```

### Placement
Insert the new helper after `build_escalation_response` (after line 126). Insert the new test after `dispatch_returns_escalation_failed_on_depth` (after line 380), keeping escalation-related tests grouped together.

## Dependencies
- **Blocked by:** "Add structured tracing to escalation path" -- that task may add tracing instrumentation to `handle_escalation`. This test should be written after that work lands to avoid merge conflicts in the same code region.
- **No new crate dependencies required.** The test uses only existing imports and helpers.

## Risks & Edge Cases
- **Low risk.** The code path under test (`escalate_to: None` branch at line 196-197) is straightforward and already implemented.
- **Edge case covered by this test:** An agent might legitimately set `escalated: true` to signal that it could not fully handle the request, but leave `escalate_to: None` because it does not know which agent should handle it. The orchestrator must not panic, error, or loop -- it returns the response as-is, letting the caller decide.
- **No regression risk:** This is a purely additive test; no production code changes are needed.

## Verification
1. `cargo test --package orchestrator dispatch_returns_response_when_escalated_without_target` -- the new test passes.
2. `cargo test --package orchestrator` -- all existing orchestrator tests continue to pass.
3. `cargo clippy --package orchestrator --tests` -- no new warnings.
