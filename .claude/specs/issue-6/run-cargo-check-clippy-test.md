# Spec: Run `cargo check`, `cargo clippy`, `cargo test`

> From: .claude/tasks/issue-6.md

## Objective

Run the full verification suite defined in CLAUDE.md (`cargo check`, `cargo clippy`, `cargo test`) across the entire workspace to confirm that all changes from issue-6 (startup-time skill validation) compile cleanly, produce no clippy warnings, and pass all tests. This is the final gate before the issue can be considered complete.

## Current State

The workspace contains five crates defined in the root `Cargo.toml`:

| Crate | Path | Current State |
|---|---|---|
| `agent-sdk` | `crates/agent-sdk` | Defines `SkillManifest`, `Constraints`, `ModelConfig`, `OutputSchema`, `AgentError`, `MicroAgent`, and envelope types. Has three test files under `tests/`. |
| `skill-loader` | `crates/skill-loader` | Placeholder (`add` function only). Will contain validation logic, `ToolExists` trait, and `SkillLoader::load()` integration after preceding tasks. |
| `tool-registry` | `crates/tool-registry` | Placeholder (`add` function only). No changes expected from issue-6. |
| `agent-runtime` | `crates/agent-runtime` | Placeholder (`main` with println). No changes expected from issue-6. |
| `orchestrator` | `crates/orchestrator` | Placeholder (`main` with println). No changes expected from issue-6. |

Key types that will have been modified by preceding tasks:
- `Constraints.escalate_to` changed from `String` to `Option<String>` with serde attributes
- `OutputSchema` gains `ALLOWED_OUTPUT_FORMATS` constant
- `skill-loader` gains `ToolExists` trait, `AllToolsExist` stub, `validate()` function, `SkillError` type, and `SkillLoader` with load-time validation
- Test files in both `agent-sdk/tests/` (updated fixtures) and `skill-loader/tests/` (new validation tests)

## Requirements

- `cargo check` succeeds with exit code 0 for all five workspace crates, confirming type-correctness across the entire dependency graph.
- `cargo clippy` succeeds with exit code 0 and produces zero warnings, confirming idiomatic Rust style.
- `cargo test` succeeds with exit code 0 and all tests pass, including:
  - Existing `agent-sdk` tests (updated for `Option<String>` on `escalate_to`)
  - New validation unit tests in `skill-loader`
  - New integration test for load-with-validation in `skill-loader`
- No `#[allow(dead_code)]`, `#[allow(unused)]`, or similar suppressions were introduced solely to pass clippy.
- No test is marked `#[ignore]` unless there is a documented reason.

## Implementation Details

This task involves no file creation or modification. It is a command-line-only verification step.

Commands to run (in order):

1. **`cargo check`** -- Type-checks all crates without producing binaries. Catches missing imports, type mismatches, and unresolved references introduced by the preceding tasks.

2. **`cargo clippy`** -- Runs the Rust linter on all crates. Must produce zero warnings. Common issues to watch for after the preceding tasks:
   - Unused imports from refactored modules
   - Needless `clone()` or `to_string()` calls
   - Missing `#[must_use]` on public functions returning `Result`
   - `match` arms that could be simplified with `if let`

3. **`cargo test`** -- Runs all unit and integration tests across the workspace. Validates both the happy-path and error-path behaviors of the new validation logic.

If any command fails, the preceding tasks that introduced the failure must be fixed before this task can pass. This task does not fix code itself; it only confirms correctness.

## Dependencies

- Blocked by: All preceding tasks in issue-6:
  - "Change `escalate_to` from `String` to `Option<String>` in `Constraints`"
  - "Define allowed output format constants"
  - "Define `ToolExists` trait in skill-loader"
  - "Implement `validate` function"
  - "Integrate validation into `SkillLoader::load()`"
  - "Write validation unit tests"
  - "Write integration test for load-with-validation"
- Blocking: None (this is the final task in issue-6)

## Risks & Edge Cases

- **Cross-crate breakage from `Option<String>` change:** The `escalate_to` type change in `agent-sdk` could break any crate that depends on `agent-sdk` and accesses `Constraints.escalate_to` as a bare `String`. Currently only `skill-loader` has a potential dependency, but all crates should be checked.
- **Edition 2024 surprises:** The crates use `edition = "2024"`. Clippy rules may differ from edition 2021. Verify that clippy does not flag new edition-specific patterns.
- **Flaky tests from floating-point comparisons:** Existing tests use `f64::EPSILON` comparisons for `temperature` and `confidence_threshold`. These are stable for the values used but could become an issue if new test values are poorly chosen.
- **Missing dev-dependencies in skill-loader:** The new `skill-loader` tests may require `serde_yaml`, `agent-sdk`, or `tempfile` as dev-dependencies. If a preceding task forgot to add them, `cargo test` will fail at compile time.
- **Test isolation:** Validation tests that create temp directories must clean up properly. Using `tempfile::TempDir` (which auto-deletes on drop) is the expected pattern.

## Verification

1. Run `cargo check` and confirm exit code 0 with no errors.
2. Run `cargo clippy` and confirm exit code 0 with no warnings in the output.
3. Run `cargo test` and confirm exit code 0 with all tests passing (look for `test result: ok` with 0 failures).
4. Confirm no tests are `#[ignore]`-d by checking that the "ignored" count in the test summary is 0 (or that any ignored tests have documented reasons).
