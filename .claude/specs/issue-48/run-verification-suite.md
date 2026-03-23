# Spec: Run verification suite

> From: .claude/tasks/issue-48.md

## Objective
Validate that the fully implemented `docker-build` crate compiles, passes all tests, satisfies linting rules, and meets every acceptance criterion defined in the task breakdown. This is the final gate before the feature branch is considered complete.

## Current State
N/A — this is a verification-only task. All implementation tasks (crate scaffold, core handler, main entry point, integration tests, workspace registration) must be complete before this task runs.

## Requirements
- `cargo build -p docker-build` exits with code 0 (no compilation errors or warnings treated as errors).
- `cargo test -p docker-build` exits with code 0 and all tests pass, including:
  - Unit tests for input validation (path traversal rejection, tag sanitization, build-arg key validation, clean input acceptance).
  - Integration tests (tools/list returns `docker_build`, path traversal rejected via MCP call, invalid tag rejected via MCP call, graceful error when Docker is unavailable).
- `cargo clippy -p docker-build` exits with code 0 (no lint warnings).
- `cargo check` (workspace-wide, no `-p` flag) exits with code 0, confirming the new crate does not break any other workspace member.
- The MCP tools/list response contains exactly one tool named `docker_build` with a description containing "Docker image" and parameters including `context`, `tag`, `build_args`, and `dockerfile`.
- On valid inputs where Docker is unavailable, the tool returns structured JSON with `success: false` and a human-readable error in `build_log` (not a crash or unstructured text).
- On invalid inputs (path traversal, shell metacharacters in tag, bad build-arg keys), the tool returns structured JSON with `success: false` and a validation error message — the invalid input must never reach a `docker` subprocess.

## Implementation Details
Run the following commands in order. Each must succeed before proceeding to the next.

1. **Build the crate:**
   ```
   cargo build -p docker-build
   ```
   Expected: exit code 0, binary produced at `target/debug/docker-build`.

2. **Run tests:**
   ```
   cargo test -p docker-build
   ```
   Expected: exit code 0, all unit and integration tests pass. Review test output to confirm each named test case executed.

3. **Run linter:**
   ```
   cargo clippy -p docker-build
   ```
   Expected: exit code 0, no warnings.

4. **Workspace-wide check:**
   ```
   cargo check
   ```
   Expected: exit code 0 for every workspace member.

5. **Spot-check acceptance criteria** (already covered by integration tests, but confirm in test output):
   - `tools_list_returns_docker_build_tool` — tool name is `docker_build`.
   - `tools_call_rejects_path_traversal` — validation error returned as JSON.
   - `tools_call_rejects_invalid_tag` — validation error returned as JSON.
   - `tools_call_returns_error_when_docker_unavailable` — structured JSON with `success: false`.

## Dependencies
- Blocked by: All previous tasks (Create `Cargo.toml`, Add workspace member, Implement `DockerBuildTool`, Write `main.rs`, Write integration tests, Write `README.md`)
- Blocking: None

## Risks & Edge Cases
- **Docker availability:** CI environments may not have Docker installed. The integration test `tools_call_returns_error_when_docker_unavailable` is designed to handle this gracefully, but if Docker happens to be available, the test should still pass (it checks for valid JSON structure regardless of success/failure).
- **Flaky tests from process spawning:** The `spawn_mcp_client!` macro starts a child process. If the binary path is wrong or the build step was skipped, integration tests will fail with an opaque error. Always run `cargo build` before `cargo test`.
- **Clippy version differences:** Different Rust toolchain versions may surface different clippy lints. If a new lint appears, fix the code rather than suppressing it.
- **Workspace breakage:** Adding a new member could surface dependency conflicts or feature-flag incompatibilities. The `cargo check` step catches this at the workspace level.

## Verification
- All four commands (`cargo build -p docker-build`, `cargo test -p docker-build`, `cargo clippy -p docker-build`, `cargo check`) exit with code 0.
- Test output shows every expected test name (unit and integration) with status `ok`.
- No warnings or errors in any command output.
