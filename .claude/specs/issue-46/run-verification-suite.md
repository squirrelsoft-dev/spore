# Spec: Run verification suite

> From: .claude/tasks/issue-46.md

## Objective
Run the full verification suite for the `validate-skill` crate and the workspace as a whole to confirm that the new tool compiles cleanly, passes all tests, produces no lint warnings, and integrates correctly with the existing workspace. All five acceptance criteria from the issue must pass. This is the final gate task for issue-46.

## Current State
The workspace is defined in the root `Cargo.toml` with `resolver = "2"`. By the time this task runs, all preceding issue-46 tasks will have:
1. Exposed a public `parse_content` function in `crates/skill-loader/src/lib.rs`
2. Created `tools/validate-skill/Cargo.toml` with dependencies on `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`, `skill-loader`, and `agent-sdk`
3. Added `"tools/validate-skill"` to the workspace `members` array in the root `Cargo.toml`
4. Implemented `ValidateSkillTool` struct and handler in `tools/validate-skill/src/validate_skill.rs`
5. Created `tools/validate-skill/src/main.rs` entrypoint
6. Written unit tests in `tools/validate-skill/src/validate_skill.rs` (valid content, missing frontmatter, invalid YAML, validation failures)
7. Written integration tests in `tools/validate-skill/tests/validate_skill_server_test.rs` (tools list, description, content parameter, valid call, missing frontmatter call, invalid YAML call)
8. Written `tools/validate-skill/README.md`

## Requirements
- `cargo build -p validate-skill` succeeds with zero errors
- `cargo test -p validate-skill` succeeds with all test cases passing and zero failures
- `cargo clippy -p validate-skill` succeeds with zero warnings
- `cargo check` (full workspace) succeeds with zero errors
- No commented-out code or debug statements remain in the `validate-skill` source files
- No unused imports, dead code, or other Clippy lint violations in the `validate-skill` crate

### Issue Acceptance Criteria
All five acceptance criteria from the issue must be verified:
1. **Build succeeds**: `cargo build -p validate-skill` exits with code 0
2. **Tests pass**: `cargo test -p validate-skill` exits with code 0 with all tests reporting `ok`
3. **Valid skill files return `{valid: true, errors: [], manifest: {...}}`**: The integration test calling the tool with well-formed skill file content receives a response containing `"valid": true`, an empty `errors` array, and a populated `manifest` object
4. **Malformed skill files return `{valid: false, errors: ["..."]}`**: The integration tests calling the tool with missing frontmatter or invalid YAML receive responses containing `"valid": false` and a non-empty `errors` array
5. **Tool is named `validate_skill` in `tools/list`**: The integration test listing tools confirms the tool name is `validate_skill`

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo build -p validate-skill`** from the workspace root (`/workspaces/spore`). This compiles only the `validate-skill` crate and its dependencies (including `skill-loader` and `agent-sdk`). If it fails, diagnose the root cause -- likely candidates include:
   - Missing or incorrect dependency features in `tools/validate-skill/Cargo.toml`
   - Import errors in `validate_skill.rs` or `main.rs` (e.g., wrong `rmcp` module paths, wrong `skill_loader` function signatures)
   - Type mismatches in the `ServerHandler` or `#[tool_router]` implementations
   - The `tools/validate-skill` path not present in the root `Cargo.toml` `members` array
   - `parse_content` not exported as `pub` from `skill-loader`

2. **Run `cargo test -p validate-skill`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions within the `validate-skill` crate. Verify:
   - Unit tests: valid skill content returns `valid: true` with parsed manifest fields; missing frontmatter returns `valid: false`; invalid YAML returns `valid: false`; validation failures (empty name, bad confidence threshold, etc.) return `valid: false` with specific error messages
   - Integration tests: `tools_list_returns_validate_skill_tool` passes (tool count and name); `tools_list_has_correct_description` passes (description contains "Validate"); `tools_list_has_content_parameter` passes (input schema contains "content"); `tools_call_with_valid_skill_returns_valid_true` passes (structured JSON with `valid: true` and `manifest`); `tools_call_with_missing_frontmatter_returns_valid_false` passes; `tools_call_with_invalid_yaml_returns_valid_false` passes

