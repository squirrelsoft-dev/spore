# Spec: Wire up `lib.rs` with module declarations and re-exports

> From: .claude/tasks/issue-8.md

## Objective

Replace the placeholder unit struct `pub struct ToolRegistry;` in `crates/tool-registry/src/lib.rs` with proper module declarations and public re-exports, establishing the crate's public API surface. This wires up the internal modules (`tool_entry`, `registry_error`, `tool_registry`) so downstream crates can import types via `tool_registry::ToolEntry`, `tool_registry::RegistryError`, and `tool_registry::ToolRegistry`.

## Current State

`crates/tool-registry/src/lib.rs` contains only:

```rust
pub struct ToolRegistry;
```

This is a placeholder unit struct that allows dependent crates (e.g., `skill-loader`) to compile. Once the real `ToolRegistry` struct, `ToolEntry`, and `RegistryError` types exist in their own submodules, this placeholder must be replaced with module declarations and re-exports.

The project already follows a clear pattern for this in `crates/agent-sdk/src/lib.rs`, which declares private `mod` items and then selectively re-exports public types via `pub use`.

## Requirements

- Remove the placeholder `pub struct ToolRegistry;` entirely from `lib.rs`.
- Declare three private modules: `tool_entry`, `registry_error`, `tool_registry`.
- Re-export the following types at crate root:
  - `ToolEntry` from `tool_entry`
  - `RegistryError` from `registry_error`
  - `ToolRegistry` from `tool_registry`
- If the `ToolExists` trait has been moved into this crate (as described in the sibling task "Implement `ToolExists` trait for `ToolRegistry`"), also re-export it. The exact module it lives in depends on the sibling task's implementation -- it may be in `tool_registry.rs` or in a dedicated `tool_exists.rs` module. In either case, add `pub use <module>::ToolExists;`.
- Keep modules private (`mod`, not `pub mod`) so that only explicitly re-exported symbols are part of the public API.
- No other logic, imports, or code should be present in `lib.rs` beyond module declarations and `pub use` re-exports.

## Implementation Details

### File to modify

**`crates/tool-registry/src/lib.rs`**

Replace the entire file contents with:

```rust
mod tool_entry;
mod registry_error;
mod tool_registry;

pub use tool_entry::ToolEntry;
pub use registry_error::RegistryError;
pub use tool_registry::ToolRegistry;
```

If the `ToolExists` trait is defined within this crate (determined by the sibling task), append:

```rust
pub use tool_registry::ToolExists;
```

(or `pub use tool_exists::ToolExists;` if it lives in a dedicated module).

### Pattern to follow

The module and re-export structure mirrors `crates/agent-sdk/src/lib.rs`:

- Private `mod` declarations group related types into separate files.
- `pub use` re-exports expose only the types that form the crate's public API.
- No `pub mod` -- internal module structure is an implementation detail.

### No new files

This task does not create any new files. It only modifies `lib.rs`. The submodule files (`tool_entry.rs`, `registry_error.rs`, `tool_registry.rs`) are created by their respective sibling tasks.

## Dependencies

- Blocked by: "Implement `ToolRegistry` struct and methods" -- the submodule files (`tool_entry.rs`, `registry_error.rs`, `tool_registry.rs`) must exist before `lib.rs` can declare them as modules.
- Blocking: "Update `skill-loader` to use real `ToolRegistry` methods" -- downstream crates need the public re-exports to import the real types.

## Risks & Edge Cases

- **Conditional `ToolExists` re-export**: The sibling task "Implement `ToolExists` trait for `ToolRegistry`" may or may not move `ToolExists` into this crate. The implementer must check whether `ToolExists` is defined in any of the submodules before adding a `pub use` for it. If it is not present, omit the re-export -- adding a `pub use` for a nonexistent item will cause a compile error.
- **Compilation order**: This task and the `ToolExists` trait task are both in Group 3 and listed as parallelizable. However, if they are implemented in separate commits, the implementer of this task should coordinate with the `ToolExists` task to ensure the final `lib.rs` includes all necessary re-exports.
- **Breaking the placeholder**: Removing `pub struct ToolRegistry;` will break any code that constructs `ToolRegistry` as a unit struct (e.g., `Arc::new(ToolRegistry)`). This is expected and is handled by the Group 4 task "Update `skill-loader` to use real `ToolRegistry` methods". The implementer should not attempt to maintain backward compatibility with the unit struct.

## Verification

- `cargo check -p tool-registry` compiles without errors (requires submodule files to exist).
- `cargo check` across the full workspace compiles without errors (requires Group 4 downstream updates to also be applied).
- The following types are importable from the crate root in downstream code: `tool_registry::ToolEntry`, `tool_registry::RegistryError`, `tool_registry::ToolRegistry`.
- If `ToolExists` was moved, `tool_registry::ToolExists` is also importable.
- `cargo clippy -p tool-registry` produces no warnings.
- No commented-out code, no debug statements, no placeholder types remain in `lib.rs`.
