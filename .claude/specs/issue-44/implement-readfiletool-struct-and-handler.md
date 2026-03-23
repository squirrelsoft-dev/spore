# Spec: Implement `ReadFileTool` struct and handler in `src/read_file.rs`

> From: .claude/tasks/issue-44.md

## Objective

Create `tools/read-file/src/read_file.rs`, which defines the core MCP tool logic for reading file contents from disk. This module is the heart of the `read-file` crate: it exposes a single `read_file` tool over the MCP `tools/call` protocol, returning file contents as a string or a descriptive error string on failure. It mirrors `tools/echo-tool/src/echo.rs` exactly in structure and macro usage.

## Current State

The `tools/echo-tool` crate is the canonical reference implementation. Its `src/echo.rs` shows the full pattern this module must follow:

- `EchoRequest` — a `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` struct with a single doc-commented field.
- `EchoTool { tool_router: ToolRouter<Self> }` — constructed via `Self::tool_router()` in `new()`.
- `#[tool_router] impl EchoTool` — contains the annotated tool method returning `String`.
- `#[tool_handler] impl ServerHandler for EchoTool` — implements `get_info()` to advertise tool capability.
- A `#[cfg(test)] mod tests` block with synchronous-style unit tests that call the method directly.

The `read-file` crate directory (`tools/read-file/`) does not yet exist. This task depends on `tools/read-file/Cargo.toml` being created first (Group 1 of the task breakdown).

## Requirements

1. The file is `tools/read-file/src/read_file.rs`.
2. `ReadFileRequest` must be defined with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` and a single `pub path: String` field with doc comment `/// The path to the file to read (absolute or relative)`.
3. `ReadFileTool` must hold `tool_router: ToolRouter<Self>` and provide a `pub fn new() -> Self` that initialises it via `Self::tool_router()`.
4. The `impl ReadFileTool` block must be annotated with `#[tool_router]`.
5. The `read_file` method inside that block must carry `#[tool(description = "Read the contents of a file from disk and return them as a string")]`.
6. The method signature must be `fn read_file(&self, Parameters(request): Parameters<ReadFileRequest>) -> String`.
7. The implementation must call `std::fs::read_to_string(&request.path)`, return the content on `Ok`, and on `Err` return a string of the form `format!("Error reading '{}': {}", request.path, e)`.
8. The `read_file` method body must be 15 lines or fewer; error formatting must be inline (no helper function).
9. `impl ServerHandler for ReadFileTool` must be annotated with `#[tool_handler]` and implement `get_info()` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.
10. A `#[cfg(test)] mod tests` block must contain exactly three unit tests:
    - `read_file_returns_content` — creates a temp file via `std::fs::write`, calls the tool, asserts content matches.
    - `read_file_returns_error_for_missing_file` — calls the tool with a nonexistent path, asserts the result contains `"Error"`.
    - `read_file_returns_error_for_directory` — calls the tool with a directory path (use `std::env::temp_dir()`), asserts the result contains `"Error"`.
11. Temp file paths in tests must be derived from `std::env::temp_dir()` for portability (no hardcoded `/tmp`).
12. No new crate dependencies may be introduced; all required items (`rmcp`, `serde`, `schemars`) are already declared in `Cargo.toml`.
13. No commented-out code or debug statements in the final file.

## Implementation Details

**File to create:** `tools/read-file/src/read_file.rs`

**Imports** (mirror `echo.rs` exactly):
```
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

**`ReadFileRequest`** — follows the same single-field struct pattern as `EchoRequest`, with `path: String` and its doc comment placed above the field.

**`ReadFileTool::new()`** — identical boilerplate to `EchoTool::new()`, substituting the type name.

**`#[tool_router] impl ReadFileTool`** — the `read_file` method uses a `match` or `unwrap_or_else` on `std::fs::read_to_string` to produce the `String` return. Either style is acceptable as long as the body is at most 15 lines.

**`#[tool_handler] impl ServerHandler for ReadFileTool`** — copy `get_info()` verbatim from `echo.rs`; the content is identical.

**Test helper pattern** — tests call the tool method directly (no spawning), consistent with `echo.rs`. Each test constructs `ReadFileTool::new()` and invokes `tool.read_file(Parameters(ReadFileRequest { path: ... }))`.

**Temp file naming** — use a unique filename per test (e.g., `temp_dir().join("read_file_test_content.txt")`) to avoid cross-test collisions; clean-up after assertions is optional since temp files are ephemeral.

## Dependencies

- **Blocked by:** "Create `tools/read-file/Cargo.toml`" — the crate manifest must exist for this module to compile. `ToolRouter`, `Parameters`, `ServerCapabilities`, `ServerInfo`, and the `rmcp` macros are all pulled from the `rmcp` dependency declared there.
- **Blocking:** "Write `src/main.rs`" — `main.rs` imports `mod read_file` and uses `ReadFileTool`; it cannot be written until this module exists.
- **Blocking:** "Write integration test in `tests/read_file_server_test.rs`" — integration tests spawn the compiled binary, which in turn requires this module to compile and link correctly.

## Risks & Edge Cases

- **Directory reads are errors on all target platforms**: `std::fs::read_to_string` on a directory returns an `Err` on Linux and macOS. The test `read_file_returns_error_for_directory` relies on this; it is safe to assert the error-string branch is hit.
- **Binary file reads**: `read_to_string` returns `Err` for files containing invalid UTF-8. This is acceptable behaviour and does not need special handling in this task; no size or encoding guard is required.
- **Temp directory path in tests**: `std::env::temp_dir()` returns an `OsString`-based `PathBuf`. Convert to `String` via `.to_string_lossy().to_string()` or `.display().to_string()` when constructing the `ReadFileRequest { path }` field, since the field type is `String`.
- **Test isolation**: If multiple tests write to the same temp filename concurrently, they could interfere. Use distinct filenames per test case.
- **`#[tool_router]` macro requirement**: The method `read_file` must be inside the `#[tool_router]`-annotated `impl` block. Placing it in a separate `impl` block will cause a compile error.
- **Return type constraint**: The `#[tool_router]` macro requires the method to return `String`, not `Result<String, _>`. All error paths must produce a `String` directly.

## Verification

1. `cargo check -p read-file` — confirms the module compiles and all types resolve correctly. (Requires `Cargo.toml` to exist.)
2. `cargo test -p read-file` — runs the three unit tests; all must pass.
3. `cargo clippy -p read-file` — must produce no warnings in `read_file.rs`.
4. Inspect test output to confirm:
   - `read_file_returns_content` produces the exact bytes written to the temp file.
   - `read_file_returns_error_for_missing_file` result string starts with or contains `"Error"`.
   - `read_file_returns_error_for_directory` result string starts with or contains `"Error"`.
