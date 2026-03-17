# Task Breakdown: Create an example MCP tool implementation

> Build a reference `echo` tool in `tools/echo-tool/` as a standalone Rust binary that starts an MCP server via `rmcp`, responds to tool calls, and serves as the template for all future tool implementations.

## Group 1 — Crate scaffolding

_Tasks in this group can be done in parallel._

- [x] **Create `tools/echo-tool/` crate with Cargo.toml** `[S]`
      Create the directory `tools/echo-tool/src/` and a `Cargo.toml` with `name = "echo-tool"`, `version = "0.1.0"`, `edition = "2024"`. Dependencies: `rmcp = { version = "1", features = ["transport-io", "server"] }`, `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`, `serde = { version = "1", features = ["derive"] }`, `serde_json = "1"`, `tracing = "0.1"`, `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`. Dev-dependencies: `tokio = { version = "1", features = ["macros", "rt"] }`. This is the minimal set for an MCP tool server; `clap` is intentionally omitted to keep dependencies minimal per CLAUDE.md.
      Files: `tools/echo-tool/Cargo.toml`
      Blocking: "Implement echo tool server", "Write unit tests", "Write integration test"

- [x] **Add `tools/echo-tool` to workspace members** `[S]`
      Add `"tools/echo-tool"` to the `members` array in the root `Cargo.toml`. This enables `cargo build -p echo-tool`, `cargo test -p echo-tool`, and `cargo clippy -p echo-tool`.
      Files: `Cargo.toml`
      Blocking: "Implement echo tool server", "Write unit tests", "Write integration test"

## Group 2 — Core implementation

_Depends on: Group 1._

- [x] **Implement echo tool server** `[M]`
      Create `tools/echo-tool/src/main.rs` with a complete MCP tool server. Follow the `rmcp` counter example pattern:

      1. Define an `EchoTool` struct (can be unit-like or empty).
      2. Apply `#[tool_router]` to the impl block. Define a single method `echo` with `#[tool(description = "Returns the input message unchanged")]` that accepts a `message: String` parameter and returns `Result<CallToolResult, McpError>` with `CallToolResult::success(vec![Content::text(message)])`.
      3. Implement `ServerHandler` for `EchoTool` with `get_info()` returning `ServerInfo` that enables tools via `ServerCapabilities::builder().enable_tools().build()`.
      4. In `main()`: initialize `tracing_subscriber` logging to stderr (never stdout, which is the MCP transport channel), create `EchoTool`, call `.serve(rmcp::transport::stdio()).await`, and `.waiting().await` for the server loop.
      5. Keep `main.rs` under 50 lines by extracting the `EchoTool` struct and its impls. If needed, split into `main.rs` (entrypoint) and `echo.rs` (tool definition).

      Reference: `rmcp` examples at `https://github.com/modelcontextprotocol/rust-sdk/tree/main/examples/servers`.
      Files: `tools/echo-tool/src/main.rs` (and optionally `tools/echo-tool/src/echo.rs`)
      Blocked by: "Create `tools/echo-tool/` crate with Cargo.toml", "Add `tools/echo-tool` to workspace members"
      Blocking: "Write unit tests", "Write integration test", "Write tool README"

## Group 3 — Tests and documentation

_Depends on: Group 2. Tasks in this group can be done in parallel._

- [x] **Write unit tests for echo tool logic** `[S]`
      Add `#[cfg(test)] mod tests` (either in `main.rs` or the extracted `echo.rs` module). Test that the echo tool handler returns the input message unchanged. Directly construct the tool struct, call the tool method with a test message, and assert the response contains the same message in a `Content::text(...)`. Follow the async test pattern using `#[tokio::test]`.
      Files: `tools/echo-tool/src/main.rs` (or `tools/echo-tool/src/echo.rs`)
      Blocked by: "Implement echo tool server"
      Blocking: "Run verification suite"

- [x] **Write integration test for MCP server round-trip** `[M]`
      Create `tools/echo-tool/tests/echo_server_test.rs`. Start the echo-tool binary as a child process, connect to it as an MCP client over stdio, send a `tools/call` request for the `echo` tool with `{ "message": "hello" }`, and assert the response contains `"hello"`. Use `rmcp`'s client-side API with `transport-child-process` feature or manually spawn the binary with `tokio::process::Command` and communicate over stdin/stdout. Include a test for the `tools/list` method to verify the tool schema is correctly advertised.
      Files: `tools/echo-tool/tests/echo_server_test.rs`, `tools/echo-tool/Cargo.toml` (dev-dependency additions)
      Blocked by: "Implement echo tool server"
      Blocking: "Run verification suite"

- [x] **Write tool README** `[S]`
      Create `tools/echo-tool/README.md` documenting: (1) what the tool does, (2) how to build: `cargo build -p echo-tool`, (3) how to run: `cargo run -p echo-tool` (stdio transport), (4) how to test with MCP inspector: `npx @modelcontextprotocol/inspector cargo run -p echo-tool`, (5) how to create a new tool using this as a template. Note that registration in the tool-registry will be documented once issue #8 is complete.
      Files: `tools/echo-tool/README.md`
      Blocked by: "Implement echo tool server"
      Blocking: None

## Group 4 — Cleanup and verification

_Depends on: Group 3._

- [x] **Remove `tools/.gitkeep`** `[S]`
      Remove `tools/.gitkeep` since the directory is no longer empty after adding `echo-tool/`.
      Files: `tools/.gitkeep`
      Blocked by: "Implement echo tool server"
      Blocking: None

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the entire workspace. Ensure no warnings, all existing tests still pass, and all new echo-tool tests pass. Verify `cargo run -p echo-tool` starts without errors.
      Files: (none — command-line only)
      Blocked by: "Write unit tests for echo tool logic", "Write integration test for MCP server round-trip"
      Blocking: None

## Notes for implementers

1. **Issues #8 and #9 are still OPEN**: The tool-registry is a stub (`pub struct ToolRegistry;`) and `rmcp` is not yet in the workspace. This task is the first to bring in `rmcp` as a dependency. Registration examples should be deferred until issue #8 lands.
2. **Stdio transport only**: TCP/HTTP transport can be added as a follow-up.
3. **`rmcp` features**: Default features (`server`, `macros`, `base64`) cover most needs. `transport-io` is additionally required for `rmcp::transport::stdio()`. For integration tests, `transport-child-process` may be needed as a dev-dependency feature.
4. **Logging to stderr**: MCP servers using stdio transport must never write to stdout (it is the transport channel).
5. **Edition 2024**: All crates in this workspace use `edition = "2024"`. The echo-tool must match.
6. **No `clap`**: Omitted per CLAUDE.md guidance to avoid unnecessary dependencies.
