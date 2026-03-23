# Spec: Write integration test in `tests/read_file_server_test.rs`

> From: .claude/tasks/issue-44.md

## Objective

Create `tools/read-file/tests/read_file_server_test.rs` to validate the `read-file` MCP server binary end-to-end over stdio transport. These tests spawn the compiled binary as a child process, connect an MCP client via `TokioChildProcess`, and exercise both `tools/list` and `tools/call` protocol paths. This is the final implementation step before documentation.

## Current State

The reference test file is `tools/echo-tool/tests/echo_server_test.rs`. Its structure is:

- A private async helper `spawn_echo_client()` that calls `TokioChildProcess::new(Command::new(env!("CARGO_BIN_EXE_echo-tool")))`, then `().serve(transport).await` to obtain a `RunningService<RoleClient, ()>`.
- Five `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` async tests, each calling `spawn_echo_client()`, invoking `client.peer().list_tools(None)` or `client.peer().call_tool(params)`, asserting on the result, and finishing with `client.cancel().await`.
- Imports from `rmcp`: `CallToolRequestParams`, `RunningService`, `TokioChildProcess`, `RoleClient`, `ServiceExt`.
- Import from `tokio::process::Command`.

The `read-file` binary will expose a single MCP tool named `read_file` (snake_case method name derived from the Rust method) with:
- Description: `"Read the contents of a file from disk and return them as a string"` (defined in `src/read_file.rs` via `#[tool(description = "...")]`).
- Input schema with a single required property `"path"` (derived from `ReadFileRequest { path: String }`).
- Tool call return: the file contents as a plain string on success, or a string beginning with `"Error"` on failure.

The `[dev-dependencies]` in `tools/read-file/Cargo.toml` include `tokio` with `rt-multi-thread`, `rmcp` with `client` and `transport-child-process`, and `serde_json` — everything required by this test file.

## Requirements

- The file must be created at `tools/read-file/tests/read_file_server_test.rs`.
- A private async helper `spawn_read_file_client()` must spawn the binary using `env!("CARGO_BIN_EXE_read-file")` and return `RunningService<RoleClient, ()>`.
- All five tests must be annotated `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- Each test must call `client.cancel().await.expect(...)` as the final statement.
- Test `tools_list_returns_read_file_tool`: assert `tools_result.tools.len() == 1` and `tools_result.tools[0].name == "read_file"`.
- Test `tools_list_read_file_has_correct_description`: assert the tool's description contains `"Read the contents of a file"`.
- Test `tools_list_read_file_has_path_parameter`: assert `input_schema` properties contain the key `"path"`.
- Test `tools_call_read_file_returns_content`: write a temp file with `std::fs::write`, call the tool with the file path as the `"path"` argument, assert the response text matches the written content exactly.
- Test `tools_call_read_file_returns_error_for_missing_file`: call the tool with a path guaranteed not to exist, assert the response text contains `"Error"`.
- No `use` imports beyond those required for the above (no unused imports).

## Implementation Details

- File to create: `tools/read-file/tests/read_file_server_test.rs`
- Imports required:
  - `rmcp::{model::CallToolRequestParams, service::RunningService, transport::TokioChildProcess, RoleClient, ServiceExt}`
  - `tokio::process::Command`
- The `spawn_read_file_client` helper mirrors `spawn_echo_client` from the echo test exactly, substituting `CARGO_BIN_EXE_read-file` and an appropriate error message string.
- For `tools_call_read_file_returns_content`: use `std::env::temp_dir()` to construct a portable temp file path (e.g., append a fixed filename like `"read_file_test_content.txt"`). Write a known string with `std::fs::write`. Pass the path as a `String` in the `serde_json::json!({ "path": path_str })` argument map. Assert `text.text == written_content`.
- For `tools_call_read_file_returns_error_for_missing_file`: use a path that is guaranteed absent, such as `"/nonexistent_path_for_testing_12345/file.txt"`. Assert `text.text.contains("Error")`.
- `CallToolRequestParams::new("read_file").with_arguments(...)` is the call pattern, matching how the echo test calls `"echo"`.
- The `input_schema` properties check uses `tool.input_schema.get("properties").expect(...).get("path").is_some()`.
- All assertions must include a descriptive failure message as the third argument to `assert!` / `assert_eq!`.

## Dependencies

- Blocked by: "Write `src/main.rs`" (the binary must exist for `env!("CARGO_BIN_EXE_read-file")` to resolve and for the child process to be spawnable)
- Blocking: "Write `README.md`"

## Risks & Edge Cases

- The temp file path used in `tools_call_read_file_returns_content` must be writable in the test environment. Using `std::env::temp_dir()` avoids hardcoding `/tmp`, which may not be writable on all platforms.
- If two test runs execute concurrently and both write the same temp filename, they could interfere. Mitigate by using a unique filename (e.g., incorporating the test name) or by accepting the race condition given the content written is identical.
- The `env!("CARGO_BIN_EXE_read-file")` macro resolves at compile time only when the binary target exists in the same crate. If `Cargo.toml` is missing the package or the binary is not built before the test binary, `cargo test` will fail to compile. This is resolved once the Cargo.toml and `src/main.rs` tasks complete.
- The `tools_call_read_file_returns_error_for_missing_file` test depends on the `read-file` implementation returning a string that contains the literal text `"Error"`. The implementation spec specifies `format!("Error reading '{}': {}", request.path, e)`, which satisfies this assertion.
- Calling `client.cancel().await` after a failed tool call (where the server returned an error string but did not crash) must succeed; the server should remain running after returning an error-string response.

## Verification

- `cargo test -p read-file` runs all five integration tests and all pass.
- `cargo clippy -p read-file` reports no warnings in the test file.
- Running `cargo test -p read-file -- --test-output immediate` shows each of the five test names in the output.
- The test `tools_list_returns_read_file_tool` confirms the binary exposes exactly one tool named `read_file` over the MCP protocol.
- The test `tools_call_read_file_returns_content` confirms real file I/O works end-to-end through the MCP stdio transport.
- The test `tools_call_read_file_returns_error_for_missing_file` confirms error strings propagate correctly through the protocol without panicking the server.
