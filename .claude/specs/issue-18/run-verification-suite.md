# Spec: Run verification suite

> From: .claude/tasks/issue-18.md

## Objective
Run the full verification suite to confirm all changes compile, pass linting, and all tests (new and existing) pass without regressions.

## Current State
The workspace contains multiple crates: agent-sdk, agent-runtime, orchestrator, skill-loader, tool-registry. All currently pass cargo check, cargo clippy, and cargo test.

## Requirements
1. `cargo check` must succeed with no errors across the full workspace
2. `cargo clippy` must produce no new warnings
3. `cargo test` must pass all tests including the new orchestrator skill integration test
4. No regressions in existing crates

## Implementation Details
This is a verification-only task. No code changes. Run:
- `cargo check`
- `cargo clippy`
- `cargo test`

## Dependencies
- Blocked by: All other tasks in issue-18
- Blocking: Nothing

## Risks & Edge Cases
- New skill file could fail validation if frontmatter format is incorrect
- Orchestrator wiring changes could break existing tests if build_default_manifest removal is not handled carefully

## Verification
All three commands pass with zero errors and zero warnings.
