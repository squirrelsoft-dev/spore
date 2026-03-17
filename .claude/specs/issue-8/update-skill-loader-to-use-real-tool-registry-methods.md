# Spec: Update `skill-loader` to use real `ToolRegistry` methods

> From: .claude/tasks/issue-8.md

## Objective

After the `tool-registry` crate evolves from a unit struct (`pub struct ToolRegistry;`) to a proper struct with fields (`entries: Arc<RwLock<HashMap<String, ToolEntry>>>`), all downstream code that constructs `ToolRegistry` as a unit struct must be updated to use `ToolRegistry::new()`. Additionally, if the `ToolExists` trait has been moved from `skill-loader` to `tool-registry` (per the "Implement `ToolExists` trait for `ToolRegistry`" task), `skill-loader` must switch to importing it rather than defining it locally, while preserving backward-compatible re-exports.

## Current State

### `tool-registry/src/lib.rs`
Currently a single-line file exporting a unit struct:
```rust
pub struct ToolRegistry;
```

### `skill-loader/src/validation.rs`
Defines the `ToolExists` trait locally (lines 9-11) along with the `AllToolsExist` stub (lines 18-24). The `validate` function accepts `&dyn ToolExists` to check tool names.

### `skill-loader/src/lib.rs`
Re-exports `ToolExists`, `AllToolsExist`, and `validate` from the `validation` module. The `SkillLoader` struct holds `tool_registry: Arc<ToolRegistry>` and `tool_checker: Box<dyn ToolExists + Send + Sync>`.

### Test files
Both test files construct the registry identically:
```rust
let registry = Arc::new(ToolRegistry);
```
- `crates/skill-loader/tests/skill_loader_test.rs` (line 9)
- `crates/skill-loader/tests/validation_integration_test.rs` (line 9)

Both import `ToolRegistry` from `tool_registry` and `AllToolsExist` from `skill_loader`.

## Requirements

1. **Update `ToolRegistry` construction in test files**: Replace `Arc::new(ToolRegistry)` with `Arc::new(ToolRegistry::new())` in both:
   - `crates/skill-loader/tests/skill_loader_test.rs`
   - `crates/skill-loader/tests/validation_integration_test.rs`

2. **Update `ToolExists` trait import in `skill-loader/src/validation.rs`**: Remove the local `ToolExists` trait definition and replace it with an import from `tool_registry::ToolExists`. The `AllToolsExist` stub struct and its `impl ToolExists` block must remain in `skill-loader/src/validation.rs` (not moved to `tool-registry`).

3. **Preserve backward-compatible re-export in `skill-loader/src/lib.rs`**: The line `pub use validation::{AllToolsExist, ToolExists, validate};` must continue to work. After `ToolExists` is imported from `tool_registry` in `validation.rs`, the re-export in `lib.rs` must still expose `ToolExists` to downstream consumers of `skill-loader`. This may require changing to `pub use tool_registry::ToolExists;` in `lib.rs`, or the re-export from `validation` will naturally forward it.

4. **No functional behavior changes**: All existing `skill-loader` tests must continue to pass without modification to their test logic (only the `Arc::new(ToolRegistry)` -> `Arc::new(ToolRegistry::new())` construction change).

5. **No new dependencies**: No new crates should be added to `skill-loader/Cargo.toml`. The existing `tool-registry = { path = "../tool-registry" }` dependency is sufficient.

## Implementation Details

### File: `crates/skill-loader/tests/skill_loader_test.rs`
- **Line 9**: Change `let registry = Arc::new(ToolRegistry);` to `let registry = Arc::new(ToolRegistry::new());`
- No other changes needed. The `use tool_registry::ToolRegistry;` import (line 6) remains valid.

### File: `crates/skill-loader/tests/validation_integration_test.rs`
- **Line 9**: Change `let registry = Arc::new(ToolRegistry);` to `let registry = Arc::new(ToolRegistry::new());`
- No other changes needed. Same import remains valid.

