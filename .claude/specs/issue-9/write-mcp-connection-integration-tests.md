# Spec: Write MCP connection integration tests

> From: .claude/tasks/issue-9.md

## Objective

Create integration tests that verify the `ToolRegistry` can establish real MCP client connections over TCP and Unix socket transports, list tools from a connected server, and call tools through the connection handle. These tests spin up in-process MCP servers using `rmcp` server-side APIs so they are self-contained and do not require external infrastructure. This validates the full connection lifecycle implemented in Groups 1-3 (dependencies, `McpHandle`, transport module, and `connect()` method).

## Current State

### tool-registry crate

- `crates/tool-registry/Cargo.toml` currently has no dependencies and no dev-dependencies section. Once the Group 1 task ("Add `rmcp` and `tokio` dependencies") completes, it will have `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }` and `tokio = { version = "1", features = ["net"] }` as dependencies, plus `tokio = { version = "1", features = ["macros", "rt", "net"] }` as a dev-dependency.
- `crates/tool-registry/src/lib.rs` currently contains only `pub struct ToolRegistry;`. Once issue #8 and issue #9 Groups 1-3 complete, it will re-export `ToolRegistry` (with `register()`, `connect()`, `get_handle()`, `connect_all()`), `ToolEntry`, `RegistryError`, and `McpHandle`.
- No integration test files exist yet under `crates/tool-registry/tests/`.

### Existing test patterns

- Integration tests use one file per concern under `crates/<name>/tests/<name>_test.rs`.
- Tests use `#[tokio::test]` for async tests (see `crates/skill-loader/tests/validation_integration_test.rs`).
- Tests use helper functions at the top of the file for reusable setup (e.g., `make_loader()`, `make_manifest()`).
- Assertions use `assert!`, `assert_eq!`, and pattern matching on error enums.
- Tests import from the crate's public API only (not `crate::` internal paths).

### rmcp server-side APIs (v0.16)

The `rmcp` crate with the `server` feature provides:
- `ServerHandler` trait for implementing an MCP server that responds to tool calls, list-tools requests, etc.
- `serve_server()` / `().serve(transport)` patterns for starting a server on a transport.
- `RoleServer` type parameter for server-side sessions.
- Tool definitions via `rmcp::model::Tool`, `ToolInputSchema`, etc.
- The `transport-async-rw` feature enables using `TcpStream` and `UnixStream` as transports for both client and server sides.

## Requirements

1. Add `rmcp = { version = "0.16", features = ["server", "transport-async-rw"] }` to `[dev-dependencies]` in `crates/tool-registry/Cargo.toml`. The `server` feature is needed only for tests (to spin up in-process MCP servers); production code uses only the `client` feature.
2. Add `tempfile = "3"` to `[dev-dependencies]` for creating temporary Unix socket paths.
3. Create `crates/tool-registry/tests/mcp_connection_test.rs` with the following five tests:

### Test 1: `connect_tcp_succeeds`
- Start a TCP listener on `127.0.0.1:0` (OS-assigned port).
- Spawn a tokio task that accepts one connection and serves an MCP server on it using a minimal `ServerHandler` implementation.
- Create a `ToolRegistry`, register a `ToolEntry` with endpoint `mcp://127.0.0.1:{port}`.
- Call `registry.connect("{tool_name}")` and assert it returns `Ok(())`.
- Assert `registry.get_handle("{tool_name}")` returns `Some(handle)`.

### Test 2: `connect_unix_succeeds`
- Create a temporary directory and a socket path within it.
- Start a Unix listener on that path.
- Spawn a tokio task that accepts one connection and serves an MCP server on it.
- Create a `ToolRegistry`, register a `ToolEntry` with endpoint `mcp+unix://{socket_path}`.
- Call `registry.connect("{tool_name}")` and assert it returns `Ok(())`.
- Assert `registry.get_handle("{tool_name}")` returns `Some(handle)`.

### Test 3: `call_tool_through_handle`
- Start a TCP listener and spawn a mock MCP server that exposes one tool (e.g., `"echo"`) and returns a fixed result when that tool is called.
- Register the tool, connect, obtain the handle.
- Call the tool through the handle's peer: `handle.peer().call_tool(...)`.
- Assert the response contains the expected result content.

### Test 4: `connect_to_invalid_endpoint_returns_error`
- Create a `ToolRegistry`, register a `ToolEntry` with an endpoint that nothing is listening on (e.g., `mcp://127.0.0.1:1` or a non-existent Unix socket path).
- Call `registry.connect("{tool_name}")`.
- Assert the result is `Err(RegistryError::ConnectionFailed { .. })`.
- Assert the error's `endpoint` field matches the registered endpoint.

### Test 5: `list_tools_through_handle`
- Start a TCP listener and spawn a mock MCP server that advertises two tools (e.g., `"tool_a"` and `"tool_b"`) with names and descriptions.
- Register the tool, connect, obtain the handle.
- Call `handle.peer().list_tools(...)` to discover available tools.
- Assert the returned tool list contains exactly the two expected tool names.

## Implementation Details

### Files to create/modify

1. **`crates/tool-registry/Cargo.toml`** -- Add to `[dev-dependencies]`:
   ```toml
   rmcp = { version = "0.16", features = ["server", "transport-async-rw"] }
   tempfile = "3"
   ```
   Note: `tokio` with `macros`, `rt`, and `net` should already be present in dev-dependencies from the Group 1 dependency task. If not, also add:
   ```toml
   tokio = { version = "1", features = ["macros", "rt", "net"] }
   ```

