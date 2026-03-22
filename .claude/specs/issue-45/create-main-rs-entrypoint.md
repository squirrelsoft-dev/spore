# Spec: Create `main.rs` entrypoint

> From: .claude/tasks/issue-45.md

## Objective

Create `tools/write-file/src/main.rs` as the binary entrypoint for the `write-file` MCP tool, following the exact pattern established by `tools/echo-tool/src/main.rs`.

## Current State

The file `tools/write-file/src/main.rs` does not yet exist. The reference implementation at `tools/echo-tool/src/main.rs` provides the exact structure to replicate: it declares a module, imports the tool struct, initializes tracing to stderr, constructs the tool, and serves it over stdio transport.

## Requirements

1. Declare `mod write_file;` to reference the sibling module that will contain the `WriteFileTool` implementation.
2. Import `WriteFileTool` from the `write_file` module via `use write_file::WriteFileTool;`.
3. Import `rmcp::ServiceExt` and `tracing_subscriber::{self, EnvFilter}` exactly as the echo-tool does.
4. Use `#[tokio::main(flavor = "current_thread")]` as the async runtime attribute.
5. Initialize `tracing_subscriber` with:
   - `EnvFilter::from_default_env()` with a default directive of `tracing::Level::DEBUG`.
   - Writer set to `std::io::stderr`.
   - ANSI colors disabled (`.with_ansi(false)`).
6. Log an info-level message: `"Starting write-file MCP server"`.
7. Construct the tool via `WriteFileTool::new()`, serve it with `rmcp::transport::stdio()`, and await the service with `.waiting().await`.
8. Propagate errors using `Box<dyn std::error::Error>` as the return type, with `inspect_err` logging on serve failure.

## Implementation Details

The file should mirror `tools/echo-tool/src/main.rs` with only these differences:

| Aspect | echo-tool | write-file |
|---|---|---|
| Module declaration | `mod echo;` | `mod write_file;` |
| Import | `use echo::EchoTool;` | `use write_file::WriteFileTool;` |
| Log message | `"Starting echo-tool MCP server"` | `"Starting write-file MCP server"` |
| Tool constructor | `EchoTool::new()` | `WriteFileTool::new()` |

Everything else (imports, tracing setup, serve/waiting pattern, error handling) remains identical.

## Dependencies

- **Blocked by**: "Add `tools/write-file` to workspace members" -- the crate must exist in `Cargo.toml` workspace members before this file can compile.
- **Crate dependencies**: `rmcp` (with `transport` feature), `tokio`, `tracing`, `tracing-subscriber` -- these must be declared in `tools/write-file/Cargo.toml`.
- **Sibling module**: `tools/write-file/src/write_file.rs` must exist and export `WriteFileTool` with a `new()` constructor and the necessary trait implementations for `rmcp::ServiceExt`.

## Risks & Edge Cases

- If the `write_file` module is not yet implemented, the crate will fail to compile. This is expected since the module and the entrypoint may be developed in parallel; the entrypoint can be written first as long as compilation is deferred until the module exists.
- The `WriteFileTool::new()` constructor signature must match what the module ultimately exports. If `new()` requires parameters, this file will need updating.

## Verification

1. `cargo check -p write-file` succeeds (once all sibling modules and workspace membership are in place).
2. `cargo clippy -p write-file` reports no warnings.
3. The file is structurally identical to `tools/echo-tool/src/main.rs` aside from the four substitutions listed in the Implementation Details table.
4. Running the binary produces the log line `"Starting write-file MCP server"` on stderr when `RUST_LOG=debug` is set.