### File: `crates/skill-loader/src/validation.rs`
- Remove the local `ToolExists` trait definition (lines 5-11):
  ```rust
  /// Trait for checking whether a tool name is registered.
  ///
  /// Used by the `validate` function to verify that all tool names
  /// referenced in a `SkillManifest` actually exist in the runtime.
  pub trait ToolExists {
      fn tool_exists(&self, name: &str) -> bool;
  }
  ```
- Add an import at the top: `use tool_registry::ToolExists;`
- Re-export it so downstream code (including `lib.rs`) can still reach it: `pub use tool_registry::ToolExists;`
- The `AllToolsExist` struct and its `impl ToolExists for AllToolsExist` block remain unchanged in this file.
- The `validate` function signature (`tool_checker: &dyn ToolExists`) remains unchanged.
- The `#[cfg(test)] mod tests` block at the bottom remains unchanged (it uses `super::*` which will pick up the re-exported trait).

### File: `crates/skill-loader/src/lib.rs`
- The existing line `pub use validation::{AllToolsExist, ToolExists, validate};` should continue to work because `validation.rs` will re-export `ToolExists` via `pub use tool_registry::ToolExists;`.
- If the compiler requires it, an alternative is to change to:
  ```rust
  pub use tool_registry::ToolExists;
  pub use validation::{AllToolsExist, validate};
  ```
  Either approach is acceptable as long as `skill_loader::ToolExists` resolves correctly for downstream consumers.

## Dependencies

- Blocked by: "Wire up `lib.rs`" (tool-registry must expose `ToolRegistry` with `new()` method), "Implement `ToolExists` trait for `ToolRegistry`" (trait must exist in `tool-registry` before `skill-loader` can import it)
- Blocking: "Write unit tests", "Write integration tests" (Group 5 tests depend on `skill-loader` compiling cleanly with the new `ToolRegistry`)

## Risks & Edge Cases

1. **Re-export chain ambiguity**: If both `validation.rs` and `lib.rs` try to re-export `ToolExists`, the compiler may flag a conflict. The simplest resolution is to have `validation.rs` do `pub use tool_registry::ToolExists;` and let `lib.rs` forward it via `pub use validation::ToolExists;` as it does today.

2. **`AllToolsExist` depends on `ToolExists` trait**: Since `AllToolsExist` remains in `skill-loader` but `ToolExists` moves to `tool-registry`, the `impl ToolExists for AllToolsExist` block in `validation.rs` must import the trait. This is handled by the `use tool_registry::ToolExists;` import.

3. **Circular dependency prevention**: `skill-loader` depends on `tool-registry`, so `ToolExists` cannot live in `skill-loader` if `tool-registry` needs to implement it. This task assumes the "Implement `ToolExists` trait for `ToolRegistry`" task has already moved the trait to `tool-registry`. If that task chose a different approach (e.g., keeping the trait in `skill-loader` and using `skill-loader` as a dev-dependency of `tool-registry`), this spec would need adjustment.

4. **`#[allow(dead_code)]` on `tool_registry` field**: The `SkillLoader` struct has `#[allow(dead_code)]` on the `tool_registry: Arc<ToolRegistry>` field. This annotation can remain for now since this task does not add runtime usage of the registry (it is still only used for construction/passing). Future tasks may remove this annotation when the field is actively used.

5. **Conditional handling**: The task description says "If `ToolExists` trait was moved to `tool-registry`". If the preceding task chose not to move the trait, then only the `Arc::new(ToolRegistry)` -> `Arc::new(ToolRegistry::new())` changes are needed, and `validation.rs` and `lib.rs` remain unchanged. The implementer should check the actual state of `tool-registry/src/lib.rs` to determine which path applies.

## Verification

1. **`cargo check -p skill-loader`** compiles without errors.
2. **`cargo check -p tool-registry`** compiles without errors.
3. **`cargo clippy -p skill-loader`** produces no warnings.
4. **`cargo test -p skill-loader`** -- all 12 existing tests pass (6 in `skill_loader_test.rs`, 7 in `validation_integration_test.rs`, plus inline unit tests in `validation.rs`).
5. **`cargo test`** -- full workspace test suite passes, confirming no regressions.
6. Confirm that `skill_loader::ToolExists` is still a valid import path (backward compat check -- verified by the existing test imports of `AllToolsExist` which depends on `ToolExists`).
