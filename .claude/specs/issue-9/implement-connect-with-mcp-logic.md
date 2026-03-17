# Spec: Implement `connect()` with real MCP client logic

> From: .claude/tasks/issue-9.md

## Objective

Replace the stub `connect()` method on `ToolRegistry` with a real MCP client connection flow that uses the `transport` module and `McpHandle` type. Add companion methods `connect_all()` and `get_handle()` so that callers can connect all registered tools in one call and retrieve live handles for tool invocation. Update `lib.rs` to re-export `McpHandle` as part of the crate's public API. This transforms the tool-registry from a passive data store into an active connection manager capable of establishing MCP client sessions with tool servers.

## Current State

**`crates/tool-registry/src/tool_registry.rs`** (as defined by issue-8 spec):
- Contains a `ToolRegistry` struct with field `entries: Arc<RwLock<HashMap<String, ToolEntry>>>`.
- Methods: `new()`, `register()`, `assert_exists()`, `resolve_for_skill()`, `get()`.
- Has a stub `connect(_url: &str)` static method with body `// TODO: real MCP connection logic in issue #9`. This is a static method (no `&self`) that returns `()`.
- Uses `std::sync::RwLock` (not tokio).

**`crates/tool-registry/src/lib.rs`** (as defined by issue-8 spec):
- Declares private modules: `tool_entry`, `registry_error`, `tool_registry`.
- Re-exports: `ToolEntry`, `RegistryError`, `ToolRegistry`.

**`crates/tool-registry/src/tool_entry.rs`** (as defined by issue-8 spec, then extended by sibling issue-9 task "Add `McpHandle` field to `ToolEntry`"):
- Has fields: `name: String`, `version: String`, `endpoint: String`.
- After the blocking sibling task completes, will have an additional field: `handle: Option<McpHandle>` with `#[serde(skip)]`.

**`crates/tool-registry/src/mcp_handle.rs`** (created by sibling issue-9 task "Define `McpHandle` newtype"):
- Wraps `RunningService<RoleClient, ()>`.
- Derives `Clone`.
- Exposes `peer(&self) -> &Peer<RoleClient>` and `close(&mut self)`.

**`crates/tool-registry/src/transport.rs`** (created by sibling issue-9 task "Create transport module"):
- `pub(crate) async fn connect_transport(endpoint: &str) -> Result<RunningService<RoleClient, ()>, RegistryError>`.
- Parses `mcp://host:port` (TCP) and `mcp+unix:///path` (Unix socket) endpoints.
- Maps connection failures to `RegistryError::ConnectionFailed`.

**`crates/tool-registry/src/registry_error.rs`** (from issue-8):
- Variants: `ToolNotFound { name }`, `DuplicateEntry { name }`, `ConnectionFailed { endpoint, reason }`.

## Requirements

1. **Remove the stub `connect(_url: &str)` static method** from `ToolRegistry`. It is replaced by an instance method with a different signature.

2. **Implement `connect(&self, name: &str) -> Result<(), RegistryError>`** (async):
   - Acquire a read lock on `entries`.
   - Find the entry by `name`. If not found, return `Err(RegistryError::ToolNotFound { name: name.to_string() })`.
   - Clone the `endpoint` string from the found entry (so the read lock can be released).
   - Drop the read lock (it must not be held across `.await` points since this is `std::sync::RwLock`).
   - Call `transport::connect_transport(&endpoint).await?` to establish the MCP client session.
   - Wrap the returned `RunningService` in `McpHandle::new(...)`.
   - Acquire a write lock on `entries`.
   - Find the entry again by `name` (it must still exist; if removed between the read and write, return `RegistryError::ToolNotFound`).
   - Set the entry's `handle` field to `Some(mcp_handle)`.
   - Return `Ok(())`.

3. **Implement `connect_all(&self) -> Result<(), RegistryError>`** (async):
   - Acquire a read lock to collect all entry names into a `Vec<String>`.
   - Drop the read lock.
   - Iterate over the collected names and call `self.connect(&name).await?` for each.
   - Return `Ok(())` if all connections succeed, or propagate the first error.

4. **Implement `get_handle(&self, name: &str) -> Option<McpHandle>`**:
   - Acquire a read lock on `entries`.
   - Look up the entry by `name`.
   - If found and `entry.handle` is `Some`, clone and return it.
   - Otherwise return `None`.

