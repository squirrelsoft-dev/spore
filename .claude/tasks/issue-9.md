# Task Breakdown: Integrate rmcp for MCP tool protocol

> Add the `rmcp` crate to the tool-registry so that `ToolRegistry` can establish real MCP client connections to tool servers over TCP and Unix socket transports, and integrate with `rig-core` so the agent-runtime can use MCP tool handles when building agents.

## Group 1 — Add rmcp dependency and MCP handle type

_Tasks in this group can be done in parallel._

- [x] **Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml** `[S]`
      Add `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }` and `tokio = { version = "1", features = ["net"] }` to `crates/tool-registry/Cargo.toml` dependencies. The `client` feature enables `serve_client`, `Peer<RoleClient>`, `ServerSink`, and `ClientHandler`. The `transport-async-rw` feature enables `IntoTransport` for `TcpStream` and `UnixStream`. Pin `rmcp` to `0.16` because `rig-core 0.32` depends on `rmcp ^0.16` — using rmcp 1.x would cause a version conflict. Add `tokio = { version = "1", features = ["macros", "rt", "net"] }` to dev-dependencies for async tests.
      Files: `crates/tool-registry/Cargo.toml`
      Blocking: "Create transport module", "Implement `connect()` with real MCP client logic", "Add `McpHandle` field to `ToolEntry`"

- [x] **Define `McpHandle` newtype wrapping the rmcp client session** `[S]`
      Create `crates/tool-registry/src/mcp_handle.rs` with a newtype struct wrapping the rmcp running service. The handle holds `RunningService<RoleClient, ()>` (since `()` implements `ClientHandler` with default no-op behavior). Implement `McpHandle::peer(&self) -> &Peer<RoleClient>` to expose the peer for tool calls, and `McpHandle::close(&mut self)` for graceful shutdown. Derive `Clone`. Do NOT derive `Serialize`/`Deserialize` — handles are runtime-only, not serializable.
      Files: `crates/tool-registry/src/mcp_handle.rs`
      Blocking: "Add `McpHandle` field to `ToolEntry`"

## Group 2 — Transport and connection plumbing

_Depends on: Group 1._

- [x] **Create transport module with endpoint URL parsing** `[M]`
      Create `crates/tool-registry/src/transport.rs` with `pub(crate) async fn connect_transport(endpoint: &str) -> Result<RunningService<RoleClient, ()>, RegistryError>` that: (1) Parses endpoint URL to determine transport type — `mcp://host:port` → TCP, `mcp+unix:///path` → Unix socket. (2) For TCP: `TcpStream::connect(addr).await`, then `().serve(stream).await`. (3) For Unix: `UnixStream::connect(path).await`, then `().serve(stream).await`. (4) Map errors to `RegistryError::ConnectionFailed`. Add helper `parse_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError>` returning enum `TransportTarget { Tcp { host: String, port: u16 }, Unix { path: PathBuf } }`. Keep each function under 50 lines. Include unit tests for `parse_endpoint`.
      Files: `crates/tool-registry/src/transport.rs`
      Blocked by: "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml"
      Blocking: "Implement `connect()` with real MCP client logic"

- [x] **Add `McpHandle` field to `ToolEntry`** `[S]`
      Add `handle: Option<McpHandle>` field to `ToolEntry` in `crates/tool-registry/src/tool_entry.rs`. Mark with `#[serde(skip)]` since handles are runtime-only. Switch from derived `PartialEq` to a manual implementation that compares only `name`, `version`, and `endpoint`. Update existing tests to include `handle: None`.
      Files: `crates/tool-registry/src/tool_entry.rs`
      Blocked by: "Define `McpHandle` newtype wrapping the rmcp client session", "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml"
      Blocking: "Implement `connect()` with real MCP client logic"

## Group 3 — Connect method implementation

_Depends on: Group 2._

- [x] **Implement `connect()` with real MCP client logic** `[M]`
      Replace the stub `connect()` in `crates/tool-registry/src/tool_registry.rs` with real implementation: (1) Read lock → find entry by name → `RegistryError::ToolNotFound` if missing. (2) Clone endpoint URL. (3) `transport::connect_transport(&endpoint).await?`. (4) Wrap in `McpHandle`. (5) Write lock → set entry's `handle` to `Some(mcp_handle)`. Add `connect_all()` to connect all entries, and `get_handle(name) -> Option<McpHandle>`. Update `lib.rs` to re-export `McpHandle`.
      Files: `crates/tool-registry/src/tool_registry.rs`, `crates/tool-registry/src/lib.rs`
      Blocked by: "Create transport module", "Add `McpHandle` field to `ToolEntry`"
      Blocking: "Implement MCP-to-rig-core bridge in agent-runtime"

## Group 4 — rig-core integration in agent-runtime

_Depends on: Group 3._

