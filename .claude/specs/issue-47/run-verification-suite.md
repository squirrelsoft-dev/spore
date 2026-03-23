# Spec: Run verification suite

> From: .claude/tasks/issue-47.md

## Objective
Run the full verification suite for the `cargo-build` tool crate and the workspace as a whole to confirm that the implementation is correct, complete, and introduces no regressions. This is the final gate task for issue-47 -- it validates that the crate scaffolding, core implementation, integration tests, and workspace membership all integrate correctly.

## Current State
By the time this task runs, the preceding issue-47 tasks will have:
1. Created `tools/cargo-build/Cargo.toml` with dependencies mirroring `echo-tool` (rmcp, tokio, serde, serde_json, mcp-tool-harness, mcp-test-utils)
2. Added `"tools/cargo-build"` to the workspace `members` array in the root `Cargo.toml`
3. Implemented `CargoBuildTool` in `tools/cargo-build/src/cargo_build.rs` with package name validation, `std::process::Command` invocation, and structured JSON output
4. Written `tools/cargo-build/src/main.rs` using `mcp_tool_harness::serve_stdio_tool`
5. Written unit tests for package name validation (rejects injection attempts, accepts valid names)
6. Written integration tests in `tools/cargo-build/tests/cargo_build_server_test.rs` (MCP round-trip: tools/list, successful build, failed build, invalid package name rejection)

## Requirements
- `cargo build -p cargo-build` succeeds with zero errors
- `cargo test -p cargo-build` succeeds with all tests passing, including:
  - Unit tests: `rejects_invalid_package_name`, `rejects_package_with_path_separator`, `validates_clean_package_name`
  - Integration tests: `tools_list_returns_cargo_build_tool`, `tools_call_builds_echo_tool_successfully`, `tools_call_returns_error_for_nonexistent_package`, `tools_call_rejects_invalid_package_name`
- `cargo clippy -p cargo-build` succeeds with zero warnings
- `cargo check` (workspace-wide) succeeds with zero errors, confirming no regressions in any other crate
- The tool is named `cargo_build` in the MCP `tools/list` response (verified by the integration test)
- A successful build returns structured JSON with `success: true`, `stdout`, `stderr`, and `exit_code: 0`
- A failed build (nonexistent package) returns structured JSON with `success: false` and non-empty `stderr`
- No commented-out code or debug statements remain in `cargo-build` source files
- No unused imports, dead code, or Clippy lint violations in the `cargo-build` crate

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo build -p cargo-build`** from the workspace root. This compiles the `cargo-build` binary. If it fails, diagnose the root cause -- likely candidates include:
   - Missing or incorrect dependency features in `tools/cargo-build/Cargo.toml`
   - Import errors in source files (wrong `rmcp` module paths, missing `mcp_tool_harness` import)
   - Type mismatches in the `ServerHandler` or `#[tool_router]` implementations
   - The `tools/cargo-build` path not present in the root `Cargo.toml` `members` array

2. **Run `cargo test -p cargo-build`** from the workspace root. This compiles and executes all unit and integration tests for the crate. Verify:
   - All unit tests pass (package name validation: injection attempts rejected, valid names accepted)
   - All integration tests pass (MCP client connects, tools/list returns `cargo_build`, build succeeds for `echo-tool`, build fails for nonexistent package, invalid package name is rejected before command execution)

3. **Run `cargo clippy -p cargo-build`** from the workspace root with `-- -D warnings` to treat warnings as errors. Pay attention to:
   - Unused imports or variables
   - Clippy suggestions for `Command` API usage
   - Warnings in test modules
   - Any lint issues with the `#[tool_router]` macro-generated code

4. **Run `cargo check`** (workspace-wide, no `-p` filter) to confirm that adding the `cargo-build` crate has not broken any other workspace member.

5. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `cargo-build` crate unless a workspace-level issue is discovered.

### Files potentially touched (fixes only, if needed)
- `tools/cargo-build/Cargo.toml` -- dependency version or feature adjustments
- `tools/cargo-build/src/cargo_build.rs` -- tool handler or validation fixes
- `tools/cargo-build/src/main.rs` -- import or wiring fixes
- `tools/cargo-build/tests/cargo_build_server_test.rs` -- test assertion corrections
- `Cargo.toml` -- workspace member path fix (unlikely)

## Dependencies
- Blocked by: All preceding issue-47 tasks (Groups 1-3: crate scaffold, core implementation, integration tests)
- Blocking: None (this is the final task for issue-47)

## Risks & Edge Cases
- **Integration test duration**: The integration test `tools_call_builds_echo_tool_successfully` invokes a real `cargo build -p echo-tool`. This is inherently slow (compiling a real crate). If the test runner has a short timeout, this test may fail. Mitigation: the test uses `#[tokio::test(flavor = "multi_thread")]` which has no default timeout; the CI environment should allow sufficient build time.
- **Workspace-wide `cargo check` regressions**: A failing check in an unrelated crate (e.g., `agent-sdk`, `skill-loader`) would block this task. Mitigation: if a pre-existing issue is found, confirm it also fails on the `main` branch before attributing it to `cargo-build` changes.
- **Command injection test false positive**: The unit test `validates_clean_package_name` actually invokes `cargo build -p echo-tool`. If `echo-tool` is not yet compiled or has issues, this test will fail for reasons unrelated to `cargo-build`. Mitigation: ensure `echo-tool` builds cleanly first (it should, as a workspace member).
- **Binary name vs tool name mismatch**: The binary is `cargo-build` (hyphenated, from `[package] name`), but the MCP tool name must be `cargo_build` (snake_case, from the `#[tool_router]` method name). The integration test `tools_list_returns_cargo_build_tool` explicitly verifies this mapping.
- **Stdio transport interference**: Since `cargo-build` uses stdin/stdout as the MCP transport, any accidental stdout writes (e.g., from `println!`) will corrupt the protocol. The integration tests will catch this if it happens.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Address each lint individually rather than blanket-suppressing with `#[allow]`.

## Verification
- `cargo build -p cargo-build` exits with code 0
- `cargo test -p cargo-build` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- `cargo clippy -p cargo-build -- -D warnings` exits with code 0 and produces no warning output
- `cargo check` (workspace-wide) exits with code 0, confirming no regressions
- The integration test output confirms the tool is listed as `cargo_build` in MCP tools/list
- The integration test output confirms structured JSON with `success: true` for a valid build and `success: false` with non-empty `stderr` for a failed build
