# Spec: Create `main.rs` entrypoint for read-file tool

> From: .claude/tasks/issue-44.md

## Objective
Create the `main.rs` entrypoint for the `read-file` MCP tool server. This file follows the established echo-tool pattern: declare the tool module, import the tool struct, configure tracing to stderr, and serve the tool over stdio transport.

## Current State
- `tools/echo-tool/src/main.rs` exists and serves as the canonical template for all tool entrypoints.
- The `read-file` crate does not yet exist. The workspace members update and `Cargo.toml` scaffolding must land first.
- The tool implementation (`ReadFileTool`) will live in a sibling module `read_file.rs`, created by a separate task.

## Requirements
- Create `tools/read-file/src/main.rs` following the exact echo-tool entrypoint pattern.
- Declare `mod read_file;` and import `ReadFileTool` from that module.
- Configure `tracing_subscriber` to write to stderr with ANSI disabled and an `EnvFilter` defaulting to `DEBUG`.
- Log `"Starting read-file MCP server"` at `info` level on startup.
- Serve `ReadFileTool::new()` over `rmcp::transport::stdio()`.
- Wait for transport close via `.waiting().await`.
- No CLI argument parsing. No `clap` dependency.
- File must stay under 50 lines.

## Implementation Details

### File: `tools/read-file/src/main.rs`

```rust
use rmcp::ServiceExt;
use tracing_subscriber::{self, EnvFilter};

mod read_file;
use read_file::ReadFileTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting read-file MCP server");

    let service = ReadFileTool::new()
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

    service.waiting().await?;
    Ok(())
}
```

### Key decisions
- **`flavor = "current_thread"`**: Matches echo-tool. A single-threaded tokio runtime is sufficient for a stdio-based MCP server.
- **`Box<dyn std::error::Error>`**: Used instead of `anyhow::Result` to avoid an extra dependency, consistent with echo-tool.
- **Module name `read_file`**: Rust module names use underscores. The file will be `tools/read-file/src/read_file.rs`.

### Line budget
- Imports: 4 lines
- Module declaration + use: 2 lines
- `main` function: ~16 lines
- Blank lines: ~3 lines
- **Total: ~25 lines** (well under the 50-line limit)

## Dependencies
- Blocked by: workspace members update (adding `tools/read-file` to workspace `Cargo.toml`), `Cargo.toml` scaffolding for the read-file crate
- Blocking: read-file tool implementation (`read_file.rs`), integration tests

## Risks & Edge Cases

1. **`ReadFileTool::new()` not yet available:** This file will not compile until the `read_file.rs` module is created with a `ReadFileTool` struct that has a `new()` constructor. This is expected -- the entrypoint is scaffolded first, and the tool implementation follows.

2. **Stdout contamination:** All logging must go to stderr. The `tracing_subscriber` is explicitly configured with `.with_writer(std::io::stderr)`. No `println!` calls are permitted anywhere in the crate, as stdout is reserved for the MCP stdio transport.

3. **Module naming collision:** The module is named `read_file` (underscore), matching Rust conventions. The crate is named `read-file` (hyphen). These do not conflict.

## Verification

1. **File exists:** `tools/read-file/src/main.rs` is present and matches the implementation above.
2. **Line count:** `wc -l tools/read-file/src/main.rs` is under 50 lines.
3. **Pattern match:** The file structurally mirrors `tools/echo-tool/src/main.rs` with only the module name, struct name, and log message changed.
4. **Compilation:** `cargo check -p read-file` succeeds once the `read_file.rs` module and `Cargo.toml` are in place (not expected to pass in isolation from this task alone).
5. **Lint:** `cargo clippy -p read-file` produces no warnings (once dependencies are in place).
6. **Server starts:** `cargo run -p read-file` logs "Starting read-file MCP server" to stderr and waits for MCP client input (once the full tool is implemented).
7. **Workspace tests:** `cargo test` across the full workspace still passes.
