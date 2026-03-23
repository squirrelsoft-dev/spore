# Spec: Write `src/main.rs`

> From: .claude/tasks/issue-44.md

## Objective

Create `tools/read-file/src/main.rs` as the binary entry point for the `read-file` MCP server. This file wires the `ReadFileTool` struct into the `rmcp` stdio transport runtime and is the counterpart to `tools/echo-tool/src/main.rs`, following the same structural pattern exactly.

## Current State

`tools/echo-tool/src/main.rs` (24 lines) serves as the reference implementation. It:

- Imports `rmcp::ServiceExt` and `tracing_subscriber` with `EnvFilter`
- Declares `mod echo` and imports `EchoTool` from it
- Defines `#[tokio::main(flavor = "current_thread")]` async `main`
- Initialises a `tracing_subscriber` writing to stderr with no ANSI codes
- Logs an info message identifying the server
- Constructs the tool via `EchoTool::new()`, calls `.serve(rmcp::transport::stdio())`, and awaits `.waiting()`

`tools/read-file/src/read_file.rs` must exist and export `ReadFileTool` before this file can compile (see Dependencies).

## Requirements

- The file must be `tools/read-file/src/main.rs`
- The module declaration must be `mod read_file` (not `mod echo`)
- The struct import must be `use read_file::ReadFileTool`
- The tracing info log line must read exactly `"Starting read-file MCP server"`
- The file must be 25 lines or fewer
- All other structure (imports, tokio runtime flavour, tracing setup, serve/waiting pattern) must mirror `tools/echo-tool/src/main.rs` without deviation
- No new dependencies may be introduced; all required crates (`rmcp`, `tokio`, `tracing`, `tracing-subscriber`) are already declared in `tools/read-file/Cargo.toml`

## Implementation Details

- **File to create:** `tools/read-file/src/main.rs`
- Copy `tools/echo-tool/src/main.rs` verbatim, then apply exactly three substitutions:
  1. `mod echo` → `mod read_file`
  2. `use echo::EchoTool` → `use read_file::ReadFileTool`
  3. `"Starting echo-tool MCP server"` → `"Starting read-file MCP server"`
  4. `EchoTool::new()` → `ReadFileTool::new()`
- The `#[tokio::main(flavor = "current_thread")]` attribute and the full tracing subscriber init chain must remain unchanged
- The error inspection closure `|e| tracing::error!("serving error: {:?}", e)` must remain unchanged

## Dependencies

- Blocked by: "Implement `ReadFileTool` struct and handler in `src/read_file.rs`" — `ReadFileTool` and its `new()` constructor must be defined and exported from `tools/read-file/src/read_file.rs` before this file compiles
- Blocking: "Write integration test in `tests/read_file_server_test.rs`" — the integration test spawns the compiled `read-file` binary, which requires this entry point to exist

## Risks & Edge Cases

- **Module name collision:** Rust's module system requires `mod read_file` to match the filename `read_file.rs` exactly. A mismatch (e.g., `mod readfile`) will cause a compile error.
- **`current_thread` runtime:** The single-threaded tokio runtime is intentional for stdio-based MCP servers. Do not change the flavour to `multi_thread`.
- **ANSI codes disabled:** `with_ansi(false)` is required because the server communicates over stdio; ANSI escape sequences in log output would corrupt the MCP message stream. This line must not be removed.

## Verification

- `cargo build -p read-file` completes without errors or warnings
- `cargo clippy -p read-file` reports no issues
- The compiled binary exists at the path reported by `cargo build` (typically `target/debug/read-file`)
- Line count: `wc -l tools/read-file/src/main.rs` reports 25 or fewer
- The integration test suite (`cargo test -p read-file`) passes, confirming the binary starts and responds to MCP protocol messages
