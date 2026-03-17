# Spec: Define `McpHandle` newtype wrapping the rmcp client session

> From: .claude/tasks/issue-9.md

## Objective

Create a newtype struct `McpHandle` that wraps the rmcp `RunningService<RoleClient, ()>` type, providing a clean, ergonomic API for the rest of the `tool-registry` crate to interact with MCP client sessions. The handle exposes the peer (for issuing tool calls and listing tools) and a graceful shutdown method. This is a runtime-only type -- it must not be serializable or deserializable -- because it holds a live connection to an MCP server process.

## Current State

- The `tool-registry` crate exists at `crates/tool-registry/` with a placeholder `pub struct ToolRegistry;` in `src/lib.rs` and an empty `[dependencies]` section in `Cargo.toml`.
- Issue #8 defines `ToolEntry`, `RegistryError`, and `ToolRegistry` with core registry methods. The `ToolEntry` struct has three fields: `name`, `version`, `endpoint` (all `String`). It does not yet have a `handle` field -- that is added by a downstream task in this issue ("Add `McpHandle` field to `ToolEntry`").
- The `rmcp` crate is not yet added as a dependency. The sibling Group 1 task "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" will add `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }`.
- The `rmcp` crate (v0.16) provides:
  - `rmcp::service::RunningService<Role, Handler>` -- a running MCP service holding the transport, peer handle, and handler.
  - `rmcp::model::RoleClient` -- the role marker type for client-side connections.
  - `rmcp::service::Peer<Role>` -- the peer handle used to send requests (e.g., `list_tools()`, `call_tool()`).
  - `rmcp::handler::ClientHandler` -- the handler trait. The unit type `()` implements `ClientHandler` with default no-op behavior, which is the standard pattern for minimal MCP clients.
  - `RunningService` implements `Clone` (it wraps `Arc`-based internals).
  - `RunningService` provides a `peer()` method returning `&Peer<RoleClient>` and a `shutdown()` async method for graceful teardown.
- The codebase convention for error types uses manual `Display + Error` implementations (no `thiserror`). Types follow a one-type-per-file pattern with private `mod` and `pub use` re-exports in `lib.rs`.

## Requirements

- Create a new file `crates/tool-registry/src/mcp_handle.rs` containing a single public struct `McpHandle`.
- The struct must have one private field wrapping the rmcp running service:
  - `inner: RunningService<RoleClient, ()>` -- the live MCP client session.
- Derive `Clone` on the struct. `RunningService` implements `Clone` (it uses `Arc` internally), so the derived impl will work.
- Do NOT derive `Serialize`, `Deserialize`, `JsonSchema`, or `PartialEq`. Handles are runtime-only, non-comparable connection wrappers.
- Derive `Debug` for diagnostic/logging purposes (if `RunningService` implements `Debug`; if it does not, implement `Debug` manually with an opaque representation like `McpHandle { .. }`).
- Implement `McpHandle::new(service: RunningService<RoleClient, ()>) -> Self` constructor.
- Implement `McpHandle::peer(&self) -> &Peer<RoleClient>` that delegates to the inner service's `peer()` method, exposing the peer for tool calls (`list_tools()`, `call_tool()`, etc.).
- Implement `McpHandle::shutdown(self)` as an async method that consumes the handle and calls the inner service's `shutdown()` for graceful teardown. This consumes `self` to prevent use-after-shutdown.
- All methods must be under 50 lines (per project rules).
- No test module in this file. Tests for `McpHandle` will be covered by integration tests in Group 5 that spin up real in-process MCP servers.
- No commented-out code, no debug statements, no placeholder TODOs.

## Implementation Details

### File to create

**`crates/tool-registry/src/mcp_handle.rs`**

- Import the necessary rmcp types:
  ```rust
  use rmcp::model::RoleClient;
  use rmcp::service::{Peer, RunningService};
  ```
- Define the struct:
  ```rust
  #[derive(Clone)]
  pub struct McpHandle {
      inner: RunningService<RoleClient, ()>,
  }
  ```
  If `RunningService` does not implement `Debug`, add a manual impl:
  ```rust
  impl std::fmt::Debug for McpHandle {
      fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
          f.debug_struct("McpHandle").finish_non_exhaustive()
      }
  }
  ```
- Implement methods:
  ```rust
  impl McpHandle {
      pub fn new(service: RunningService<RoleClient, ()>) -> Self {
          Self { inner: service }
      }

      pub fn peer(&self) -> &Peer<RoleClient> {
          self.inner.peer()
      }

      pub async fn shutdown(self) {
          self.inner.shutdown().await;
      }
  }
  ```

### Key design decisions

