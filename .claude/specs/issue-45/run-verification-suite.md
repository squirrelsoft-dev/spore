# Spec: Run verification suite

> From: .claude/tasks/issue-45.md

## Objective
Run the full verification suite for the `write-file` crate and the workspace as a whole to confirm that the new tool compiles cleanly, passes all tests, produces no lint warnings, and integrates correctly with the existing workspace. This is the final gate task for issue-45.

## Current State
The workspace is defined in the root `Cargo.toml` with `resolver = "2"`. By the time this task runs, all preceding issue-45 tasks will have:
1. Created `tools/write-file/Cargo.toml` with dependencies matching echo-tool (`rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`)
2. Added `"tools/write-file"` to the workspace `members` array in the root `Cargo.toml`
3. Implemented `WriteFileTool` struct and handler in `tools/write-file/src/write_file.rs`
4. Created `tools/write-file/src/main.rs` entrypoint
5. Written unit tests in `tools/write-file/src/write_file.rs` (create file, create parent dirs, empty path, byte count, overwrite existing, unicode)
6. Written integration tests in `tools/write-file/tests/write_file_server_test.rs` (tools list, description, parameters, call creates file)
7. Written `tools/write-file/README.md`

## Requirements
- `cargo build -p write-file` succeeds with zero errors
- `cargo test -p write-file` succeeds with all test cases passing and zero failures
- `cargo clippy -p write-file` succeeds with zero warnings
- `cargo check` (full workspace) succeeds with zero errors
- No commented-out code or debug statements remain in the `write-file` source files
- No unused imports, dead code, or other Clippy lint violations in the `write-file` crate

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo build -p write-file`** from the workspace root (`/workspaces/spore`). This compiles only the `write-file` crate and its dependencies. If it fails, diagnose the root cause -- likely candidates include:
   - Missing or incorrect dependency features in `tools/write-file/Cargo.toml`
   - Import errors in `write_file.rs` or `main.rs` (e.g., wrong `rmcp` module paths)
   - Type mismatches in the `ServerHandler` or `#[tool_router]` implementations
   - The `tools/write-file` path not present in the root `Cargo.toml` `members` array

2. **Run `cargo test -p write-file`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions within the `write-file` crate. Verify:
   - `write_file_creates_file_with_content` passes (temp file with known content)
   - `write_file_creates_parent_directories` passes (nested path inside temp dir)
   - `write_file_empty_path` passes (descriptive error for empty string)
   - `write_file_returns_byte_count` passes (confirmation message contains correct count)
   - `write_file_overwrites_existing` passes (second write replaces first)
   - `write_file_preserves_unicode` passes (unicode round-trip)
   - Integration tests in `write_file_server_test.rs` pass (tools list, description, parameters, call creates file)

3. **Run `cargo clippy -p write-file -- -D warnings`** from the workspace root. The `-D warnings` flag treats warnings as errors, ensuring a clean lint pass. Pay attention to:
   - Unused imports or variables in `write_file.rs` and `main.rs`
   - Clippy suggestions for `std::fs` API usage patterns (e.g., `create_dir_all`, `write`)
   - Warnings in the test module
   - Any `#[allow(...)]` suppressions that were added solely to silence legitimate warnings (these should be removed)

4. **Run `cargo check`** (full workspace, no `--package` filter) from the workspace root. This verifies that the new `write-file` crate does not break type-checking for any other workspace member. If it fails on a crate other than `write-file`, confirm the failure also exists on `main` before attributing it to write-file changes.

5. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `write-file` crate unless a workspace-level issue is discovered.

### Exact Commands
All commands run from `/workspaces/spore`:
```bash
cargo build -p write-file
cargo test -p write-file
cargo clippy -p write-file -- -D warnings
cargo check
```

### Files potentially touched (fixes only, if needed)
- `tools/write-file/Cargo.toml` -- dependency version or feature adjustments
- `tools/write-file/src/main.rs` -- import or initialization fixes
- `tools/write-file/src/write_file.rs` -- tool handler or test fixes
- `tools/write-file/tests/write_file_server_test.rs` -- integration test fixes
- `Cargo.toml` -- workspace member path fix (unlikely)

## Dependencies
- Blocked by: "Create `tools/write-file/Cargo.toml`", "Add `tools/write-file` to workspace members", "Implement `WriteFileTool` struct and handler", "Create `main.rs` entrypoint", "Write unit tests", "Write integration tests", "Write README"
- Blocking: None (this is the final task for issue-45)

## Risks & Edge Cases
- **Async runtime conflicts**: The write-file tool uses `tokio` with specific features. If a feature mismatch exists between the binary target and test targets, compilation or runtime errors may occur. Mitigation: ensure `[dev-dependencies]` includes `tokio` with `macros` and `rt` features.
- **Temp file cleanup in tests**: Unit tests that create temp files and directories must clean up after themselves. If temp file creation fails (e.g., permissions), tests will fail for environmental reasons rather than code defects. Mitigation: use `std::env::temp_dir()` with unique subdirectories for isolation.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Mitigation: address each lint individually rather than blanket-suppressing with `#[allow]`.
- **Regressions in other crates**: The `cargo check` step runs workspace-wide, so a failing check in `agent-sdk`, `skill-loader`, `tool-registry`, `echo-tool`, or `read-file` would block this task even if unrelated to write-file. Mitigation: confirm the failure also exists on `main` before attributing it to write-file changes.
- **File system permissions**: The `write_file` tool writes to disk and creates directories, so tests that target restricted directories will fail. All tests should use temp directories with known-good permissions.
- **Disk space**: Writing large content in tests could theoretically fail on constrained environments. All test content should be small and deterministic.

## Verification (Pass/Fail Criteria)
- **PASS**: `cargo build -p write-file` exits with code 0 and produces no error output
- **PASS**: `cargo test -p write-file` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- **PASS**: `cargo clippy -p write-file -- -D warnings` exits with code 0 and produces no warning or error output
- **PASS**: `cargo check` (full workspace) exits with code 0 and produces no error output
- **FAIL**: Any of the above commands exits with a non-zero code or produces error/warning output (after applying `-D warnings` to clippy)
