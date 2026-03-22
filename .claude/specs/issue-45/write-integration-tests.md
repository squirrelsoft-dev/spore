# Spec: Write integration tests

> From: .claude/tasks/issue-45.md

## Objective

Create integration tests for the `write-file` MCP tool in `tools/write-file/tests/write_file_server_test.rs`, following the established pattern in `tools/echo-tool/tests/echo_server_test.rs`. The tests exercise the tool's MCP interface end-to-end by spawning the binary as a child process and communicating over the MCP protocol.

## Current State

No integration tests exist for the `write-file` tool. The `echo-tool` tests provide the reference pattern: spawn the tool binary via `TokioChildProcess`, connect an MCP client with `ServiceExt::serve`, and exercise `list_tools` / `call_tool` endpoints.

## Requirements

1. **`tools_list_returns_write_file_tool`** — Spawn the `write-file` binary, call `list_tools`, assert exactly 1 tool is returned and its name is `write_file`.
2. **`tools_list_write_file_has_correct_description`** — Assert the tool's description contains the substring `"Write content to a file"`.
3. **`tools_list_write_file_has_path_and_content_parameters`** — Assert `input_schema.properties` contains both `path` and `content` keys.
4. **`tools_call_write_file_creates_file`** — Call the tool via MCP with a temporary file path and known content string, then read the file back from disk and assert the contents match exactly.

Every test must cancel the client at the end (matching the echo-tool teardown pattern).

## Implementation Details

- Add a helper function `spawn_write_file_client` that mirrors `spawn_echo_client`:
  - Use `TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_write-file")))` to spawn the binary.
  - Connect and return `RunningService<RoleClient, ()>`.
- All test functions use the attribute `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- For test 4 (`tools_call_write_file_creates_file`):
  - Use `std::env::temp_dir()` (or `tempfile` if already a dependency) to construct a unique temporary file path. Ensure the file does not exist before the call.
  - Pass `{ "path": "<temp_path>", "content": "<test_content>" }` as arguments to `call_tool`.
  - After the call, use `std::fs::read_to_string` to read the file back and assert equality with the original content.
  - Clean up the temporary file in a scope guard or at the end of the test.
- Import the same crates used by the echo-tool tests: `rmcp::{model::CallToolRequestParams, service::RunningService, transport::TokioChildProcess, RoleClient, ServiceExt}` and `tokio::process::Command`.
- The test file must be listed in (or auto-discovered by) the `write-file` crate's `Cargo.toml` under `[[test]]` or via the default `tests/` directory convention.

## Dependencies

- **Blocked by**: "Implement `WriteFileTool` struct and handler" — the binary must exist and expose the `write_file` tool over MCP before these tests can pass.
- **Blocking**: "Write README" — the README can reference passing tests as evidence of correctness.

## Risks & Edge Cases

- **Temp file collisions**: Use a unique file name per test run (e.g., include a UUID or timestamp) to avoid conflicts in parallel CI.
- **Leftover temp files**: Ensure cleanup happens even if assertions fail; consider a drop guard.
- **Platform paths**: `std::env::temp_dir()` is cross-platform, but avoid hardcoding path separators.
- **Binary not found**: If the `write-file` crate name or binary target name differs from `write-file`, the `CARGO_BIN_EXE_write-file` env var will fail at compile time. Confirm the exact binary name in `Cargo.toml`.

## Verification

- `cargo test -p write-file` passes with all four tests green.
- `cargo clippy -p write-file` reports no warnings on the test file.
- The temporary file created in test 4 is cleaned up after the test completes.
