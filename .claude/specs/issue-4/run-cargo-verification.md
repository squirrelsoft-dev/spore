# Spec: Run `cargo check`, `cargo clippy`, `cargo test` to verify

> From: .claude/tasks/issue-4.md

## Objective

Run the full verification suite to confirm that all code introduced in issue #4 (envelope types, supporting types, module wiring, and tests) compiles cleanly, passes linting, and all tests succeed. This is the final gate before the issue can be considered complete.

## Current State

The `agent-sdk` crate (`crates/agent-sdk/`) currently contains four modules from issue #2:
- `constraints.rs`, `model_config.rs`, `output_schema.rs`, `skill_manifest.rs`
- Re-exported from `lib.rs`
- One integration test file: `tests/skill_manifest_test.rs` (4 tests)
- Dependencies: `serde` (with `derive`), `schemars` (with `derive`); dev-dep: `serde_yaml`

By the time this task runs, the following will have been added by earlier tasks in issue #4:
- **Dependencies**: `uuid` (with `v4`, `serde` features) and `serde_json` in `[dependencies]`; `serde_json` also in `[dev-dependencies]`
- **New source files**: `agent_request.rs`, `agent_response.rs`, `agent_error.rs`, `health_status.rs`, `tool_call_record.rs`
- **Updated `lib.rs`**: `mod` declarations and `pub use` re-exports for all five new modules
- **New test file**: `tests/envelope_types_test.rs` covering construction, serialization round-trips, Display impls, and nested JSON values

## Requirements

1. `cargo check` must complete with exit code 0 and produce no errors for the `agent-sdk` crate.
2. `cargo clippy` must complete with exit code 0 and produce zero warnings across all source files (including new modules and test files). This means no `#[allow(...)]` attributes should be needed.
3. `cargo test` must complete with exit code 0. Specifically:
   - All 4 existing tests in `tests/skill_manifest_test.rs` must pass.
   - All new tests in `tests/envelope_types_test.rs` must pass (expected: at least 7 tests covering `AgentRequest::new()` construction, `AgentRequest` JSON round-trip, `AgentResponse::success()` construction, `AgentResponse` JSON round-trip with tool calls, `AgentError` Display output, `HealthStatus` serde round-trip, `ToolCallRecord` serde round-trip).
4. No compilation warnings (not just errors) from `cargo check` or `cargo build`.

## Implementation Details

This task involves no file creation or modification. It is purely a command-line verification step.

Commands to run in order:
1. `cargo check` -- confirms the crate and all dependencies compile without errors or warnings
2. `cargo clippy` -- confirms no lint warnings; use `cargo clippy -- -D warnings` to promote warnings to errors for a strict check
3. `cargo test` -- runs all unit and integration tests

All commands should be run from the workspace root (`/workspaces/spore/`) so they pick up the workspace-level `Cargo.toml` if one exists, or from `crates/agent-sdk/` if the project is a standalone crate.

If any command fails:
- Read the error output carefully.
- Diagnose before fixing: identify which file and line caused the issue.
- Fix the root cause in the relevant source or test file (from a prior task).
- Re-run the full suite from the beginning to confirm the fix did not introduce regressions.

## Dependencies

- **Blocked by**: "Write serialization and construction tests" (all code and tests must exist before verification)
- **Blocking**: None (this is the terminal task for issue #4)

## Risks & Edge Cases

- **Clippy version differences**: The CI environment and local toolchain may run different clippy versions. If a new clippy lint fires that did not exist when earlier tasks were written, it must be addressed (fix the code, do not suppress with `#[allow]`).
- **Feature-gated warnings**: `uuid`'s `v4` feature requires the `getrandom` crate. On exotic targets this could fail, but standard Linux/macOS/Windows are fine.
- **Test ordering**: Cargo runs tests in arbitrary order. Tests must not depend on shared mutable state or execution order.
- **Workspace vs. crate scope**: If the workspace has other crates that fail independently, scope commands with `-p agent-sdk` (e.g., `cargo test -p agent-sdk`) to isolate this crate's results.
- **Stale build artifacts**: If prior tasks were done in a worktree, ensure `cargo clean` is not needed. Incremental compilation should handle this, but a clean build can be used as a fallback if phantom errors appear.

## Verification

This task *is* the verification step for the entire issue. It is confirmed done when:
1. `cargo check` exits 0 with no warnings or errors.
2. `cargo clippy -- -D warnings` exits 0 with no warnings or errors.
3. `cargo test` exits 0, and the output shows all tests passing (0 failures, 0 ignored unless intentionally marked).
4. The combined test count includes both `skill_manifest_test` and `envelope_types_test` suites.