5. **Update `lib.rs`** to:
   - Add `mod mcp_handle;` and `mod transport;` module declarations (if not already added by sibling tasks).
   - Add `pub use mcp_handle::McpHandle;` re-export.
   - The `transport` module should remain `pub(crate)` -- it is an internal implementation detail and should NOT be `pub use`-exported.

6. All methods must be under 50 lines each (per project rules).

7. No test code in `tool_registry.rs`. Integration tests are handled by a separate task ("Write MCP connection integration tests").

## Implementation Details

### Files to modify

**`crates/tool-registry/src/tool_registry.rs`**

- Add imports at the top:
  ```rust
  use crate::mcp_handle::McpHandle;
  use crate::transport;
  ```
- Remove the existing stub:
  ```rust
  pub fn connect(_url: &str) {
      // TODO: real MCP connection logic in issue #9
  }
  ```
- Add three new methods to the `impl ToolRegistry` block:

  **`connect` method:**
  ```rust
  pub async fn connect(&self, name: &str) -> Result<(), RegistryError> {
      let endpoint = {
          let entries = self.entries.read().unwrap();
          let entry = entries.get(name).ok_or_else(|| RegistryError::ToolNotFound {
              name: name.to_string(),
          })?;
          entry.endpoint.clone()
      };
      // Lock is dropped here -- safe to .await

      let service = transport::connect_transport(&endpoint).await?;
      let mcp_handle = McpHandle::new(service);

      let mut entries = self.entries.write().unwrap();
      let entry = entries.get_mut(name).ok_or_else(|| RegistryError::ToolNotFound {
          name: name.to_string(),
      })?;
      entry.handle = Some(mcp_handle);
      Ok(())
  }
  ```

  **`connect_all` method:**
  ```rust
  pub async fn connect_all(&self) -> Result<(), RegistryError> {
      let names: Vec<String> = {
          let entries = self.entries.read().unwrap();
          entries.keys().cloned().collect()
      };

      for name in &names {
          self.connect(name).await?;
      }
      Ok(())
  }
  ```

  **`get_handle` method:**
  ```rust
  pub fn get_handle(&self, name: &str) -> Option<McpHandle> {
      let entries = self.entries.read().unwrap();
      entries.get(name).and_then(|entry| entry.handle.clone())
  }
  ```

**`crates/tool-registry/src/lib.rs`**

Add module declarations and re-export. The final file should look like:

```rust
mod mcp_handle;
mod registry_error;
mod tool_entry;
mod tool_registry;
mod transport;

pub use mcp_handle::McpHandle;
pub use registry_error::RegistryError;
pub use tool_entry::ToolEntry;
pub use tool_registry::ToolRegistry;
```

Note: `transport` is declared as `mod transport;` (not `pub mod`) and has no `pub use` re-export -- its functions are `pub(crate)` only.

### Key design decisions

- **Read lock released before `.await`**: `std::sync::RwLock` guards are `!Send`, so they cannot be held across `.await` points. The `connect` method uses a scoped block `{ ... }` to acquire the read lock, extract the endpoint, and drop the lock before calling `connect_transport().await`. This is critical for correctness.

- **Double lookup in `connect`**: The method looks up the entry once under a read lock (to get the endpoint) and again under a write lock (to store the handle). Between these two lookups, another thread could theoretically remove the entry. The second lookup handles this with a `ToolNotFound` error. This is the expected TOCTOU resolution.

- **`connect_all` is sequential**: Connections are made one at a time in a loop. Parallel connection (e.g., `futures::join_all`) is not used because: (a) it would require adding a `futures` dependency, and (b) sequential connection is simpler and sufficient for the initial implementation. Parallel connection can be added later if performance requires it.

- **`get_handle` returns a clone**: Since `McpHandle` derives `Clone` (wrapping `RunningService` which is cloneable), the method returns an owned clone. This is consistent with `ToolRegistry::get()` which also returns cloned values.

### Integration points

