# Spec: Migrate echo-tool main.rs to use `serve_stdio_tool`

> From: Issue #56

## Objective
Replace the boilerplate in `tools/echo-tool/src/main.rs` with a single call to `mcp_tool_harness::serve_stdio_tool`, reducing the entrypoint to approximately 5 lines. Add the `mcp-tool-harness` crate as a dependency and remove tracing dependencies that are now handled internally by the harness.

## Current State
- `tools/echo-tool/src/main.rs` (24 lines) manually sets up `tracing_subscriber`, creates the `EchoTool`, calls `.serve(rmcp::transport::stdio())`, and calls `.waiting().await`.
- `tools/echo-tool/Cargo.toml` lists direct dependencies on `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, and `tracing-subscriber`.
- The tracing setup (stderr writer, env filter, ANSI disabled) and the serve-then-wait pattern are identical across tool binaries (e.g., `echo-tool`, `read-file`). This duplication is exactly what `mcp-tool-harness` eliminates.
- `tools/echo-tool/src/echo.rs` defines `EchoTool` with `ServerHandler` impl and unit tests. This file is unchanged by this migration.

## Requirements
1. `tools/echo-tool/src/main.rs` calls `mcp_tool_harness::serve_stdio_tool(EchoTool::new(), "echo-tool").await` and contains no direct tracing setup or `rmcp` transport code.
2. The `mcp-tool-harness` workspace crate is added as a dependency in `tools/echo-tool/Cargo.toml`.
3. The `tracing` and `tracing-subscriber` direct dependencies are removed from `tools/echo-tool/Cargo.toml` if they are no longer used anywhere in `echo-tool` source files (including `echo.rs` and tests). If `echo.rs` or tests still reference `tracing` macros, keep the `tracing` dependency but still remove `tracing-subscriber`.
4. The `rmcp` dependency remains because `echo.rs` uses `rmcp` types directly (`ServerHandler`, `ToolRouter`, `tool_router`, etc.).
5. The `tokio` dependency remains because `#[tokio::main]` is still used in `main.rs` and `#[tokio::test]` in tests.
6. All existing integration tests (`tools/echo-tool/tests/echo_server_test.rs`) and unit tests continue to pass without modification.
7. The binary behavior is identical: starts MCP server over stdio, logs to stderr, advertises the `echo` tool, and exits when the transport closes.

## Implementation Details

### File: `tools/echo-tool/Cargo.toml`

Add the `mcp-tool-harness` dependency as a workspace path dependency. Remove `tracing` and `tracing-subscriber` from `[dependencies]` if they are no longer referenced in the crate source. Keep all other dependencies and `[dev-dependencies]` unchanged.

After migration, the `[dependencies]` section should look approximately like:

```toml
[dependencies]
mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }
rmcp = { version = "1", features = ["transport-io", "server", "macros"] }
tokio = { version = "1", features = ["macros", "rt", "io-std"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

Note: If `echo.rs` or its tests use `tracing::info!`, `tracing::debug!`, or similar macros, keep `tracing = "0.1"` in `[dependencies]`. The current `echo.rs` does not use any tracing macros, so both `tracing` and `tracing-subscriber` should be removable.

### File: `tools/echo-tool/src/main.rs`

Replace the entire file contents with:

```rust
mod echo;
use echo::EchoTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(EchoTool::new(), "echo-tool").await
}
```

This removes:
- `use rmcp::ServiceExt;` (now internal to the harness)
- `use tracing_subscriber::{self, EnvFilter};` (now internal to the harness)
- The 5-line `tracing_subscriber::fmt()` setup block
- The `tracing::info!("Starting echo-tool MCP server");` log line (the harness logs this using the provided name)
- The `EchoTool::new().serve(rmcp::transport::stdio())` call (encapsulated by harness)
- The `service.waiting().await?;` call (encapsulated by harness)

### File: `tools/echo-tool/src/echo.rs`

No changes. The `EchoTool` struct, `ServerHandler` impl, and unit tests remain as-is.

## Dependencies
- Blocked by: "Create `crates/mcp-tool-harness` crate"
- Blocking: None

## Risks & Edge Cases

1. **Harness API mismatch:** The `serve_stdio_tool` function signature and return type must match what `main.rs` expects (`Result<(), Box<dyn std::error::Error>>`). If the harness returns a different error type, the `main.rs` return type or error conversion may need adjustment. Mitigation: confirm the harness API once the harness crate spec is finalized.

2. **Tracing dependency still needed transitively:** Even after removing `tracing` and `tracing-subscriber` from `Cargo.toml`, the `mcp-tool-harness` crate brings them in transitively. If any code in `echo.rs` or tests uses `tracing` macros (e.g., added in a future change), the build will still succeed via transitive deps but this is fragile. Mitigation: grep for `tracing::` in the crate before removing the direct dependency; if found, keep `tracing` as a direct dep.

3. **`rmcp::ServiceExt` no longer imported:** The current `main.rs` imports `rmcp::ServiceExt` for the `.serve()` method. After migration, this import is removed. If `echo.rs` also needed `ServiceExt` (it currently does not), the build would break. Mitigation: confirm `echo.rs` does not use `ServiceExt`.

4. **Behavior parity:** The harness must configure tracing identically to the current setup (stderr writer, env filter, ANSI disabled) to maintain the same logging behavior. Differences (e.g., default log level, ANSI colors) would be visible to operators. Mitigation: this is the harness crate's responsibility; verify during integration testing.

## Verification

1. **Compilation:** `cargo check -p echo-tool` succeeds with no errors.
2. **Lint:** `cargo clippy -p echo-tool -- -D warnings` produces no warnings.
3. **Unit tests:** `cargo test -p echo-tool --lib` passes all existing tests in `echo.rs`.
4. **Integration tests:** `cargo test -p echo-tool --test echo_server_test` passes all existing tests.
5. **Line count:** `wc -l tools/echo-tool/src/main.rs` is under 10 lines.
6. **No unused deps:** `tracing` and `tracing-subscriber` do not appear in `tools/echo-tool/Cargo.toml` `[dependencies]` (assuming no tracing usage in `echo.rs`).
7. **Workspace tests:** `cargo test` across the full workspace passes with no regressions.
