# Spec: Add successful multi-hop escalation chain test
> From: .claude/tasks/issue-17.md

## Objective
Add a test `dispatch_handles_multi_hop_escalation` to validate that the orchestrator's escalation loop correctly follows a chain of multiple hops (A -> B -> C) and returns the final successful response from agent C. The existing `dispatch_handles_escalation` test only covers a single-hop escalation (A -> B); this test ensures the full loop logic works for chains longer than one hop.

## Current State
- `crates/orchestrator/tests/orchestrator_test.rs` contains a single-hop escalation test (`dispatch_handles_escalation`) that creates two agents: A escalates to B, B returns success. This validates only the first iteration of the escalation loop.
- `crates/orchestrator/tests/orchestrator_test.rs` also contains a max-depth test (`dispatch_returns_escalation_failed_on_depth`) that creates a chain of 7 agents all escalating, but it tests the failure path, not a successful multi-hop chain.
- The `handle_escalation` method in `crates/orchestrator/src/orchestrator.rs` (line 180) uses a `loop` that iterates through escalations until either: (a) a non-escalated response is returned, (b) max depth is exceeded, or (c) a cycle is detected. The multi-hop success path (iterating the loop more than once and then returning `Ok`) has no dedicated test coverage.

## Requirements
1. Create a test function named `dispatch_handles_multi_hop_escalation` in `crates/orchestrator/tests/orchestrator_test.rs`.
2. Create three mock agents:
   - **agent-a**: healthy, returns an escalation response targeting `agent-b`.
   - **agent-b**: healthy, returns an escalation response targeting `agent-c`.
   - **agent-c**: healthy, returns a successful response (e.g., output `"handled-by-c"`).
3. Construct an `Orchestrator` with all three agents registered (no semantic router needed).
4. Build an `AgentRequest` with `context: {"target_agent": "agent-a"}` to initiate dispatch at agent-a.
5. Call `orchestrator.dispatch(request).await` and assert:
   - The result is `Ok`.
   - The response output matches agent-c's success output (`{"result": "handled-by-c"}`).
   - The response `escalated` field is `false` (agent-c did not escalate).

## Implementation Details
- Follow the established test patterns exactly: use `create_mock_endpoint`, `build_escalation_response`, `build_success_response`, `build_test_manifest`, and `Orchestrator::new`.
- Use a single shared `request_id` (`uuid::Uuid::new_v4()`) for all agents and the request, matching the pattern in existing tests.
- Place the test in the `// Tests` section of the file, after `dispatch_handles_escalation` and before `dispatch_returns_escalation_failed_on_depth` for logical grouping.
- The test is `#[tokio::test] async fn`.
- No new helper functions, imports, or dependencies are needed; all required utilities already exist.

### Test structure (pseudocode):
```
#[tokio::test]
async fn dispatch_handles_multi_hop_escalation() {
    let request_id = uuid::Uuid::new_v4();

    // agent-a escalates to agent-b
    let agent_a = create_mock_endpoint("agent-a", ..., build_escalation_response(request_id, "agent-b"));

    // agent-b escalates to agent-c
    let agent_b = create_mock_endpoint("agent-b", ..., build_escalation_response(request_id, "agent-c"));

    // agent-c returns success
    let agent_c = create_mock_endpoint("agent-c", ..., build_success_response(request_id, "handled-by-c"));

    let orchestrator = Orchestrator::new(build_test_manifest(), vec![agent_a, agent_b, agent_c], None);

    let request = AgentRequest { ..., context: Some(json!({"target_agent": "agent-a"})), ... };

    let response = orchestrator.dispatch(request).await.unwrap();
    assert_eq!(response.output, json!({"result": "handled-by-c"}));
    assert!(!response.escalated);
}
```

## Dependencies
- **Blocked by**: "Add structured tracing to escalation path" -- the tracing task may modify the `handle_escalation` method signature or add tracing instrumentation. This test should be implemented after that work is merged to avoid conflicts and to verify the tracing changes do not break multi-hop behavior.
- **No new crate dependencies** are required.

## Risks & Edge Cases
- **Mock server statefulness**: Each mock agent always returns the same fixed response regardless of request content. This is fine for this test because the escalation loop replays the request with a new `target_agent` context, and routing is done by the orchestrator registry, not by the mock servers.
- **Request ID consistency**: The same `request_id` is used across all agents and the request. This is consistent with the existing test pattern and works because the mock servers ignore the request body details.
- **Chain length is well under MAX_ESCALATION_DEPTH (5)**: A 3-agent chain (A -> B -> C) produces a chain vector of length 3, which is safely below the limit. No depth-limit error should occur.

## Verification
1. Run `cargo test dispatch_handles_multi_hop_escalation` -- the new test must pass.
2. Run `cargo test --package orchestrator` -- all existing orchestrator tests must continue to pass.
3. Run `cargo clippy --package orchestrator --tests` -- no new warnings.
