# Spec: Add missing escalation target test
> From: .claude/tasks/issue-17.md

## Objective
Add a test `dispatch_returns_escalation_failed_on_missing_target` that verifies the orchestrator returns `OrchestratorError::EscalationFailed` when an agent escalates to a target that does not exist in the registry.

## Current State
The test file `crates/orchestrator/tests/orchestrator_test.rs` already covers several escalation scenarios:
- `dispatch_handles_escalation` -- successful escalation from agent-a to agent-b (line 289)
- `dispatch_returns_escalation_failed_on_depth` -- escalation chain exceeding `MAX_ESCALATION_DEPTH` (line 324)

The missing coverage is the case where `escalate_to` names an agent that is not registered. The `lookup_escalation_target` method in `crates/orchestrator/src/orchestrator.rs` (line 245) handles this by returning `OrchestratorError::EscalationFailed` with a reason of `"escalation target '<name>' not found in registry"`, but no test exercises this path.

## Requirements
1. Add a single `#[tokio::test]` named `dispatch_returns_escalation_failed_on_missing_target`.
2. Register one agent (agent-a) that returns an escalation response pointing to `"nonexistent-agent"`.
3. Do **not** register any agent named `"nonexistent-agent"` in the orchestrator.
4. Dispatch a request targeting agent-a via `context.target_agent`.
5. Assert the result is `Err(OrchestratorError::EscalationFailed { chain, reason })`.
6. Assert `reason` contains `"not found in registry"`.
7. Assert `chain` contains `"agent-a"` (the agent that initiated the escalation).

## Implementation Details
- Place the test in the `// Tests` section of `crates/orchestrator/tests/orchestrator_test.rs`, after the existing `dispatch_returns_escalation_failed_on_depth` test (after line 380).
- Use the existing helpers:
  - `create_mock_endpoint` to create agent-a with `HealthStatus::Healthy` and a response from `build_escalation_response(request_id, "nonexistent-agent")`.
  - `build_test_manifest()` for the orchestrator manifest.
- Construct the orchestrator with `Orchestrator::new(build_test_manifest(), vec![agent_a], None)` -- only agent-a in the registry, no `"nonexistent-agent"`.
- Build the request with `context: Some(json!({"target_agent": "agent-a"}))` so routing succeeds and reaches agent-a.
- Match on the error variant using the same pattern as `dispatch_returns_escalation_failed_on_depth`:
  ```rust
  match result.unwrap_err() {
      OrchestratorError::EscalationFailed { chain, reason } => {
          assert!(reason.contains("not found in registry"), "reason was: {}", reason);
          assert!(chain.contains(&"agent-a".to_string()), "chain was: {:?}", chain);
      }
      other => panic!("expected EscalationFailed, got: {:?}", other),
  }
  ```
- No new imports, helpers, or dependencies are needed.

## Dependencies
- **Blocked by**: "Add structured tracing to escalation path" -- that task may modify the escalation code path. The test itself does not depend on tracing, but if the escalation logic or error format changes, this test must be updated to match.

## Risks & Edge Cases
- If the `lookup_escalation_target` error message wording changes (currently `"escalation target '{}' not found in registry"`), the substring assertion `"not found in registry"` will break. This is an acceptable coupling since the test is specifically validating that error message.
- The `chain` vector at the point of failure contains only `["agent-a"]` because the target agent is looked up before being appended to the chain (see `handle_escalation` at line 211 in orchestrator.rs where `current_chain.push(target_name)` happens after `lookup_escalation_target`). The assertion should reflect this.

## Verification
1. `cargo test -p orchestrator dispatch_returns_escalation_failed_on_missing_target` -- the new test passes.
2. `cargo test -p orchestrator` -- all existing orchestrator tests continue to pass.
3. `cargo clippy -p orchestrator --tests` -- no new warnings.