- [x] **Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml** `[S]`
      Add `rig-core = { version = "0.32", features = ["rmcp"] }`, `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }`, `tool-registry = { path = "../tool-registry" }`, `agent-sdk = { path = "../agent-sdk" }`, `skill-loader = { path = "../skill-loader" }`, `tokio = { version = "1", features = ["full"] }`. The `rig-core` `rmcp` feature enables `McpTool` and `AgentBuilder::rmcp_tools()`.
      Files: `crates/agent-runtime/Cargo.toml`
      Blocking: "Implement MCP-to-rig-core bridge in agent-runtime"

- [x] **Implement MCP-to-rig-core bridge in agent-runtime** `[M]`
      Create `crates/agent-runtime/src/tool_bridge.rs` with `pub fn resolve_mcp_tools(registry: &ToolRegistry, manifest: &SkillManifest) -> Result<Vec<McpTool>, RegistryError>` that: (1) Calls `registry.resolve_for_skill(manifest)` to get matching entries. (2) For entries with connected `McpHandle`, calls `handle.peer().list_tools()` to discover tools. (3) Creates `McpTool` instances for each tool definition paired with the `ServerSink`. Also add helper `pub async fn build_agent_with_tools(builder: AgentBuilder, tools: Vec<McpTool>) -> Agent`.
      Files: `crates/agent-runtime/src/tool_bridge.rs`
      Blocked by: "Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml", "Implement `connect()` with real MCP client logic"
      Blocking: "Update agent-runtime main.rs"

- [x] **Update agent-runtime main.rs with skeleton startup flow** `[M]`
      Replace the `println!("Hello, world!")` stub with a `#[tokio::main]` async main demonstrating: (1) Create `ToolRegistry::new()`. (2) Register tool entries. (3) `registry.connect_all().await`. (4) Load skill manifest via `SkillLoader`. (5) Resolve MCP tools via `tool_bridge::resolve_mcp_tools()`. (6) Build rig-core agent with tools. This is a scaffold — actual HTTP serving, config parsing, and production error handling are deferred.
      Files: `crates/agent-runtime/src/main.rs`
      Blocked by: "Implement MCP-to-rig-core bridge in agent-runtime"
      Blocking: None

## Group 5 — Integration tests

_Depends on: Group 3. Can be done in parallel with Group 4._

- [x] **Write transport unit tests** `[S]`
      Add `#[cfg(test)] mod tests` in `crates/tool-registry/src/transport.rs` with tests for `parse_endpoint`: valid TCP (`mcp://localhost:7001`), valid Unix (`mcp+unix:///var/run/tool.sock`), TCP with IP address, missing scheme, unknown scheme, missing port, invalid port, Unix with no path.
      Files: `crates/tool-registry/src/transport.rs`
      Blocked by: "Create transport module"
      Blocking: "Run verification suite"

- [x] **Write MCP connection integration tests** `[L]`
      Create `crates/tool-registry/tests/mcp_connection_test.rs`. Spin up in-process MCP servers using rmcp server-side APIs. Add `rmcp = { version = "0.16", features = ["server", "transport-async-rw"] }` to dev-dependencies. Tests: (1) `connect_tcp_succeeds` (2) `connect_unix_succeeds` (3) `call_tool_through_handle` (4) `connect_to_invalid_endpoint_returns_error` (5) `list_tools_through_handle`. Use `#[tokio::test]` throughout.
      Files: `crates/tool-registry/tests/mcp_connection_test.rs`
      Blocked by: "Implement `connect()` with real MCP client logic"
      Blocking: "Run verification suite"

## Group 6 — Verification

_Depends on: Groups 4 and 5._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the workspace. Verify: no compiler errors, no clippy warnings, all existing tests pass, all new tests pass, `rmcp 0.16` and `rig-core 0.32` resolve without version conflicts.
      Blocked by: All previous tasks
      Blocking: None

---

## Design Notes

1. **rmcp pinned to 0.16**: `rig-core 0.32` depends on `rmcp ^0.16`. Using rmcp 1.x would cause a Cargo version conflict.
2. **`()` as ClientHandler**: The unit type implements `ClientHandler` with default no-op behavior — standard pattern for a minimal MCP client.
3. **`transport-async-rw` feature**: Enables `IntoTransport` for `AsyncRead + AsyncWrite` types. Both `TcpStream` and `UnixStream` satisfy this.
4. **`McpHandle` naming**: Uses `McpHandle` (not `ToolServerHandle`) to clarify this wraps an rmcp client session, not a rig-core local tool server.
5. **`ToolEntry.handle` is `Option` and `#[serde(skip)]`**: Handles are runtime-only and may not exist before connection.
6. **WASM support deferred**: Out of scope for this issue.
7. **Depends on issue #8**: Assumes `ToolEntry`, `RegistryError`, `ToolRegistry` with `register()`, `assert_exists()`, `resolve_for_skill()`, `get()` all exist.
