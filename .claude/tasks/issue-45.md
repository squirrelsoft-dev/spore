# Task Breakdown: Implement write_file MCP tool

> Implement `write_file` as a standalone Rust MCP server binary following the echo-tool reference pattern, enabling agents to write content to files on disk.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/write-file/Cargo.toml`** `[S]`
      Copy from `tools/echo-tool/Cargo.toml` and change the package name to `write-file`. Keep the same dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`) and dev-dependencies. No new dependencies are needed since `std::fs` covers directory creation and file writing.
      Files: `tools/write-file/Cargo.toml`
      Blocking: "Add `tools/write-file` to workspace members"

- [x] **Add `tools/write-file` to workspace members** `[S]`
      Add `"tools/write-file"` to the `members` list in the root `Cargo.toml` workspace section, after the existing `"tools/echo-tool"` entry.
      Files: `Cargo.toml`
      Blocked by: "Create `tools/write-file/Cargo.toml`"
      Blocking: "Implement `WriteFileTool` struct and handler", "Create `main.rs` entrypoint"

## Group 2 — Core implementation

_Depends on: Group 1._

- [x] **Implement `WriteFileTool` struct and handler** `[M]`
      Create `tools/write-file/src/write_file.rs` following the `echo.rs` pattern. This file must contain:
      1. A `WriteFileRequest` struct with two fields: `path: String` (file path to write) and `content: String` (content to write), deriving `Deserialize` and `JsonSchema`.
      2. A `WriteFileTool` struct holding a `ToolRouter<Self>`, with a `new()` constructor (same pattern as `EchoTool`).
      3. A `#[tool_router]` impl block with a single `#[tool(description = "Write content to a file on disk, creating parent directories as needed")]` method `write_file` that:
         - Validates the path is non-empty, returning a descriptive error string if empty.
         - Calls `std::fs::create_dir_all` on the parent directory (extracted from the path) to create parent directories as needed. Returns a descriptive error if directory creation fails.
         - Uses `std::fs::write` to write the content to the file.
         - Returns a confirmation message with the number of bytes written and the path on success (e.g., `"Wrote 1234 bytes to /path/to/file"`).
         - Returns a descriptive error string on failure (permission denied, disk full, invalid path).
      4. A `#[tool_handler]` impl of `ServerHandler` with `get_info()` returning capabilities with tools enabled (identical to echo-tool pattern).
      Keep the `write_file` method under 50 lines by extracting validation into a helper function (e.g., `validate_write_path`).
      Files: `tools/write-file/src/write_file.rs`
      Blocked by: "Add `tools/write-file` to workspace members"
      Blocking: "Write unit tests"

- [x] **Create `main.rs` entrypoint** `[S]`
      Create `tools/write-file/src/main.rs` following the `echo-tool/src/main.rs` pattern exactly. Declare `mod write_file;`, import `WriteFileTool`, initialize tracing to stderr, create the tool, and serve over stdio transport. Update the log message to `"Starting write-file MCP server"`.
      Files: `tools/write-file/src/main.rs`
      Blocked by: "Add `tools/write-file` to workspace members"
      Blocking: None

## Group 3 — Tests

_Depends on: Group 2._

- [x] **Write unit tests** `[M]`
      Add a `#[cfg(test)] mod tests` block in `tools/write-file/src/write_file.rs` with test cases:
      1. `write_file_creates_file_with_content` — Write to a temp dir path, then read back with `std::fs::read_to_string` and assert content matches.
      2. `write_file_creates_parent_directories` — Write to a nested path inside a temp dir (e.g., `tmpdir/a/b/c/file.txt`), assert file exists and content is correct.
      3. `write_file_empty_path` — Call with an empty string path, assert descriptive error.
      4. `write_file_returns_byte_count` — Write known content, assert the confirmation message contains the correct byte count.
      5. `write_file_overwrites_existing` — Write to a file, write again with different content, assert the second content is present.
      6. `write_file_preserves_unicode` — Write unicode content, read back and verify round-trip.
      Use `std::env::temp_dir()` or `tempfile`-style manual temp dirs for isolation. Clean up temp files after each test.
      Files: `tools/write-file/src/write_file.rs`
      Blocked by: "Implement `WriteFileTool` struct and handler"
      Blocking: "Write integration tests"

- [x] **Write integration tests** `[M]`
      Create `tools/write-file/tests/write_file_server_test.rs` following the `echo_server_test.rs` pattern. Include tests:
      1. `tools_list_returns_write_file_tool` — Spawn the binary, call `list_tools`, assert exactly 1 tool named `write_file`.
      2. `tools_list_write_file_has_correct_description` — Assert the description contains "Write content to a file".
      3. `tools_list_write_file_has_path_and_content_parameters` — Assert `input_schema.properties` contains both `path` and `content`.
      4. `tools_call_write_file_creates_file` — Call the tool via MCP with a temp path and content, then read back the file and verify.
      Files: `tools/write-file/tests/write_file_server_test.rs`
      Blocked by: "Implement `WriteFileTool` struct and handler"
      Blocking: "Write README"

## Group 4 — Documentation and verification

_Depends on: Group 3._

- [x] **Write README** `[S]`
      Create `tools/write-file/README.md` following `tools/echo-tool/README.md` pattern. Include: purpose (write content to files, creating parent dirs as needed), build/run/test commands (`cargo build -p write-file`, etc.), MCP Inspector command, stdio transport note, input parameters (`path`, `content`).
      Files: `tools/write-file/README.md`
      Blocked by: "Write integration tests"
      Blocking: "Run verification suite"

- [x] **Run verification suite** `[S]`
      Run `cargo build -p write-file`, `cargo test -p write-file`, `cargo clippy -p write-file`, and `cargo check` (full workspace). Verify all acceptance criteria pass.
      Files: (none — command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **No new dependencies**: Only `std::fs` needed for `create_dir_all` and `write`.
2. **Error handling**: Return descriptive error strings (not Result), matching echo-tool pattern. Cover permission denied, disk full, and invalid path cases.
3. **Parent directory creation**: Always call `create_dir_all` on the parent before writing. If the path has no parent (e.g., just a filename), write to the current directory.
4. **Tool registry**: No registry code changes needed. Registration via `TOOL_ENDPOINTS` env var at deploy time.
5. **Function size**: Keep handler under 50 lines; extract validation into helpers.

## Critical Files

- `tools/echo-tool/src/echo.rs` — Reference tool pattern
- `tools/echo-tool/src/main.rs` — Reference main entrypoint
- `tools/echo-tool/Cargo.toml` — Dependency template
- `tools/echo-tool/tests/echo_server_test.rs` — Integration test pattern
- `Cargo.toml` — Workspace members list
