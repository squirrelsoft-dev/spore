# Spec: Run Verification Suite

> From: .claude/tasks/issue-44.md

## Objective

Run the four cargo commands that confirm the `read-file` MCP tool crate is correct, complete, and causes no regressions in the wider workspace. This is the final gate for the `read_file` implementation: it validates the build, unit tests, lints, and the workspace-wide type-check all succeed before the issue is closed.

## Current State

The workspace root `Cargo.toml` at `/Users/sbeardsley/Developer/squirrelsoft-dev/spore/Cargo.toml` currently contains these members:

```
crates/agent-sdk
crates/skill-loader
crates/tool-registry
crates/agent-runtime
crates/orchestrator
tools/echo-tool
```

The `tools/read-file` crate will be added by the "Add `tools/read-file` to workspace `Cargo.toml`" task. All prior tasks (Groups 1–4) must be complete before this verification step runs:

- `tools/read-file/Cargo.toml` — package manifest
- Root `Cargo.toml` updated with `"tools/read-file"` in `members`
- `tools/read-file/src/read_file.rs` — `ReadFileTool` implementation and unit tests
- `tools/read-file/src/main.rs` — binary entry point
- `tools/read-file/tests/read_file_server_test.rs` — integration tests
- `tools/read-file/README.md` — documentation

The reference crate `tools/echo-tool` uses the same dependency set (`rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`) and integration test pattern (`TokioChildProcess` + `CARGO_BIN_EXE_<name>`), so its passing state is the baseline.

There are no CI workflow files (no `.github/workflows/`) in the repository. Verification is manual via the four commands described below.

## Requirements

- `cargo build -p read-file` must exit with status 0, producing the `read-file` binary.
- `cargo test -p read-file` must exit with status 0, with all unit tests in `src/read_file.rs` and all integration tests in `tests/read_file_server_test.rs` passing:
  - Unit: `read_file_returns_content`, `read_file_returns_error_for_missing_file`, `read_file_returns_error_for_directory`
  - Integration: `tools_list_returns_read_file_tool`, `tools_list_read_file_has_correct_description`, `tools_list_read_file_has_path_parameter`, `tools_call_read_file_returns_content`, `tools_call_read_file_returns_error_for_missing_file`
- `tools/list` over stdio must return exactly one tool with `name == "read_file"` (verified by `tools_list_returns_read_file_tool`).
- `tools/call` over stdio must return file contents on success and an error string on failure (verified by `tools_call_read_file_returns_content` and `tools_call_read_file_returns_error_for_missing_file`).
- `cargo clippy -p read-file` must exit with status 0 with no warnings or errors emitted.
- `cargo check` (workspace-wide, no `-p` flag) must exit with status 0, confirming no regressions in any workspace member.

## Implementation Details

This task involves no file creation or modification. It is exclusively command execution and output inspection.

Commands to run in order:

1. `cargo build -p read-file`
   - Confirm exit code 0 and that the binary is produced at `target/debug/read-file`.

2. `cargo test -p read-file`
   - Confirm exit code 0.
   - Confirm all 8 tests (3 unit + 5 integration) are listed as `ok` in the output.
   - The integration tests spawn the `read-file` binary as a child process via `CARGO_BIN_EXE_read-file`; `cargo test` builds the binary before running tests, so a separate `cargo build` is not required for this step, but running build first provides an earlier failure signal.

3. `cargo clippy -p read-file`
   - Confirm exit code 0 and no `warning:` or `error:` lines in the output.
   - If warnings appear, they must be resolved in the source files before this task is considered complete.

4. `cargo check`
   - Confirm exit code 0 across all workspace members.
   - This verifies that adding `tools/read-file` to the workspace did not introduce any type errors or dependency conflicts in sibling crates.

## Dependencies

- Blocked by: "Create `tools/read-file/Cargo.toml`", "Add `tools/read-file` to workspace `Cargo.toml`", "Implement `ReadFileTool` struct and handler in `src/read_file.rs`", "Write `src/main.rs`", "Write integration test in `tests/read_file_server_test.rs`", "Write `README.md`"
- Blocking: None

## Risks & Edge Cases

- **Missing workspace member**: If `tools/read-file` was not added to the root `Cargo.toml` `members` list, `cargo build -p read-file` will fail with `error: package ID specification 'read-file' did not match any packages`. Confirm the workspace task completed before running these commands.
- **Integration test binary not built**: The integration tests rely on `CARGO_BIN_EXE_read-file`. If the binary failed to compile, `cargo test` will fail at the integration test stage even if unit tests pass. The `cargo build -p read-file` step surfaces this earlier.
- **Clippy edition lint**: The `edition = "2024"` setting enables stricter lints. Any `read_file.rs` or `main.rs` code that passes `cargo check` might still emit clippy warnings (e.g., unnecessary `mut`, unused imports). These must be fixed, not suppressed with `#[allow(...)]` attributes, unless the attribute was already present in the reference `echo-tool`.
- **Temp file collisions in tests**: Unit tests and integration tests both write to `std::env::temp_dir()`. If tests run in parallel they could collide. Each test should use a unique filename (e.g., incorporating the test name or a UUID). If tests are currently non-unique, this is a bug to report rather than silently ignore.
- **Workspace-wide check regressions**: `cargo check` without `-p` re-checks all crates. A version conflict between `tools/read-file` and an existing workspace crate (unlikely given the shared dependency set) would surface here. If this step fails while the `-p read-file` commands pass, the root cause is in the workspace dependency graph, not in `read-file` itself.

## Verification

- All four commands exit with status 0 in sequence.
- `cargo test -p read-file` output shows exactly 8 tests passing with no failures or ignored tests.
- The integration test output includes a line matching `tools_list_returns_read_file_tool ... ok`, confirming `tools/list` returns a tool named `read_file` over stdio.
- The integration test output includes a line matching `tools_call_read_file_returns_content ... ok`, confirming `tools/call` works over stdio.
- `cargo clippy -p read-file` output contains no `warning:` or `error:` lines (only the `Finished` line).
- `cargo check` output contains no `error:` lines for any workspace member.
