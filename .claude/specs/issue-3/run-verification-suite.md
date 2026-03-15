# Spec: Run `cargo check`, `cargo clippy`, `cargo test` to verify

> From: .claude/tasks/issue-3.md

## Objective

Run the full verification suite defined in CLAUDE.md against the `agent-sdk` crate (and the workspace as a whole) after all preceding issue-3 tasks are complete. This is the final gate that confirms the `MicroAgent` trait, all envelope types, and all tests compile cleanly, pass lint, and execute without failures. It also specifically verifies that `Box<dyn MicroAgent>` compiles -- the core dyn-compatibility requirement that motivated the `async-trait` approach.

This task produces no code changes. It is a command-line-only verification step that validates the work done in Groups 1-4.

## Current State

By the time this task runs, the following should already be in place (from prior groups):

- **Group 1**: `async-trait`, `uuid`, and `serde_json` dependencies added to `crates/agent-sdk/Cargo.toml`; `tokio` added as a dev-dependency.
- **Group 2**: `ToolCallRecord`, `AgentRequest`, `AgentResponse`, `AgentError`, and `HealthStatus` types defined in their respective modules under `crates/agent-sdk/src/`.
- **Group 3**: `MicroAgent` trait defined in `crates/agent-sdk/src/micro_agent.rs` using `#[async_trait]` with `Send + Sync` supertraits; `lib.rs` updated with all module declarations, `pub use` re-exports, and `async_trait` re-export.
- **Group 4 (tests)**: `crates/agent-sdk/tests/micro_agent_test.rs` (object-safety and mock-implementation tests) and `crates/agent-sdk/tests/envelope_types_test.rs` (serialization round-trip tests) written and ready to execute.

## Requirements

- `cargo check` (workspace-wide) must complete with zero errors.
- `cargo check -p agent-sdk` must complete with zero errors.
- `cargo clippy` (workspace-wide) must complete with zero warnings and zero errors.
- `cargo clippy -p agent-sdk` must complete with zero warnings and zero errors.
- `cargo test` (workspace-wide) must pass all tests with zero failures.
- `cargo test -p agent-sdk` must pass all tests with zero failures, including:
  - Existing `skill_manifest_test.rs` tests (regression check).
  - New `micro_agent_test.rs` tests (object-safety, mock implementation, `Box<dyn MicroAgent>` usage, `HealthStatus` variants, `invoke` Ok/Err paths).
  - New `envelope_types_test.rs` tests (serialization round-trips, `Display` on `AgentError`, `AgentRequest::new()` constructor).
- `Box<dyn MicroAgent>` must compile without errors, confirming the trait is object-safe. This is validated implicitly by the `micro_agent_test.rs` tests, but should also be confirmed explicitly in the verification output.

## Implementation Details

No files are created or modified. This task consists solely of running commands and interpreting their output.

**Commands to run (in order):**

1. `cargo check` -- Confirms all crates in the workspace compile, including the new types and trait. This catches syntax errors, missing imports, type mismatches, and unresolved dependencies.

2. `cargo clippy` -- Runs the Rust linter across the workspace. Catches common mistakes, style issues, and potential bugs. Must produce zero warnings. If warnings appear, they must be resolved before proceeding (though resolving them is out of scope for this task -- it would indicate a defect in a prior task).

3. `cargo test` -- Executes all unit and integration tests across the workspace. Confirms that all new tests pass and that no existing tests have regressed.

**Key verification points within the test output:**

- Look for `test result: ok` with `0 failed` in the test summary.
- Confirm that test binaries for `micro_agent_test` and `envelope_types_test` are compiled and executed.
- Confirm the test that boxes a `MockAgent` as `Box<dyn MicroAgent>` passes (this is the object-safety proof).

## Dependencies

- **Blocked by**:
  - "Write object-safety and mock-implementation tests" (Group 4)
  - "Write serialization tests for envelope types" (Group 4)
  - (Transitively blocked by all Groups 1-3)
- **Blocking**: Nothing -- this is the terminal task for issue-3.

## Risks & Edge Cases

- **Clippy false positives**: A new version of clippy could introduce warnings that did not exist when the code was written. Mitigation: if a genuine false positive is encountered, it can be suppressed with `#[allow(...)]` annotations on the specific item, but that would require a code change and a re-run. This is unlikely given the straightforward nature of the types.
- **Flaky tests**: If any tests use randomness (e.g., `Uuid::new_v4()` in `AgentRequest::new()`) they should not depend on specific UUID values. The tests from prior tasks should compare structural properties, not exact serialized strings. If a test is flaky, it indicates a defect in the test, not in this verification task.
- **Workspace-level compilation failures**: Changes in `agent-sdk` could theoretically break downstream crates that depend on it (if any exist by the time this runs). Running workspace-wide `cargo check` catches this. Currently no other crates depend on `agent-sdk`, so this risk is minimal.
- **Missing re-exports**: If `lib.rs` does not re-export all required types, the integration tests (which import from the crate root) will fail at compile time during `cargo test`. This would indicate a defect in the "Update `lib.rs`" task.

## Verification

This task is itself the verification step. It is confirmed complete when all three of the following are true:

1. `cargo check` exits with code 0 and produces no error output.
2. `cargo clippy` exits with code 0 and produces no warning or error output.
3. `cargo test` exits with code 0 and reports `0 failed` in every test summary line, with all expected test binaries (`micro_agent_test`, `envelope_types_test`, and existing tests) having been compiled and executed.

If any command fails, the failure output should be reported along with identification of which prior task likely introduced the defect, so it can be fixed before re-running verification.