- **`()` as `ClientHandler`**: The unit type implements `ClientHandler` with default no-op behavior. This is the standard rmcp pattern for a minimal client that doesn't need to handle server-initiated requests (notifications, sampling, etc.). If custom handler behavior is needed later, `()` can be replaced with a concrete handler type.
- **`shutdown(self)` consumes the handle**: Taking `self` by value (not `&mut self`) prevents callers from using a handle after shutdown. Since `McpHandle` is `Clone`, callers who need to keep a copy can clone before shutting down. This is safer than `&mut self` which would leave the handle in an indeterminate state.
- **Private `inner` field**: The `RunningService` is not exposed directly. All interaction goes through `peer()` (for MCP operations) and `shutdown()` (for teardown). This encapsulates the rmcp API and allows internal changes without breaking downstream code.

### Integration points

- This file will be declared as `mod mcp_handle;` and re-exported as `pub use mcp_handle::McpHandle;` from `crates/tool-registry/src/lib.rs` in a later wiring task (or as part of the Group 3 connect implementation).
- The "Add `McpHandle` field to `ToolEntry`" task will add `handle: Option<McpHandle>` with `#[serde(skip)]` to `ToolEntry`.
- The `transport::connect_transport()` function (Group 2) returns `RunningService<RoleClient, ()>`, which gets wrapped in `McpHandle::new()` by the `connect()` method (Group 3).
- The `tool_bridge` module in `agent-runtime` (Group 4) calls `handle.peer()` to access `list_tools()` and create `McpTool` instances for rig-core integration.

## Dependencies

- Blocked by: "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" (Group 1 sibling task) -- the `rmcp` crate must be available for the `RunningService`, `RoleClient`, and `Peer` imports to resolve.
- Blocking: "Add `McpHandle` field to `ToolEntry`" (Group 2) -- `ToolEntry` needs this type to exist before adding the `handle` field.

## Risks & Edge Cases

1. **rmcp API surface**: The exact module paths for `RunningService`, `RoleClient`, and `Peer` depend on `rmcp` v0.16's public API. If the paths differ from `rmcp::service::RunningService` / `rmcp::model::RoleClient` / `rmcp::service::Peer`, the imports must be adjusted. The implementer should check the actual rmcp v0.16 documentation or source to confirm the correct import paths.

2. **`Debug` derivation**: `RunningService` may or may not implement `Debug`. If it does not, `#[derive(Debug)]` will fail to compile and a manual `Debug` implementation must be provided instead (as shown in the Implementation Details section). The implementer should attempt `derive(Debug, Clone)` first and fall back to manual `Debug` if compilation fails.

3. **`Clone` semantics**: Cloning an `McpHandle` clones the `Arc`-based internals of `RunningService`, creating a second handle to the same underlying connection. Multiple clones share the same transport. Shutting down one clone may affect others -- the implementer should verify rmcp's shutdown semantics (whether shutdown is per-handle or per-connection). This is acceptable for the current design since `shutdown(self)` consumes the clone.

4. **`shutdown()` error handling**: The rmcp `RunningService::shutdown()` method may return a `Result` rather than `()`. If so, the `McpHandle::shutdown()` method should propagate the error or map it to `RegistryError::ConnectionFailed`. The implementer should check the actual return type and adjust accordingly.

5. **Compile order**: This file depends on `rmcp` being present in `Cargo.toml`. If implemented before the dependency task, `cargo check` will fail. Both Group 1 tasks should be completed together before verifying.

6. **Future extensibility**: If the project later needs to support server-initiated notifications or sampling requests, the `()` handler can be replaced with a custom type implementing `ClientHandler`. The `McpHandle` newtype isolates this change -- only the type parameter and constructor need updating.

## Verification

After implementation (and after the "Add `rmcp` and `tokio` dependencies" task is complete):

1. Run `cargo check -p tool-registry` -- must compile without errors (note: the module must be declared in `lib.rs` for this to work, or verified in isolation by temporarily adding `mod mcp_handle;`).
2. Run `cargo clippy -p tool-registry` -- must produce no warnings.
3. Verify the file `crates/tool-registry/src/mcp_handle.rs` exists.
4. Verify `McpHandle` has exactly one field: `inner: RunningService<RoleClient, ()>`.
5. Verify `McpHandle` derives `Clone` (and `Debug` if supported by `RunningService`).
6. Verify `McpHandle` does NOT derive `Serialize`, `Deserialize`, `JsonSchema`, or `PartialEq`.
7. Verify `McpHandle::new()` accepts a `RunningService<RoleClient, ()>` and returns `Self`.
8. Verify `McpHandle::peer()` returns `&Peer<RoleClient>`.
9. Verify `McpHandle::shutdown()` is async and consumes `self`.
10. Verify all methods are under 50 lines.
11. Verify there are no test modules, no commented-out code, and no debug statements.