3. **Run `cargo clippy -p validate-skill -- -D warnings`** from the workspace root. The `-D warnings` flag treats warnings as errors, ensuring a clean lint pass. Pay attention to:
   - Unused imports or variables in `validate_skill.rs` and `main.rs`
   - Clippy suggestions for serialization patterns (`serde_json::json!`, `serde_json::to_string`)
   - Warnings in the test module
   - Any `#[allow(...)]` suppressions that were added solely to silence legitimate warnings (these should be removed)

4. **Run `cargo check`** (full workspace, no `--package` filter) from the workspace root. This verifies that the new `validate-skill` crate and the changes to `skill-loader` (adding `parse_content`) do not break type-checking for any other workspace member. If it fails on a crate other than `validate-skill`, confirm the failure also exists on `main` before attributing it to issue-46 changes.

5. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `validate-skill` crate or `crates/skill-loader/src/lib.rs` unless a workspace-level issue is discovered.

### Exact Commands
All commands run from `/workspaces/spore`:
```bash
cargo build -p validate-skill
cargo test -p validate-skill
cargo clippy -p validate-skill -- -D warnings
cargo check
```

### Files potentially touched (fixes only, if needed)
- `tools/validate-skill/Cargo.toml` -- dependency version or feature adjustments
- `tools/validate-skill/src/main.rs` -- import or initialization fixes
- `tools/validate-skill/src/validate_skill.rs` -- tool handler or test fixes
- `tools/validate-skill/tests/validate_skill_server_test.rs` -- integration test fixes
- `crates/skill-loader/src/lib.rs` -- `parse_content` signature or visibility fixes
- `Cargo.toml` -- workspace member path fix (unlikely)

## Dependencies
- Blocked by: "Expose a public `parse_content` function in skill-loader", "Create `tools/validate-skill/Cargo.toml`", "Add `tools/validate-skill` to workspace members", "Implement `ValidateSkillTool` struct and handler", "Write `main.rs`", "Write integration test", "Write README"
- Blocking: None (this is the final task for issue-46)

## Risks & Edge Cases
- **Async runtime conflicts**: The validate-skill tool uses `tokio` with specific features. If a feature mismatch exists between the binary target and test targets, compilation or runtime errors may occur. Mitigation: ensure `[dev-dependencies]` includes `tokio` with `macros` and `rt` features.
- **skill-loader API breakage**: The `parse_content` function is newly added to `skill-loader`. If its signature does not match what `validate_skill.rs` expects (e.g., different error type, different return type), the build will fail. Mitigation: confirm the function signature in `lib.rs` matches the call site in `validate_skill.rs`.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Mitigation: address each lint individually rather than blanket-suppressing with `#[allow]`.
- **Regressions in other crates**: The `cargo check` step runs workspace-wide, so a failing check in `agent-sdk`, `skill-loader`, `tool-registry`, `echo-tool`, `read-file`, or `write-file` would block this task even if unrelated to validate-skill. Mitigation: confirm the failure also exists on `main` before attributing it to issue-46 changes.
- **AllToolsExist behavior**: The tool uses `AllToolsExist` as the tool checker for structural validation. If `AllToolsExist` has been removed or renamed in `agent-sdk` or `skill-loader`, the build will fail. Mitigation: verify the import path before running the build.
- **Integration test subprocess spawning**: Integration tests spawn the `validate-skill` binary as a child process via `TokioChildProcess`. If the binary path from `env!("CARGO_BIN_EXE_validate-skill")` is incorrect or the binary was not built, tests will fail at spawn time rather than in assertions. Mitigation: ensure `cargo build -p validate-skill` passes before running tests.

## Verification (Pass/Fail Criteria)
- **PASS**: `cargo build -p validate-skill` exits with code 0 and produces no error output
- **PASS**: `cargo test -p validate-skill` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- **PASS**: `cargo clippy -p validate-skill -- -D warnings` exits with code 0 and produces no warning or error output
- **PASS**: `cargo check` (full workspace) exits with code 0 and produces no error output
- **PASS**: Integration tests confirm acceptance criteria 3, 4, and 5 (valid/invalid JSON responses and tool naming)
- **FAIL**: Any of the above commands exits with a non-zero code or produces error/warning output (after applying `-D warnings` to clippy)
