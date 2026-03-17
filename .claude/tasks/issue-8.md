# Task Breakdown: Create tool-registry crate with ToolEntry storage

> Implement the `tool-registry` crate with a thread-safe `ToolRegistry` that stores `ToolEntry` mappings (tool name to MCP endpoint), supports registration and lookup, resolves tools for a skill manifest, and implements the `ToolExists` trait from `skill-loader`.

## Group 1 — Dependencies and types

_Tasks in this group can be done in parallel._

- [x] **Add dependencies to tool-registry Cargo.toml** `[S]`
      Add the following dependencies to `crates/tool-registry/Cargo.toml`: `serde = { version = "1", features = ["derive"] }`, `schemars = { version = "0.8", features = ["derive", "uuid1"] }`, `serde_json = "1"`, `agent-sdk = { path = "../agent-sdk" }`. These match the dependency versions and patterns used in `crates/agent-sdk/Cargo.toml`. Note: `tokio`, `async-trait`, and `rmcp` are NOT needed yet — `connect()` is a stub and real MCP integration is deferred to issue #9.
      Files: `crates/tool-registry/Cargo.toml`
      Blocking: All tasks in Group 2

- [x] **Define `ToolEntry` struct** `[S]`
      Create `crates/tool-registry/src/tool_entry.rs` with a `ToolEntry` struct containing fields: `name: String`, `version: String`, `endpoint: String`. Use the derive stack from the codebase: `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`. The `endpoint` field stores the MCP endpoint URL (e.g., `"mcp://localhost:7001"` or a unix socket path). Omit the `handle: ToolServerHandle` field — that type does not exist yet and depends on `rmcp` integration in issue #9.
      Files: `crates/tool-registry/src/tool_entry.rs`
      Blocking: "Implement `ToolRegistry` struct and methods"

- [x] **Define `RegistryError` enum** `[S]`
      Create `crates/tool-registry/src/registry_error.rs` with a `RegistryError` enum with three variants: `ToolNotFound { name: String }`, `DuplicateEntry { name: String }`, `ConnectionFailed { endpoint: String, reason: String }`. Follow the manual `impl Display + Error` pattern from `crates/agent-sdk/src/agent_error.rs` (no `thiserror`). Display messages: `ToolNotFound` -> `"tool not found: '{name}'"`, `DuplicateEntry` -> `"duplicate tool entry: '{name}'"`, `ConnectionFailed` -> `"connection to '{endpoint}' failed: {reason}"`. Derive `Debug, Clone, PartialEq` on the enum.
      Files: `crates/tool-registry/src/registry_error.rs`
      Blocking: "Implement `ToolRegistry` struct and methods"

## Group 2 — Core implementation

_Depends on: Group 1._

- [x] **Implement `ToolRegistry` struct and methods** `[M]`
      Create `crates/tool-registry/src/tool_registry.rs` with:
      ```rust
      pub struct ToolRegistry {
          entries: Arc<RwLock<HashMap<String, ToolEntry>>>,
      }
      ```
      Implement methods (each under 50 lines):
      - `new() -> Self` — creates an empty registry with `Arc::new(RwLock::new(HashMap::new()))`
      - `register(&self, entry: ToolEntry) -> Result<(), RegistryError>` — acquires write lock, checks for duplicate by name, inserts entry. Returns `Err(RegistryError::DuplicateEntry)` if name already exists.
      - `assert_exists(&self, name: &str) -> Result<(), RegistryError>` — acquires read lock, returns `Err(RegistryError::ToolNotFound)` if name is not in the map.
      - `resolve_for_skill(&self, manifest: &SkillManifest) -> Result<Vec<ToolEntry>, RegistryError>` — iterates `manifest.tools`, calls `assert_exists` for each, collects matching `ToolEntry` clones. Returns error on first missing tool.
      - `connect(_url: &str)` — stub method with `// TODO: real MCP connection logic in issue #9`. No-op for now.
      - `get(&self, name: &str) -> Option<ToolEntry>` — convenience method: acquires read lock, returns cloned entry if found.

      Import `std::collections::HashMap`, `std::sync::{Arc, RwLock}`, `agent_sdk::SkillManifest`, `ToolEntry`, `RegistryError`.
      Files: `crates/tool-registry/src/tool_registry.rs`
      Blocked by: "Define `ToolEntry` struct", "Define `RegistryError` enum", "Add dependencies to tool-registry Cargo.toml"
      Blocking: "Implement `ToolExists` trait for `ToolRegistry`", "Wire up `lib.rs`"

## Group 3 — Trait implementation and public API

_Depends on: Group 2. Tasks in this group can be done in parallel._

- [x] **Implement `ToolExists` trait for `ToolRegistry`** `[S]`
      The `ToolExists` trait is currently defined in `skill-loader/src/validation.rs`. To implement it on `ToolRegistry` without a circular dependency (`skill-loader` depends on `tool-registry`), move the `ToolExists` trait definition to `tool-registry` and re-export it from `skill-loader` for backward compatibility. Then implement `ToolExists` for `ToolRegistry` directly by delegating to the `assert_exists` method. The `AllToolsExist` test stub should remain in `skill-loader`.
      Files: `crates/tool-registry/src/tool_registry.rs`, `crates/tool-registry/src/lib.rs`, `crates/skill-loader/src/validation.rs`, `crates/skill-loader/src/lib.rs`
      Blocked by: "Implement `ToolRegistry` struct and methods"
      Blocking: "Update `skill-loader` to use real `ToolRegistry` methods"

