# Spec: Write `src/main.rs`

> From: .claude/tasks/issue-49.md

## Objective
Create the entrypoint file for the `docker-push` MCP tool server. This file mirrors `tools/cargo-build/src/main.rs` exactly in structure, substituting only the module name, struct name, and tool name string for docker-push.

## Current State
- `tools/cargo-build/src/main.rs` exists and serves as the canonical template. It uses the `mcp_tool_harness::serve_stdio_tool` helper to avoid inlining tracing setup, keeping the file to 7 lines.
- The `mcp-tool-harness` crate exposes `serve_stdio_tool<T: ServerHandler>(tool: T, tool_name: &str) -> Result<(), Box<dyn std::error::Error>>`, which handles tracing initialization (to stderr, ANSI disabled), startup logging, and MCP stdio transport serving.
- `tools/docker-push/src/main.rs` does not yet exist.
- `tools/docker-push/src/docker_push.rs` (the `DockerPushTool` struct and handler) is not yet implemented; this task is blocked by that work.

## Requirements
- Create `tools/docker-push/src/main.rs` following the exact same pattern as `tools/cargo-build/src/main.rs`.
- The module declaration must be `mod docker_push;`.
- The use statement must import `docker_push::DockerPushTool`.
- The `#[tokio::main]` attribute must use `flavor = "current_thread"` (matching the cargo-build pattern).
- The `main()` function must call `mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await`.
- The return type must be `Result<(), Box<dyn std::error::Error>>`.
- No direct `tracing_subscriber` setup in this file -- that is handled by `serve_stdio_tool`.
- No `clap` dependency. No CLI argument parsing.
- No `use` of `rmcp` or `tracing_subscriber` -- those are encapsulated by the harness.
- The file must be under 10 lines.

## Implementation Details

### File: `tools/docker-push/src/main.rs`

```rust
mod docker_push;
use docker_push::DockerPushTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await
}
```

This is 7 lines, well within the 10-line budget.

### Changes from `tools/cargo-build/src/main.rs`

| Line | cargo-build | docker-push |
|------|-------------|-------------|
| 1 | `mod cargo_build;` | `mod docker_push;` |
| 2 | `use cargo_build::CargoBuildTool;` | `use docker_push::DockerPushTool;` |
| 6 | `mcp_tool_harness::serve_stdio_tool(CargoBuildTool::new(), "cargo-build").await` | `mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await` |

No other lines change.

## Dependencies
- Blocked by: "Implement `DockerPushTool` struct and handler" -- the `docker_push` module must define `DockerPushTool` with a `new()` constructor and a `ServerHandler` implementation before this file can compile.
- Blocking: "Write integration tests" -- the integration tests spawn the compiled binary, which requires `main.rs` to exist.

## Risks & Edge Cases

1. **`DockerPushTool` not yet available:** This file imports `docker_push::DockerPushTool`, which must be defined in `tools/docker-push/src/docker_push.rs`. If the blocked task has not landed, compilation will fail. Mitigation: ensure the handler task is complete before attempting to build.

2. **Stdout contamination:** Any accidental `println!` or default tracing subscriber writing to stdout will corrupt the MCP stdio transport. Mitigation: `serve_stdio_tool` explicitly configures tracing with `.with_writer(std::io::stderr)`, and this file contains no direct print statements.

3. **Module naming:** The module file must be named `docker_push.rs` (with underscore), matching the `mod docker_push;` declaration. A mismatch (e.g., `docker-push.rs` with a hyphen) will cause a compilation error.

4. **Tokio runtime flavor:** The `current_thread` flavor is intentional -- MCP tool servers are single-threaded by design in this project. Using the default multi-thread flavor would work but would be inconsistent with the established pattern.

## Verification

1. **Compilation:** `cargo check -p docker-push` succeeds with no errors (requires the handler module and `Cargo.toml` to exist).
2. **Lint:** `cargo clippy -p docker-push` produces no warnings.
3. **Line count:** `wc -l tools/docker-push/src/main.rs` is under 10 lines.
4. **Diff check:** The file differs from `tools/cargo-build/src/main.rs` in exactly three lines (module declaration, use statement, serve call arguments).
5. **Server starts:** `cargo run -p docker-push` starts without errors and waits for MCP client input on stdin (verified by checking stderr log output shows "Starting docker-push MCP server").
6. **Workspace tests:** `cargo test` across the full workspace still passes (no regressions).
