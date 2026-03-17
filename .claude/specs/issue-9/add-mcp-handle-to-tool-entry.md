# Spec: Add `McpHandle` field to `ToolEntry`

> From: .claude/tasks/issue-9.md

## Objective

Add an optional `McpHandle` field to the `ToolEntry` struct so that each registered tool can hold a live MCP client session handle once connected. This bridges the static tool metadata (name, version, endpoint) with the runtime connection state, enabling `ToolRegistry` to manage both registration and active connections in a single data structure. The handle is runtime-only and must not participate in serialization or equality comparisons.

## Current State

- `ToolEntry` is defined in `crates/tool-registry/src/tool_entry.rs` (created by issue #8, spec `define-tool-entry-struct.md`) with three `String` fields: `name`, `version`, and `endpoint`.
- `ToolEntry` derives `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.
- `ToolEntry` is re-exported from `crates/tool-registry/src/lib.rs` via `pub use tool_entry::ToolEntry;`.
- Unit tests in `crates/tool-registry/tests/tool_entry_test.rs` (created by issue #8) construct `ToolEntry` values with the three fields and assert on equality, serialization round-trips, and clone behavior.
- `McpHandle` does not yet exist. It will be defined in `crates/tool-registry/src/mcp_handle.rs` by the parallel task "Define `McpHandle` newtype wrapping the rmcp client session" (issue #9, Group 1). It wraps `RunningService<RoleClient, ()>` from `rmcp`, derives `Clone`, and does NOT derive `Serialize`, `Deserialize`, or `PartialEq`.
- The `rmcp` and `tokio` dependencies will be added to `tool-registry/Cargo.toml` by the parallel task "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" (issue #9, Group 1).

## Requirements

- Add a new field `handle: Option<McpHandle>` to the `ToolEntry` struct.
- Annotate the `handle` field with `#[serde(skip)]` so it is excluded from serialization and deserialization. When deserialized, the field defaults to `None`.
- Remove `PartialEq` from the derive macro and replace it with a manual `impl PartialEq for ToolEntry` that compares only `name`, `version`, and `endpoint` -- the `handle` field is excluded because `McpHandle` (which wraps an rmcp `RunningService`) does not implement `PartialEq`.
- Keep `JsonSchema` in the derive stack. Since `handle` is `Option<McpHandle>` and `McpHandle` does not implement `JsonSchema`, annotate the field with `#[schemars(skip)]` to exclude it from the generated schema.
- Update all existing test call sites in `crates/tool-registry/tests/tool_entry_test.rs` that construct `ToolEntry` values to include `handle: None`.
- Import `McpHandle` from the sibling module: `use crate::mcp_handle::McpHandle;`.
- Ensure the `mcp_handle` module is declared in `lib.rs` (this may already be handled by the "Define `McpHandle` newtype" task; coordinate to avoid duplicate declarations).

## Implementation Details

### File to modify: `crates/tool-registry/src/tool_entry.rs`

1. **Add import** for the `McpHandle` type:
   ```rust
   use crate::mcp_handle::McpHandle;
   ```

2. **Update derive macro** -- remove `PartialEq` from the derive list:
   ```rust
   #[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
   ```

3. **Add the new field** to `ToolEntry` with skip annotations:
   ```rust
   pub struct ToolEntry {
       pub name: String,
       pub version: String,
       pub endpoint: String,
       #[serde(skip)]
       #[schemars(skip)]
       pub handle: Option<McpHandle>,
   }
   ```

4. **Add manual `PartialEq` implementation** below the struct definition:
   ```rust
   impl PartialEq for ToolEntry {
       fn eq(&self, other: &Self) -> bool {
           self.name == other.name
               && self.version == other.version
               && self.endpoint == other.endpoint
       }
   }
   ```
   This implementation deliberately excludes `handle` from equality comparison, so two `ToolEntry` values with the same name/version/endpoint are equal regardless of whether one has a connected handle and the other does not.

### File to modify: `crates/tool-registry/tests/tool_entry_test.rs`

Update every `ToolEntry { name: ..., version: ..., endpoint: ... }` construction to include `handle: None`. This affects all `ToolEntry` tests:
- `tool_entry_json_round_trip`
- `tool_entry_json_round_trip_with_unix_socket_endpoint`
- `tool_entry_equality_with_matching_entries`
- `tool_entry_inequality_differs_by_name`
- `tool_entry_inequality_differs_by_version`
- `tool_entry_inequality_differs_by_endpoint`
- `tool_entry_clone_produces_equal_value`

The tests should NOT need to import `McpHandle` -- they use `handle: None` which only requires the `Option` type.

### File to verify: `crates/tool-registry/src/lib.rs`

Ensure `mod mcp_handle;` is declared. If the "Define `McpHandle` newtype" task already adds this declaration, no change is needed. If not, add it. The `McpHandle` type should be re-exported: `pub use mcp_handle::McpHandle;`.

### Integration points

- `ToolRegistry::register()` will construct `ToolEntry` values with `handle: None` initially.
- `ToolRegistry::connect()` (Group 3 task) will set `entry.handle = Some(mcp_handle)` after establishing a connection.
- `ToolRegistry::get_handle()` (Group 3 task) will read `entry.handle.as_ref()` to expose the handle.
- Serialization round-trips will continue to work: `handle` is skipped on serialize (omitted from JSON) and defaults to `None` on deserialize.
- The manual `PartialEq` preserves existing test assertions: two entries with the same name/version/endpoint are equal, regardless of handle state.

## Dependencies

- Blocked by:
  - "Define `McpHandle` newtype wrapping the rmcp client session" -- `McpHandle` must exist before it can be used as a field type.
  - "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" -- `rmcp` must be in `Cargo.toml` for `McpHandle` (and transitively this file) to compile.
- Blocking:
  - "Implement `connect()` with real MCP client logic" -- `connect()` writes to the `handle` field added by this task.

## Risks & Edge Cases

- **`#[schemars(skip)]` compatibility**: The `schemars` crate supports `#[schemars(skip)]` on fields to exclude them from the generated JSON Schema. If for some reason this attribute is not available in `schemars 0.8`, an alternative is to wrap the field type in a newtype that implements `JsonSchema` trivially, or to switch to a manual `JsonSchema` implementation. However, `#[schemars(skip)]` has been supported since `schemars 0.7` and should work.

- **`#[serde(skip)]` deserialization default**: When `#[serde(skip)]` is applied to a field, serde uses `Default::default()` for that field during deserialization. `Option<McpHandle>` defaults to `None`, which is the correct behavior. No explicit `#[serde(default)]` is needed.

- **`Eq` not derived**: The original `ToolEntry` derives `PartialEq` but not `Eq`. The manual `PartialEq` maintains this -- `Eq` is not implemented. If `Eq` is ever needed (e.g., for use as a `HashSet` element), it can be added as a manual impl alongside `PartialEq` since the comparison fields are all `String` (which is `Eq`).

- **Test construction verbosity**: Adding `handle: None` to every test construction site is verbose but necessary because Rust struct literals require all fields. An alternative would be adding a `ToolEntry::new(name, version, endpoint)` constructor that sets `handle: None`, but that is outside the scope of this task and can be added later if desired.

- **Module declaration coordination**: Both this task and the "Define `McpHandle` newtype" task touch `lib.rs` (one adds `mod mcp_handle;`, the other may need to verify it exists). Ensure only one task adds the module declaration. The "Define `McpHandle` newtype" task is the natural owner of that declaration since it creates the file.

## Verification

- `cargo check -p tool-registry` compiles without errors after both blocking tasks are complete.
- `cargo clippy -p tool-registry -- -D warnings` produces no warnings.
- `cargo test -p tool-registry` passes all existing tests (updated with `handle: None`).
- Construct a `ToolEntry` with `handle: None`, serialize to JSON with `serde_json::to_string`, and confirm the JSON output does NOT contain a `"handle"` key.
- Deserialize a JSON string `{"name":"x","version":"1","endpoint":"e"}` (no `handle` key) into a `ToolEntry` and confirm `handle` is `None`.
- Construct two `ToolEntry` values with identical name/version/endpoint but different `handle` values (one `None`, one would be `Some` in a real scenario), and confirm `==` returns `true` -- verifying the manual `PartialEq` excludes `handle`.
- The `JsonSchema` output for `ToolEntry` does not include a `handle` property (verified by `schemars::schema_for!(ToolEntry)` if desired).