- [x] **Wire up `lib.rs` with module declarations and re-exports** `[S]`
      Replace the placeholder `pub struct ToolRegistry;` in `crates/tool-registry/src/lib.rs` with module declarations and public re-exports:
      ```rust
      mod tool_entry;
      mod registry_error;
      mod tool_registry;

      pub use tool_entry::ToolEntry;
      pub use registry_error::RegistryError;
      pub use tool_registry::ToolRegistry;
      ```
      If `ToolExists` trait is moved here, also add `pub use` for it. Follow the pattern from `crates/agent-sdk/src/lib.rs`.
      Files: `crates/tool-registry/src/lib.rs`
      Blocked by: "Implement `ToolRegistry` struct and methods"
      Blocking: "Update `skill-loader` to use real `ToolRegistry` methods"

## Group 4 — Downstream updates

_Depends on: Group 3._

- [x] **Update `skill-loader` to use real `ToolRegistry` methods** `[S]`
      The `skill-loader` crate currently constructs `Arc::new(ToolRegistry)` (unit struct). After `ToolRegistry` gains fields, this must change to `Arc::new(ToolRegistry::new())`. Update all occurrences:
      - `crates/skill-loader/tests/skill_loader_test.rs`: `Arc::new(ToolRegistry)` -> `Arc::new(ToolRegistry::new())`
      - `crates/skill-loader/tests/validation_integration_test.rs`: same change
      - If `ToolExists` trait was moved to `tool-registry`, update `skill-loader/src/validation.rs` to import from `tool_registry::ToolExists` instead of defining it locally (but keep `pub use` re-export from `skill-loader/src/lib.rs` for backward compat).
      Files: `crates/skill-loader/tests/skill_loader_test.rs`, `crates/skill-loader/tests/validation_integration_test.rs`, `crates/skill-loader/src/validation.rs`, `crates/skill-loader/src/lib.rs`
      Blocked by: "Wire up `lib.rs`", "Implement `ToolExists` trait for `ToolRegistry`"
      Blocking: "Write unit tests", "Write integration tests"

## Group 5 — Tests

_Depends on: Group 4. Tests can be done in parallel._

- [x] **Write unit tests for `ToolEntry` and `RegistryError`** `[S]`
      Add `#[cfg(test)] mod tests` blocks in `tool_entry.rs` and `registry_error.rs`, or create `crates/tool-registry/tests/tool_entry_test.rs`. Tests:
      - `ToolEntry` JSON serialization round-trip (following `tool_call_record` test pattern from `agent-sdk/tests/envelope_types_test.rs`)
      - `ToolEntry` equality check with matching and non-matching entries
      - `RegistryError::Display` output contains expected substrings for each variant (following `agent_error_display_contains_expected_substrings` pattern)
      Files: `crates/tool-registry/tests/tool_entry_test.rs` (or inline `#[cfg(test)]` modules)
      Blocked by: "Update `skill-loader` to use real `ToolRegistry` methods"
      Blocking: "Run verification suite"

- [x] **Write integration tests for `ToolRegistry` methods** `[M]`
      Create `crates/tool-registry/tests/tool_registry_test.rs` with tests:
      1. `register_and_get` — register an entry, verify `get()` returns it
      2. `assert_exists_returns_ok_for_registered_tool` — register, then `assert_exists` succeeds
      3. `assert_exists_returns_error_for_missing_tool` — call without registering, verify `RegistryError::ToolNotFound`
      4. `register_duplicate_returns_error` — register same name twice, verify `RegistryError::DuplicateEntry`
      5. `resolve_for_skill_returns_matching_entries` — register multiple tools, create a `SkillManifest` referencing a subset, verify `resolve_for_skill` returns exactly the referenced entries
      6. `resolve_for_skill_fails_on_missing_tool` — create manifest referencing unregistered tool, verify `ToolNotFound` error
      7. `get_returns_none_for_missing_tool` — verify `get()` returns `None`
      8. `tool_exists_trait_impl` — verify `ToolRegistry` implements `ToolExists`, returns `true` for registered and `false` for unregistered tools (use `&dyn ToolExists` to prove object safety)
      Files: `crates/tool-registry/tests/tool_registry_test.rs`
      Blocked by: "Update `skill-loader` to use real `ToolRegistry` methods"
      Blocking: "Run verification suite"

## Group 6 — Verification

_Depends on: Group 5._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the entire workspace. Ensure: no compiler errors, no clippy warnings, all existing tests still pass (especially `skill-loader` tests that depend on `ToolRegistry`), all new `tool-registry` tests pass.
      Blocked by: All previous tasks
      Blocking: None

---

## Design decisions and notes for implementers

1. **`ToolServerHandle` omitted**: The issue description includes `handle: ToolServerHandle` in `ToolEntry`, but this type depends on `rmcp` (issue #9). It is omitted from `ToolEntry` in this issue and will be added when MCP integration lands.

2. **`connect()` is a stub**: The method signature exists but performs no work. A TODO comment marks it for issue #9.

3. **`ToolExists` trait location**: The `ToolExists` trait is currently defined in `skill-loader/src/validation.rs`. To implement it on `ToolRegistry` without a circular dependency (skill-loader depends on tool-registry), the trait should be moved to `tool-registry` and re-exported from `skill-loader` for backward compatibility. The `AllToolsExist` test stub should remain in `skill-loader`.

4. **Thread safety**: `Arc<RwLock<HashMap<...>>>` uses `std::sync::RwLock` (not `tokio::sync::RwLock`) because registry operations are fast in-memory lookups that do not need to be held across `.await` points. This avoids requiring `tokio` as a dependency.

5. **Breaking change in `ToolRegistry` construction**: Changing from a unit struct to `ToolRegistry::new()` will break 2 test files in `skill-loader`. These must be updated in Group 4.

6. **No new external dependencies**: All dependencies (`serde`, `schemars`, `serde_json`) are already used in the workspace. `agent-sdk` is an internal path dependency. No new third-party crates are introduced.
