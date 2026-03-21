# Spec: Add cycle detection test
> From: .claude/tasks/issue-17.md

## Objective
Add an integration test `dispatch_returns_escalation_failed_on_cycle` to `crates/orchestrator/tests/orchestrator_test.rs` that verifies the orchestrator detects and rejects escalation cycles (agent A escalates to agent B, agent B escalates back to agent A).

## Current State
- The orchestrator already implements cycle detection in `Orchestrator::validate_no_cycle` (`crates/orchestrator/src/orchestrator.rs`, lines 231-243). It checks whether `target_name` already exists in the escalation `chain` and returns `OrchestratorError::EscalationFailed` with a reason containing `"cycle detected"`.
- The existing test `dispatch_returns_escalation_failed_on_depth` (lines 323-380 in the test file) validates the *depth* limit using the same `EscalationFailed` variant but with `"max escalation depth"` in the reason string. The new test is structurally similar but exercises the *cycle* guard instead.
- Helper functions `create_mock_endpoint`, `build_escalation_response`, `build_success_response`, `build_test_manifest`, and the `MockAgentConfig` / `start_mock_agent` infrastructure are already available in the test file.

## Requirements
1. Create a `#[tokio::test]` named `dispatch_returns_escalation_failed_on_cycle`.
2. Stand up two mock agents:
   - `"agent-a"` -- returns an escalation response targeting `"agent-b"`.
   - `"agent-b"` -- returns an escalation response targeting `"agent-a"`.
3. Construct an `Orchestrator` with both agents (no semantic router).
4. Build an `AgentRequest` with `context: Some(json!({"target_agent": "agent-a"}))` so routing sends the initial request to agent-a.
5. Call `orchestrator.dispatch(request).await` and assert the result is `Err`.
6. Pattern-match on `OrchestratorError::EscalationFailed { chain, reason }` and assert:
   - `reason` contains the substring `"cycle detected"` (matching the format string in `validate_no_cycle`).
   - `chain` contains `"agent-a"` (the originator that was already visited when agent-b tries to escalate back).
7. Panic with a descriptive message on any other error variant.

## Implementation Details
- Use the same request-id pattern as other tests: `let request_id = uuid::Uuid::new_v4();`.
- Both agents should be `HealthStatus::Healthy` so the health check does not short-circuit.
- Place the new test in the "Tests" section of the file, after `dispatch_returns_escalation_failed_on_depth` and before `register_adds_dispatchable_agent`, to keep escalation-related tests grouped together.
- No new helpers, dependencies, or modules are needed.

### Suggested test body (reference, not prescriptive):
```rust
#[tokio::test]
async fn dispatch_returns_escalation_failed_on_cycle() {
    let request_id = uuid::Uuid::new_v4();

    // Agent A escalates to agent-b
    let agent_a = create_mock_endpoint(
        "agent-a",
        "alpha agent",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-b"),
    )
    .await;

    // Agent B escalates back to agent-a, creating a cycle
    let agent_b = create_mock_endpoint(
        "agent-b",
        "beta agent",
        HealthStatus::Healthy,
        build_escalation_response(request_id, "agent-a"),
    )
    .await;

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b], None);

    let request = AgentRequest {
        id: request_id,
        input: "trigger cycle".to_string(),
        context: Some(json!({"target_agent": "agent-a"})),
        caller: None,
    };

    let result = orchestrator.dispatch(request).await;
    assert!(result.is_err());
    match result.unwrap_err() {
        OrchestratorError::EscalationFailed { chain, reason } => {
            assert!(
                reason.contains("cycle detected"),
                "reason was: {}",
                reason
            );
            assert!(
                chain.contains(&"agent-a".to_string()),
                "chain was: {:?}",
                chain
            );
        }
        other => panic!("expected EscalationFailed, got: {:?}", other),
    }
}
```

## Dependencies
- **Blocked by:** "Add structured tracing to escalation path" -- tracing instrumentation may change the escalation flow or add span fields that affect how the chain is built. This test should be implemented after that work lands to avoid conflicts.
- **No new crate dependencies** are required.

## Risks & Edge Cases
- **Reason string coupling:** The assertion checks for the substring `"cycle detected"`, which is coupled to the format string in `validate_no_cycle` (`"cycle detected: '{}' already in chain"`). If that message changes, the test will break. This is intentional -- it validates the user-facing error message.
- **Chain contents:** The escalation flow is: dispatch routes to agent-a (chain = `["agent-a"]`), agent-a escalates to agent-b, `validate_no_cycle` passes (agent-b not in chain), agent-b is invoked (chain becomes `["agent-a", "agent-b"]`), agent-b escalates to agent-a, `validate_no_cycle` fails because `"agent-a"` is already in the chain. The chain in the error will be `["agent-a", "agent-b"]`.
- **No flakiness risk:** The mock servers are deterministic and always return the same escalation response, so there is no timing sensitivity.

## Verification
1. `cargo test -p orchestrator dispatch_returns_escalation_failed_on_cycle` -- the new test passes.
2. `cargo test -p orchestrator` -- all existing tests continue to pass.
3. `cargo clippy -p orchestrator --tests` -- no new warnings.
