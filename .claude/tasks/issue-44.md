# Task Breakdown: Implement read_file MCP tool

> Implement `read_file` as a standalone Rust MCP server binary that reads file contents from disk, following the `echo-tool` reference pattern exactly.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/read-file/Cargo.toml`** `[S]`
      Copy and adapt `tools/echo-tool/Cargo.toml`. Change `name = "read-file"`. Keep the same dependencies: `rmcp` with `transport-io`, `server`, `macros` features; `tokio` with `macros`, `rt`, `io-std`; `serde` with `derive`; `serde_json`; `tracing`; `tracing-subscriber` with `env-filter`. Add the same `[dev-dependencies]` block with `tokio` `rt-multi-thread`, `rmcp` with `client` and `transport-child-process`, and `serde_json` — these are needed for the integration test.
      Files: `tools/read-file/Cargo.toml`
      Blocking: "Implement `ReadFileTool` struct and handler", "Write `main.rs`", "Write integration test"

- [x] **Add `"tools/read-file"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/read-file"` to the `members` list in the root `Cargo.toml`, following the same pattern as `"tools/echo-tool"`. This makes `cargo build -p read-file` and `cargo test -p read-file` work.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `ReadFileTool` struct and handler in `src/read_file.rs`** `[M]`
      Create `tools/read-file/src/read_file.rs`. Define `ReadFileRequest { path: String }` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` and a doc comment `/// The path to the file to read (absolute or relative)`. Define `ReadFileTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`. Annotate the `impl` block with `#[tool_router]` and add a `read_file` method with `#[tool(description = "Read the contents of a file from disk and return them as a string")]`. The method signature is `fn read_file(&self, Parameters(request): Parameters<ReadFileRequest>) -> String`. Implementation: call `std::fs::read_to_string(&request.path)` and return the content on success; on `Err`, return a descriptive error string like `format!("Error reading '{}': {}", request.path, e)`. Keep the method under 15 lines by keeping error formatting inline. Annotate `impl ServerHandler for ReadFileTool` with `#[tool_handler]` and implement `get_info()` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`. Add `#[cfg(test)] mod tests` with unit tests: (1) `read_file_returns_content` — write a temp file using `std::fs::write`, call the tool, assert the content matches; (2) `read_file_returns_error_for_missing_file` — call with a nonexistent path, assert the returned string contains "Error"; (3) `read_file_returns_error_for_directory` — call with a directory path (e.g., `/tmp`), assert the returned string contains "Error". Use `std::env::temp_dir()` for portable temp file paths.
      Files: `tools/read-file/src/read_file.rs`
      Blocked by: "Create `tools/read-file/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write integration test"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/read-file/src/main.rs`. Mirror `tools/echo-tool/src/main.rs` exactly, but change module name to `read_file`, struct name to `ReadFileTool`, and the log line to `"Starting read-file MCP server"`. The file should be under 25 lines.
      Files: `tools/read-file/src/main.rs`
      Blocked by: "Implement `ReadFileTool` struct and handler in `src/read_file.rs`"
      Blocking: "Write integration test"

## Group 3 — Integration test

_Depends on: Group 2_

- [x] **Write integration test in `tests/read_file_server_test.rs`** `[M]`
      Create `tools/read-file/tests/read_file_server_test.rs`. Mirror `tools/echo-tool/tests/echo_server_test.rs`. Use `env!("CARGO_BIN_EXE_read-file")` to spawn the binary as a child process via `TokioChildProcess`. Write these tests (each annotated `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`): (1) `tools_list_returns_read_file_tool` — assert `tools_result.tools.len() == 1` and `tools[0].name == "read_file"`; (2) `tools_list_read_file_has_correct_description` — assert description contains `"Read the contents of a file"`; (3) `tools_list_read_file_has_path_parameter` — assert `input_schema` properties contain `"path"`; (4) `tools_call_read_file_returns_content` — write a temp file with `std::fs::write`, call the tool with the path, assert the response text matches the written content; (5) `tools_call_read_file_returns_error_for_missing_file` — call the tool with a nonexistent path, assert the response text contains `"Error"`.
      Files: `tools/read-file/tests/read_file_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: "Write `README.md`"

## Group 4 — Documentation and registration

_Depends on: Group 3_

- [x] **Write `README.md`** `[S]`
      Create `tools/read-file/README.md`. Model it after `tools/echo-tool/README.md`. Include sections: build command (`cargo build -p read-file`), run command (`cargo run -p read-file`), description of stdio transport, MCP Inspector test command (`npx @modelcontextprotocol/inspector cargo run -p read-file`), input/output description (`path` input, file contents or error string as output). Keep it short (under 40 lines).
      Files: `tools/read-file/README.md`
      Blocked by: "Write integration test in `tests/read_file_server_test.rs`"
      Blocking: "Run verification suite"

## Group 5 — Verification

_Depends on: Groups 1–4_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p read-file`, then `cargo test -p read-file`, then `cargo clippy -p read-file`, then `cargo check` (workspace-wide) to confirm no regressions. All four acceptance criteria from the issue must pass: build succeeds, tests pass, `tools/list` and `tools/call` work over stdio (verified by integration tests), and the tool name in `tools/list` is `read_file`.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

---

## Implementation Notes

1. **No new dependencies**: All dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`) already appear in `tools/echo-tool/Cargo.toml`. No third-party crates need to be added.

2. **Error handling returns a string**: The `rmcp` `#[tool_router]` macro expects the tool method to return `String`. Errors are surfaced as descriptive error strings (not `Result`), consistent with how simple tools communicate failure over MCP text content type.

3. **Binary name**: The `[package] name = "read-file"` means the binary is `read-file` and the env macro is `CARGO_BIN_EXE_read-file`. The MCP tool name exposed over the protocol is `read_file` (snake_case method name).

4. **File size guard**: The issue does not specify a size limit. A size guard can be added as a follow-up; it should not block this issue.

5. **Registry is environment-driven**: The `agent-runtime` reads `TOOL_ENDPOINTS` env var. No code changes to the registry are needed — operators configure `read_file=mcp://localhost:<port>` to register the tool at runtime.