2. **`crates/tool-registry/tests/mcp_connection_test.rs`** -- New file with:
   - A `MockServer` struct implementing `rmcp::ServerHandler` that:
     - Advertises a configurable list of tools (name + description + input schema).
     - Returns a fixed JSON result when a tool is called.
     - Keeps the implementation under 50 lines per method.
   - A `start_tcp_server(mock: MockServer) -> (SocketAddr, JoinHandle<()>)` helper that:
     - Binds a `TcpListener` to `127.0.0.1:0`.
     - Returns the bound address and a `JoinHandle` for the spawned accept loop.
     - The accept loop accepts one connection, wraps it in an MCP server transport, and serves the `MockServer`.
   - A `start_unix_server(mock: MockServer, path: &Path) -> JoinHandle<()>` helper that:
     - Binds a `UnixListener` to the given path.
     - Accepts one connection and serves the `MockServer`.
   - A `make_registry_with_entry(name: &str, endpoint: &str) -> ToolRegistry` helper that creates a `ToolRegistry`, registers a single `ToolEntry`, and returns it.
   - Five `#[tokio::test]` test functions as described in Requirements.

### Key types and interfaces used

From `tool-registry` (public API, available after Groups 1-3 of issue #9 complete):
- `ToolRegistry::new() -> Self`
- `ToolRegistry::register(entry: ToolEntry) -> Result<(), RegistryError>`
- `ToolRegistry::connect(name: &str) -> Result<(), RegistryError>` -- establishes MCP client connection
- `ToolRegistry::get_handle(name: &str) -> Option<McpHandle>` -- retrieves connection handle
- `McpHandle::peer() -> &Peer<RoleClient>` -- exposes the rmcp peer for tool calls
- `ToolEntry { name, version, endpoint }` -- tool entry with MCP endpoint URL
- `RegistryError::ConnectionFailed { endpoint, reason }` -- connection failure variant

From `rmcp` (server-side, dev-dependency):
- `rmcp::ServerHandler` trait -- implement to define server behavior
- `rmcp::model::Tool` -- tool definition with name, description, input schema
- `rmcp::model::CallToolResult` -- result of a tool call
- `rmcp::service::serve_server()` or the `.serve()` pattern on transport
- `rmcp::RoleServer` -- type parameter for server sessions

From `tokio`:
- `tokio::net::TcpListener` -- for binding a TCP server
- `tokio::net::UnixListener` -- for binding a Unix socket server
- `tokio::task::JoinHandle` -- for managing spawned server tasks

### Integration points

- Tests exercise the `ToolRegistry` -> `transport::connect_transport()` -> `McpHandle` pipeline end-to-end.
- The mock server validates that the client-side code produces valid MCP protocol messages.
- The `call_tool_through_handle` test validates that the `McpHandle::peer()` accessor returns a usable peer for making RPC calls.
- The `list_tools_through_handle` test validates that tool discovery works through the connection.

## Dependencies

- **Blocked by**: "Implement `connect()` with real MCP client logic" (Group 3 of issue #9) -- the tests call `registry.connect()` and `registry.get_handle()`, which must exist and function correctly. This also transitively depends on Groups 1-2 (dependencies, `McpHandle`, transport module, `ToolEntry.handle` field).
- **Blocking**: "Run verification suite" (Group 6 of issue #9) -- the verification suite runs `cargo test` and expects these tests to pass.

## Risks & Edge Cases

1. **rmcp server-side API surface**: The `ServerHandler` trait API in rmcp 0.16 may differ from what is documented. Mitigation: check the actual rmcp 0.16 source/docs at implementation time and adjust the `MockServer` implementation accordingly. The trait may require implementing additional methods beyond tool listing and tool calling (e.g., `initialize`, `get_info`).

2. **Port conflicts in CI**: Using `127.0.0.1:0` for TCP tests avoids port conflicts since the OS assigns an ephemeral port. This is the standard pattern and should work reliably in CI.

3. **Unix socket path length**: Unix socket paths have a maximum length (typically 104-108 bytes). Using `tempfile::tempdir()` produces short paths, but if the CI tmpdir has a long prefix, this could fail. Mitigation: use a short socket filename like `s.sock` within the tempdir.

4. **Test flakiness from timing**: The server accept loop runs in a spawned task. There is a race between the server being ready and the client connecting. Mitigation: the `TcpListener::bind()` completes synchronously before the client connects, so the port is ready. For extra safety, the `start_tcp_server` helper returns the address only after binding, guaranteeing the listener is active.

5. **Unix-only tests**: `UnixStream`/`UnixListener` are not available on Windows. Since the CI environment is Linux (confirmed by the environment info), this is not a concern. However, if Windows support is needed later, the Unix-specific tests should be gated with `#[cfg(unix)]`.

6. **Mock server lifetime**: The spawned server task must stay alive long enough for the client to connect and make calls. The accept loop handles one connection and then exits. If the test needs multiple operations on the same connection, the server task must not exit after the first message. Mitigation: the server serves the full session (not just one message) -- `().serve(stream).await` runs until the client disconnects or the session ends.

7. **Cleanup of Unix sockets**: The `tempfile::tempdir()` automatically cleans up on drop, removing the socket file. No manual cleanup needed.

## Verification

1. `cargo check -p tool-registry --tests` succeeds with no errors.
2. `cargo clippy -p tool-registry --tests` produces no warnings.
3. `cargo test -p tool-registry -- mcp_connection` runs all five tests and they pass:
   - `connect_tcp_succeeds` -- PASS
   - `connect_unix_succeeds` -- PASS
   - `call_tool_through_handle` -- PASS
   - `connect_to_invalid_endpoint_returns_error` -- PASS
   - `list_tools_through_handle` -- PASS
4. `cargo test` across the full workspace still passes (no regressions).
5. Each test completes within 5 seconds (no hanging connections or timeouts).
