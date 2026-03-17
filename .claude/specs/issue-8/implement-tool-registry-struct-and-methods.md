# Spec: Implement `ToolRegistry` struct and methods

> From: .claude/tasks/issue-8.md

## Objective

Create the `ToolRegistry` struct and its core methods in `crates/tool-registry/src/tool_registry.rs`. This is the central data structure of the `tool-registry` crate: a thread-safe, in-memory store that maps tool names to `ToolEntry` values. It supports registration, existence checks, single lookups, and bulk resolution of all tools required by a `SkillManifest`. A stub `connect` method is included as a placeholder for future MCP integration (issue #9).

## Current State

- `crates/tool-registry/src/lib.rs` contains only a placeholder unit struct: `pub struct ToolRegistry;`. No fields, no methods, no module declarations.
- `crates/tool-registry/Cargo.toml` has an empty `[dependencies]` section (the "Add dependencies" task in Group 1 will populate it with `serde`, `schemars`, `serde_json`, and `agent-sdk`).
- `ToolEntry` and `RegistryError` do not exist yet; they are defined by sibling tasks in Group 1 (`tool_entry.rs` and `registry_error.rs`).
- The `SkillManifest` struct is defined in `crates/agent-sdk/src/skill_manifest.rs` and re-exported from `agent_sdk`. Its `tools` field is `Vec<String>`, where each string is a tool name.
- The `ToolExists` trait is currently defined in `crates/skill-loader/src/validation.rs` with the method `fn tool_exists(&self, name: &str) -> bool`. The `ToolRegistry` does not implement this trait in this task -- that is handled by a separate task in Group 3.
- The project uses `std::sync::RwLock` (not `tokio::sync::RwLock`) because registry operations are fast in-memory lookups that do not need to be held across `.await` points, and this avoids requiring `tokio` as a dependency.
- Error types follow a manual `impl Display + Error` pattern (no `thiserror`), as seen in `crates/agent-sdk/src/agent_error.rs`.
- The codebase follows a one-type-per-file convention with module declarations and `pub use` re-exports in `lib.rs` (see `crates/agent-sdk/src/lib.rs`).

## Requirements

1. Create a new file `crates/tool-registry/src/tool_registry.rs`.
2. Define a public struct `ToolRegistry` with a single private field:
   - `entries: Arc<RwLock<HashMap<String, ToolEntry>>>` -- the thread-safe map from tool name to entry.
3. Implement `ToolRegistry::new() -> Self` that creates an empty registry by initializing `entries` with `Arc::new(RwLock::new(HashMap::new()))`.
4. Implement `ToolRegistry::register(&self, entry: ToolEntry) -> Result<(), RegistryError>`:
   - Acquires a write lock on `entries`.
   - Checks whether the map already contains a key matching `entry.name`.
   - If a duplicate exists, returns `Err(RegistryError::DuplicateEntry { name: entry.name })`.
   - Otherwise, inserts the entry keyed by its name (clone the name for the key) and returns `Ok(())`.
5. Implement `ToolRegistry::assert_exists(&self, name: &str) -> Result<(), RegistryError>`:
   - Acquires a read lock on `entries`.
   - If the name is not present in the map, returns `Err(RegistryError::ToolNotFound { name: name.to_string() })`.
   - Otherwise returns `Ok(())`.
6. Implement `ToolRegistry::resolve_for_skill(&self, manifest: &SkillManifest) -> Result<Vec<ToolEntry>, RegistryError>`:
   - Acquires a read lock on `entries` once.
   - Iterates over `manifest.tools` (a `Vec<String>`).
   - For each tool name, looks it up in the locked map. If not found, returns `Err(RegistryError::ToolNotFound)` immediately (fail-fast on first missing tool).
   - Collects cloned `ToolEntry` values for all matching tools into a `Vec<ToolEntry>`.
   - Returns `Ok(vec)`.
   - Note: this method should acquire the lock once and do the iteration internally, rather than calling `assert_exists` per tool (which would acquire/release the read lock repeatedly). This is both more efficient and avoids any TOCTOU issues.
7. Implement `ToolRegistry::connect(_url: &str)`:
   - A static method (no `&self`) that takes a `&str` URL parameter.
   - Body is empty except for a comment: `// TODO: real MCP connection logic in issue #9`.
   - No return value (returns `()`).
8. Implement `ToolRegistry::get(&self, name: &str) -> Option<ToolEntry>`:
   - Acquires a read lock on `entries`.
   - Returns `Some(entry.clone())` if found, `None` otherwise.
9. Every method must be under 50 lines (per project rules in `.claude/rules/general.md`).
10. No `#[derive]` macros on `ToolRegistry` itself -- it is not a data-transfer type and the `Arc<RwLock<...>>` field does not support `Serialize`/`Deserialize`.
11. No tests in this file. Tests are handled by the "Write integration tests" task in Group 5.

## Implementation Details

- **File to create:** `crates/tool-registry/src/tool_registry.rs`
- **Imports needed at the top of the file:**
  ```rust
  use std::collections::HashMap;
  use std::sync::{Arc, RwLock};

  use agent_sdk::SkillManifest;

  use crate::registry_error::RegistryError;
  use crate::tool_entry::ToolEntry;
  ```
  Note: use `crate::` imports for sibling modules (`RegistryError`, `ToolEntry`) since this file lives inside the `tool-registry` crate. Use `agent_sdk::` for the cross-crate import.

- **Struct definition:**
  ```rust
  pub struct ToolRegistry {
      entries: Arc<RwLock<HashMap<String, ToolEntry>>>,
  }
  ```

- **Method signatures:**
  ```rust
  impl ToolRegistry {
      pub fn new() -> Self
      pub fn register(&self, entry: ToolEntry) -> Result<(), RegistryError>
      pub fn assert_exists(&self, name: &str) -> Result<(), RegistryError>
      pub fn resolve_for_skill(&self, manifest: &SkillManifest) -> Result<Vec<ToolEntry>, RegistryError>
      pub fn connect(_url: &str)
      pub fn get(&self, name: &str) -> Option<ToolEntry>
  }
  ```

- **Lock handling:** All methods use `.read().unwrap()` or `.write().unwrap()` on the `RwLock`. Panicking on a poisoned lock is the standard Rust approach for `std::sync::RwLock` -- these locks only become poisoned if a thread panics while holding the lock, which indicates a bug. No custom error recovery for poisoned locks is needed.

- **`register` key extraction:** The `ToolEntry` struct has a `name: String` field. The map key should be `entry.name.clone()` and then the entry is inserted as the value. Alternatively, the name can be extracted before moving the entry: `let name = entry.name.clone(); map.insert(name, entry);`.

- **`resolve_for_skill` design choice:** Rather than calling `self.assert_exists()` per tool (which re-acquires the lock each time), this method acquires the read lock once, iterates `manifest.tools`, and does map lookups directly on the locked `HashMap`. This avoids N lock acquisitions and keeps the method simple.

- **Integration with `SkillManifest`:** The `tools` field on `SkillManifest` is `Vec<String>`. The `resolve_for_skill` method iterates this vector and uses each string as a lookup key. The returned `Vec<ToolEntry>` preserves the order from the manifest.

- **No async:** None of these methods are async. `std::sync::RwLock` is used intentionally to keep the crate dependency-light (no `tokio` needed).

- **This file is NOT wired into `lib.rs` by this task.** The "Wire up `lib.rs`" task in Group 3 handles adding `mod tool_registry;` and `pub use tool_registry::ToolRegistry;` to `lib.rs`.

## Dependencies

- **Blocked by:**
  - "Define `ToolEntry` struct" (Group 1) -- `crates/tool-registry/src/tool_entry.rs` must exist with a `ToolEntry` struct that has a `name: String` field and derives `Clone`.
  - "Define `RegistryError` enum" (Group 1) -- `crates/tool-registry/src/registry_error.rs` must exist with `RegistryError::ToolNotFound { name: String }` and `RegistryError::DuplicateEntry { name: String }` variants.
  - "Add dependencies to tool-registry Cargo.toml" (Group 1) -- `agent-sdk` must be listed as a dependency for the `SkillManifest` import to resolve.
- **Blocking:**
  - "Implement `ToolExists` trait for `ToolRegistry`" (Group 3) -- needs the `assert_exists` method to delegate to.
  - "Wire up `lib.rs`" (Group 3) -- needs this file to exist so it can add `mod tool_registry;` and `pub use`.

## Risks & Edge Cases

1. **Poisoned lock.** If a thread panics while holding a write lock, all subsequent `read()` and `write()` calls will return `PoisonError`. The methods use `.unwrap()`, which will propagate the panic. This is intentional -- a poisoned lock indicates a bug elsewhere in the program, and there is no meaningful recovery. If the project later requires graceful handling, the `.unwrap()` calls can be replaced with `.unwrap_or_else(|e| e.into_inner())`.

2. **`resolve_for_skill` with duplicate tool names in manifest.** If `manifest.tools` contains the same tool name twice, the returned `Vec<ToolEntry>` will contain duplicate entries. This is not harmful (the caller gets what was requested), and manifest validation (in `skill-loader`) is the appropriate place to prevent duplicate tool names if desired.

3. **`connect` stub.** The `connect` method is a no-op stub. It is a static method (no `&self`), so calling it has no side effects. Callers should not rely on any behavior from it until issue #9 provides a real implementation.

4. **Ordering in `resolve_for_skill`.** The returned `Vec<ToolEntry>` preserves the order of `manifest.tools`, which is the order the skill author declared. This is a natural consequence of iterating the `Vec<String>` in order.

5. **HashMap key vs. ToolEntry.name consistency.** The `register` method uses `entry.name.clone()` as the key. There is no mechanism to prevent a `ToolEntry` from being mutated after insertion (since it is cloned into the map). This is fine because `HashMap` values are independent copies. However, the key and the stored entry's `name` field are guaranteed to match at insertion time.

6. **Thread safety for downstream use.** `ToolRegistry` itself is `Send + Sync` because `Arc<RwLock<T>>` is `Send + Sync` when `T: Send + Sync`, and `HashMap<String, ToolEntry>` satisfies this (assuming `ToolEntry` fields are all `Send + Sync`, which they are as `String` types). This means `ToolRegistry` can be shared via `Arc<ToolRegistry>` across threads and async tasks, matching the existing usage pattern in `SkillLoader`.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p tool-registry
cargo clippy -p tool-registry
```

Both must pass with no errors and no warnings. Additionally verify:

- The file `crates/tool-registry/src/tool_registry.rs` exists and compiles.
- `ToolRegistry` has exactly one field: `entries: Arc<RwLock<HashMap<String, ToolEntry>>>`.
- `ToolRegistry::new()` returns a `ToolRegistry` with an empty map.
- `ToolRegistry::register` returns `Err(RegistryError::DuplicateEntry)` on duplicate names and `Ok(())` on success.
- `ToolRegistry::assert_exists` returns `Err(RegistryError::ToolNotFound)` for absent names and `Ok(())` for present names.
- `ToolRegistry::resolve_for_skill` returns all matching entries or fails on the first missing tool.
- `ToolRegistry::connect` compiles and does nothing.
- `ToolRegistry::get` returns `Some(entry)` for present names and `None` for absent names.
- All methods are under 50 lines.
- No test code, no commented-out code, no debug statements in the file.
- Full test verification is handled by the "Write integration tests" task in Group 5.
