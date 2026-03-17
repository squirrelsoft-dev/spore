# Spec: Run verification suite

> From: .claude/tasks/issue-5.md

## Objective
Run the full workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that all skill-loader implementation and test code compiles cleanly, produces no warnings, and all tests pass. This is the final gate task for issue-5 -- it validates that every preceding task (dependency setup, error types, frontmatter parsing, SkillLoader struct, unit tests, and integration tests) integrates correctly across the workspace.

## Current State
The workspace contains five crates: `agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, and `orchestrator` (defined in the root `Cargo.toml`). The `skill-loader` crate currently has only a placeholder `add()` function and no real dependencies. By the time this task runs, the preceding tasks will have:
- Added dependencies (`serde`, `serde_yaml`, `agent-sdk`, `tool-registry`, `tokio`, `tempfile`) to `crates/skill-loader/Cargo.toml`
- Created `crates/skill-loader/src/error.rs` with the `SkillError` enum
- Created `crates/skill-loader/src/frontmatter.rs` with `SkillFrontmatter` struct and `extract_frontmatter()` function
- Replaced the placeholder in `crates/skill-loader/src/lib.rs` with the `SkillLoader` struct and `load()` method
- Added unit tests in `crates/skill-loader/src/frontmatter.rs` (`#[cfg(test)] mod tests`)
- Created `crates/skill-loader/tests/skill_loader_test.rs` with integration tests

## Requirements
- `cargo check` succeeds across the entire workspace with zero errors
- `cargo clippy` succeeds across the entire workspace with zero warnings (no `#[allow(...)]` suppressions added to silence legitimate warnings)
- `cargo test` succeeds across the entire workspace with all tests passing, including:
  - Existing tests in `agent-sdk`, `tool-registry`, `agent-runtime`, and `orchestrator`
  - New unit tests in `crates/skill-loader/src/frontmatter.rs`
  - New integration tests in `crates/skill-loader/tests/skill_loader_test.rs`
- No commented-out code or debug statements remain in skill-loader source files
- No unused imports, dead code, or other Clippy lint violations in the skill-loader crate

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo check`** from the workspace root (`/workspaces/spore`). This performs type-checking across all workspace members. If it fails, diagnose the root cause (likely a type mismatch, missing import, or dependency issue in skill-loader) and report exactly which file and line needs correction.

2. **Run `cargo clippy`** from the workspace root. This applies Rust's standard lints plus Clippy's extended checks. Pay attention to:
   - Unused imports or variables in skill-loader modules
   - Redundant clones or unnecessary allocations
   - Missing `pub` visibility issues
   - Any warnings in the test modules

3. **Run `cargo test`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions. Verify:
   - All frontmatter extraction unit tests pass (valid frontmatter, empty body, missing delimiters, body with `---` rules, leading whitespace)
   - All SkillLoader integration tests pass (valid load, IoError, ParseError for malformed YAML, ParseError for missing delimiters, empty body, markdown with complex content)
   - All pre-existing tests in other crates still pass (no regressions)

4. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the skill-loader crate unless a workspace-level issue is discovered.

### Files potentially touched (fixes only, if needed)
- `crates/skill-loader/Cargo.toml` -- dependency version or feature adjustments
- `crates/skill-loader/src/lib.rs` -- import fixes, visibility fixes
- `crates/skill-loader/src/error.rs` -- Display/Error impl corrections
- `crates/skill-loader/src/frontmatter.rs` -- parsing logic or test corrections
- `crates/skill-loader/tests/skill_loader_test.rs` -- test fixture or assertion corrections

## Dependencies
- Blocked by: "Write frontmatter extraction unit tests", "Write integration tests for SkillLoader"
- Blocking: None (this is the final task for issue-5)

## Risks & Edge Cases
- **Cross-crate breakage**: Changes to `agent-sdk` types (e.g., `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`) between task completion and verification could cause compile failures in skill-loader. Mitigation: run `cargo check` first to catch type mismatches before running the full test suite.
- **Clippy false positives or edition-specific lints**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Mitigation: address each lint individually rather than blanket-suppressing with `#[allow]`.
- **Async runtime conflicts**: Integration tests use `#[tokio::test]` and the dev-dependency on `tokio` must have both `macros` and `rt` features. If features are missing, tests will fail to compile. Mitigation: verify `Cargo.toml` dev-dependency features before running tests.
- **Flaky tests from filesystem operations**: Integration tests use `tempfile::tempdir()` for fixture files. On rare occasions, temp directory cleanup can race with test assertions. Mitigation: ensure each test creates its own isolated temp directory.
- **Regressions in other crates**: The verification runs workspace-wide, so a failing test in `agent-sdk` or another crate would block this task even though it is unrelated. Mitigation: if a pre-existing test fails, confirm it also fails on main before attributing it to skill-loader changes.

## Verification
- `cargo check` exits with code 0 and produces no error output
- `cargo clippy` exits with code 0 and produces no warning output
- `cargo test` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- The above three commands are run from the workspace root `/workspaces/spore` without any `--package` filter, confirming workspace-wide health
