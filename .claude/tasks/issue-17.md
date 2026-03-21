# Task Breakdown: Implement escalation handling

> Add structured tracing to the existing escalation logic and expand test coverage for cycle detection, missing targets, no-target escalation, and multi-hop chains.

## Group 1 — Escalation observability logging

_Tasks in this group can be done in parallel._

- [x] **Add structured tracing to escalation path** `[S]`
      Add `tracing::info!` calls inside `handle_escalation()` in `orchestrator.rs` to log each escalation event with structured fields: `source_agent`, `target_agent`, `confidence`, `depth`, and the escalation chain. Also add `tracing::warn!` when `escalated=true` but `escalate_to` is `None` (the code currently returns the response silently). Add `tracing::error!` in the cycle-detection and depth-exceeded error paths in `validate_escalation_depth()` and `validate_no_cycle()`. Follow the pattern already established in `semantic_router.rs` (e.g., `tracing::debug!(agent = %agent_name, "routed via intent match")`).
      Files: `crates/orchestrator/src/orchestrator.rs`
      Blocking: "Add cycle detection test", "Add missing escalation target test", "Add escalated-with-no-target test", "Add successful multi-hop escalation chain test", "Add escalation-via-semantic-routing test"

## Group 2 — Escalation edge-case tests

_Depends on: Group 1._

- [x] **Add cycle detection test** `[S]`
      Add a test `dispatch_returns_escalation_failed_on_cycle` that creates two mock agents where A escalates to B and B escalates to A. Assert that `dispatch()` returns `OrchestratorError::EscalationFailed` with a reason containing "cycle detected". Follow the existing mock-server pattern in `orchestrator_test.rs` using `create_mock_endpoint` and `build_escalation_response`.
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Add structured tracing to escalation path"
      Non-blocking

- [x] **Add missing escalation target test** `[S]`
      Add a test `dispatch_returns_escalation_failed_on_missing_target` where agent A escalates to "nonexistent-agent" which is not in the registry. Assert that `dispatch()` returns `OrchestratorError::EscalationFailed` with a reason containing "not found in registry".
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Add structured tracing to escalation path"
      Non-blocking

- [x] **Add escalated-with-no-target test** `[S]`
      Add a test `dispatch_returns_response_when_escalated_without_target` where agent A returns `escalated: true` but `escalate_to: None`. Assert that `dispatch()` returns `Ok(response)` with the original response (the code gracefully handles this case by returning the response as-is).
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Add structured tracing to escalation path"
      Non-blocking

- [x] **Add successful multi-hop escalation chain test** `[S]`
      Add a test `dispatch_handles_multi_hop_escalation` that creates three mock agents: A escalates to B, B escalates to C, C returns success. Assert that `dispatch()` returns the successful response from C. This validates the full chain-following loop, not just single-hop.
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Add structured tracing to escalation path"
      Non-blocking

- [x] **Add escalation-via-semantic-routing test** `[S]`
      Add a test `dispatch_with_model_handles_escalation` that uses `dispatch_with_model()` with the `MockEmbeddingModel` and `SemanticRouter` to route to an agent that escalates. Verifies that the escalation path works identically whether dispatch was initiated by context-based or semantic routing.
      Files: `crates/orchestrator/tests/orchestrator_test.rs`
      Blocked by: "Add structured tracing to escalation path"
      Non-blocking

## Group 3 — Verification

_Depends on: Group 2._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify no regressions in existing crates. Verify all new and existing orchestrator tests pass.
      Files: (none — command-line verification only)
      Blocked by: All other tasks
