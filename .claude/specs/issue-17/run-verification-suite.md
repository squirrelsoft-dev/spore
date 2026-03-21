# Spec: Run verification suite

> From: .claude/tasks/issue-17.md

## Objective
Run the full verification suite to confirm all changes compile, pass linting, and all tests (new and existing) pass without regressions.

## Current State
The workspace contains multiple crates: agent-sdk, agent-runtime, orchestrator, skill-loader, tool-registry. All currently pass cargo check, cargo clippy, and cargo test.

## Requirements
1. `cargo check` must succeed with no errors across the full workspace
2. `cargo clippy` must produce no new warnings
3. `cargo test` must pass all tests including new escalation tests
4. No regressions in existing crates

## Implementation Details
This is a verification-only task. No code changes. Run:
- `cargo check`
- `cargo clippy`
- `cargo test`

## Dependencies
- Blocked by: All other tasks in issue-17 (structured tracing, cycle detection test, missing target test, escalated-with-no-target test, multi-hop escalation test, escalation-via-semantic-routing test)
- Blocking: Nothing

## Risks & Edge Cases
- Tracing additions could introduce unused import warnings if not done carefully
- New tests could have flaky behavior with mock HTTP servers on random ports
- Adding `tracing` as a dependency to the orchestrator crate could cause version conflicts with other crates in the workspace

## Verification
All three commands pass with zero errors and zero warnings.
