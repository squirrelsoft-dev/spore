# Spec: Run verification suite

> From: .claude/tasks/issue-50.md

## Objective
Execute the full verification suite for the `register-agent` crate and the workspace as a whole, confirming that all implementation tasks have been completed correctly, all tests pass, no lint warnings exist, and no regressions have been introduced across the workspace.

## Current State
This is the final task in the issue-50 task breakdown. All prior tasks must be complete before this task can run:
- `tools/register-agent/Cargo.toml` created with correct dependencies (including `reqwest`)
- `"tools/register-agent"` added to the root workspace `Cargo.toml` members list
- `tools/register-agent/src/register_agent.rs` implemented with `RegisterAgentTool` struct, handler, input validation, HTTP POST logic, and unit tests
- `tools/register-agent/src/main.rs` implemented with stdio harness entry point
- `tools/register-agent/tests/register_agent_server_test.rs` implemented with MCP client integration tests
- `tools/register-agent/README.md` written

## Requirements
1. `cargo build -p register-agent` must succeed with no errors.
2. `cargo test -p register-agent` must succeed with all unit and integration tests passing.
3. `cargo clippy -p register-agent` must produce no warnings or errors.
4. `cargo check` (workspace-wide) must succeed with no errors, confirming no regressions in any other crate.
5. All acceptance criteria from the issue must be met:
   - Build succeeds.
   - Tests pass (unit tests for input validation, orchestrator URL resolution, error handling; integration tests for MCP tool listing and tool calls).
   - The tool returns structured error JSON when the orchestrator is unreachable.
   - The tool is registered in the MCP tool registry as `register_agent` (verified by the `tools_list_returns_register_agent_tool` integration test).

## Implementation Details
This task involves no file changes. It is a command-line verification step consisting of four sequential commands:

1. **Build**: `cargo build -p register-agent`
   - Confirms the crate compiles, all dependencies resolve, and the binary is produced.

2. **Test**: `cargo test -p register-agent`
   - Runs unit tests in `register_agent.rs` (8 tests covering empty field rejection, invalid URL scheme, shell metacharacter rejection, default/custom orchestrator URL, and orchestrator-unreachable error path).
   - Runs integration tests in `register_agent_server_test.rs` (4 tests: tool listing, empty-name error, valid-inputs structured JSON, invalid-URL error).

3. **Lint**: `cargo clippy -p register-agent`
   - Ensures no Clippy warnings. Run with default lint levels (do not pass `-- -D warnings` unless the project standard requires it).

4. **Workspace check**: `cargo check`
   - Ensures the new crate does not break any other crate in the workspace. This catches issues like conflicting dependency versions or broken inter-crate references.

Each command must be run sequentially. If any command fails, stop and diagnose before proceeding.

## Dependencies
- **Blocked by**: Every other task in issue-50 (Groups 1-3). All source files, test files, `Cargo.toml` entries, and documentation must be in place.
- **Blocking**: None. This is the final task.

## Risks & Edge Cases
- **Orchestrator not running**: The unit test `orchestrator_unreachable_returns_error` and integration test `tools_call_with_valid_inputs_returns_structured_json` both exercise the code path where the orchestrator HTTP endpoint is unavailable. These tests must assert `success: false` and check for connection error messaging, not attempt a real registration.
- **Environment variable leakage**: Unit tests that set `ORCHESTRATOR_URL` via `std::env::set_var` could interfere with each other if run in parallel. Rust runs tests in the same process by default. The tests should either use serial execution for env-var tests or use a helper function that accepts the URL as a parameter rather than reading the env directly.
- **reqwest blocking in async context**: If `reqwest::blocking::Client` is used inside a context where a tokio runtime is already running (e.g., during integration tests), it may panic. The implementation should handle this correctly, either by using `blocking` feature with appropriate runtime isolation or by using `tokio::task::spawn_blocking`.
- **Clippy false positives**: New dependencies (especially `reqwest`) may trigger Clippy lints about unused imports or features. Address any warnings before marking complete.
- **Workspace-wide regression**: The `cargo check` step catches cases where adding `reqwest` as a dependency introduces version conflicts or where the new workspace member breaks resolution for other crates.

## Verification
1. `cargo build -p register-agent` exits with code 0.
2. `cargo test -p register-agent` exits with code 0 and reports all tests passing (0 failures, 0 ignored unless intentional).
3. `cargo clippy -p register-agent` exits with code 0 and emits no warnings.
4. `cargo check` (workspace-wide) exits with code 0.
5. Review test output to confirm the expected tests ran:
   - Unit tests: `rejects_empty_name`, `rejects_empty_url`, `rejects_empty_description`, `rejects_invalid_url_scheme`, `rejects_url_with_shell_metachar`, `default_orchestrator_url`, `custom_orchestrator_url`, `orchestrator_unreachable_returns_error`.
   - Integration tests: `tools_list_returns_register_agent_tool`, `tools_call_with_empty_name_returns_error`, `tools_call_with_valid_inputs_returns_structured_json`, `tools_call_with_invalid_url_returns_error`.