- **`transport::connect_transport`**: Called by `connect()` to establish the MCP session. This function is defined in the sibling "Create transport module" task.
- **`McpHandle::new`**: Constructor defined in the sibling "Define `McpHandle` newtype" task.
- **`ToolEntry.handle`**: The `Option<McpHandle>` field added by the sibling "Add `McpHandle` field to `ToolEntry`" task.
- **Downstream consumer**: The `connect()`, `connect_all()`, and `get_handle()` methods are used by the "Implement MCP-to-rig-core bridge in agent-runtime" task (Group 4) and the "Update agent-runtime main.rs" task.

## Dependencies

- **Blocked by:**
  - "Create transport module" -- `transport::connect_transport` must exist for `connect()` to call.
  - "Add `McpHandle` field to `ToolEntry`" -- `ToolEntry` must have the `handle: Option<McpHandle>` field for `connect()` to set it.
  - Transitively: "Define `McpHandle` newtype" and "Add `rmcp` and `tokio` dependencies" (both are prerequisites of the above).
- **Blocking:**
  - "Implement MCP-to-rig-core bridge in agent-runtime" -- needs `get_handle()` to retrieve connected handles for tool resolution.

## Risks & Edge Cases

1. **Lock not held across `.await`**: The `std::sync::RwLock` guard is `!Send` and will cause a compile error if held across an `.await` point. The implementation must use scoped blocks to drop the guard before any `.await`. This is the most likely source of compile errors during implementation -- the compiler will catch it, but the implementer should structure the code with explicit scoping from the start.

2. **Entry removed between read and write locks (TOCTOU)**: In `connect()`, the entry is looked up under a read lock (to get the endpoint), then again under a write lock (to store the handle). If another thread calls a hypothetical `remove()` or `register()` with the same name between these two operations, the second lookup could fail or find a different entry. The second `ok_or_else` handles the removal case. Registration of a new entry with the same name is prevented by `register()` returning `DuplicateEntry`. This is an acceptable race condition for the current design.

3. **Connection failure leaves entry without handle**: If `connect_transport` returns an error, the entry's `handle` remains `None` (or whatever it was before). This is correct behavior -- a failed connection should not leave a stale handle. The caller can retry by calling `connect()` again.

4. **`connect_all` partial failure**: If one connection fails, `connect_all` stops and returns the error. Entries that were connected before the failure retain their handles. This fail-fast behavior is consistent with `resolve_for_skill`. If the caller wants to connect as many as possible and collect errors, they can call `connect()` individually.

5. **Reconnection**: Calling `connect()` on an already-connected entry will overwrite the existing handle with a new one. The old `McpHandle` is dropped. If `McpHandle`'s `Drop` implementation (or lack thereof) does not gracefully shut down the old session, this could leak resources. The `McpHandle` design should ensure that dropping it cleans up the underlying `RunningService` (rmcp's `RunningService` shuts down on drop by default).

6. **`connect_all` order is non-deterministic**: Since `HashMap::keys()` does not guarantee order, the connection order is arbitrary. This is fine because connections are independent of each other.

7. **Async method on non-async struct**: `ToolRegistry` uses `std::sync::RwLock`, but `connect` and `connect_all` are `async fn`. This is intentional -- the async is needed for the network I/O in `connect_transport`, while `std::sync::RwLock` is used for the fast in-memory lookups. The caller must be in an async context (e.g., a tokio runtime) to call these methods.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p tool-registry
cargo clippy -p tool-registry
cargo test -p tool-registry
```

All must pass with no errors and no warnings. Additionally verify:

- The stub `connect(_url: &str)` static method no longer exists.
- `ToolRegistry::connect(&self, name: &str)` is async and returns `Result<(), RegistryError>`.
- `ToolRegistry::connect_all(&self)` is async and returns `Result<(), RegistryError>`.
- `ToolRegistry::get_handle(&self, name: &str)` returns `Option<McpHandle>`.
- `connect()` returns `RegistryError::ToolNotFound` when the named tool is not registered.
- `connect()` calls `transport::connect_transport` with the entry's endpoint and stores the resulting handle.
- The `std::sync::RwLock` guard is never held across an `.await` point.
- `lib.rs` declares `mod mcp_handle;` and `mod transport;`.
- `lib.rs` re-exports `McpHandle` via `pub use mcp_handle::McpHandle;`.
- `transport` is NOT publicly re-exported.
- All methods are under 50 lines.
- No test code, no commented-out code, no debug statements in the modified files.
- Full integration test verification is handled by the "Write MCP connection integration tests" task.
