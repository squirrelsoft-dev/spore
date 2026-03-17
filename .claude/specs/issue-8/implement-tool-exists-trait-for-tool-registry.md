# Spec: Implement `ToolExists` trait for `ToolRegistry`

> From: .claude/tasks/issue-8.md

## Objective

Move the `ToolExists` trait definition from `skill-loader` to `tool-registry` so that `ToolRegistry` can implement it directly, without introducing a circular dependency. Re-export the trait from `skill-loader` so that all existing downstream code continues to compile unchanged.

## Current State

- **`ToolExists` trait** is defined in `crates/skill-loader/src/validation.rs` (lines 5-11). It has a single method: `fn tool_exists(&self, name: &str) -> bool`.
- **`AllToolsExist` stub** is defined in the same file (lines 13-24). It implements `ToolExists` and always returns `true`. Used extensively in tests.
- **`skill-loader/src/lib.rs`** publicly re-exports both: `pub use validation::{AllToolsExist, ToolExists, validate};`.
- **`tool-registry/src/lib.rs`** currently contains only a placeholder unit struct: `pub struct ToolRegistry;`.
- **`skill-loader` depends on `tool-registry`** (see `skill-loader/Cargo.toml` line 10). The reverse dependency does not exist and must not be introduced.
- **`SkillLoader`** stores a `tool_checker: Box<dyn ToolExists + Send + Sync>` and uses it in the `load` method via `validate(&manifest, &*self.tool_checker)`.
- **Downstream consumers** import `ToolExists` from `skill_loader` in three test files:
  - `crates/skill-loader/tests/validation_test.rs` (line 4): imports `ToolExists` directly, defines a custom `RejectTools` impl.
  - `crates/skill-loader/tests/skill_loader_test.rs` (line 3): imports `AllToolsExist`.
  - `crates/skill-loader/tests/validation_integration_test.rs` (line 3): imports `AllToolsExist`.

## Requirements

- The `ToolExists` trait must be defined in `crates/tool-registry/src/lib.rs` (or a submodule re-exported from `lib.rs`).
- `ToolRegistry` must implement `ToolExists` by delegating to its `assert_exists` method: return `true` if `assert_exists` returns `Ok(())`, `false` if it returns `Err`.
- `skill-loader` must re-export `ToolExists` from `tool-registry` so that `skill_loader::ToolExists` continues to resolve. The re-export path: `pub use tool_registry::ToolExists;` in `skill-loader/src/lib.rs`.
- The `AllToolsExist` stub must remain in `skill-loader/src/validation.rs`. It is a test helper that does not belong in the registry crate.
- `validation.rs` must import `ToolExists` from `tool_registry` instead of defining it locally.
- The `validate` function signature (`&dyn ToolExists`) must not change.
- All existing tests in `skill-loader` must continue to pass without modification (the re-export preserves the import path).
- `tool-registry` must NOT depend on `skill-loader` (no circular dependency).

## Implementation Details

### Files to modify

1. **`crates/tool-registry/src/lib.rs`**
   - Add the `ToolExists` trait definition (moved from `validation.rs`). Keep the same doc comment and method signature.
   - When the "Wire up `lib.rs`" task is also done, this file will contain module declarations and re-exports. The trait can live directly in `lib.rs` or in a dedicated submodule (e.g., `tool_exists.rs`); placing it in `lib.rs` is simplest since it is a single 3-line trait.
   - The `impl ToolExists for ToolRegistry` block belongs in `tool_registry.rs` (alongside the struct), but note: that file does not exist yet (it is created by the predecessor task "Implement `ToolRegistry` struct and methods"). The trait definition itself goes in `lib.rs` so it is available to all modules in the crate.

2. **`crates/tool-registry/src/tool_registry.rs`** (created by predecessor task)
   - Add `impl ToolExists for ToolRegistry`:
     ```rust
     impl ToolExists for ToolRegistry {
         fn tool_exists(&self, name: &str) -> bool {
             self.assert_exists(name).is_ok()
         }
     }
     ```
   - Import `ToolExists` from `crate::ToolExists` (or `super::ToolExists` depending on module structure).

3. **`crates/skill-loader/src/validation.rs`**
   - Remove the `ToolExists` trait definition (lines 5-11).
   - Add `use tool_registry::ToolExists;` at the top.
   - Keep `AllToolsExist` struct and its `impl ToolExists for AllToolsExist` block (which now refers to the imported trait).
   - Keep the `validate` function and all helper functions unchanged.
   - Keep the `#[cfg(test)] mod tests` block unchanged (the `AllToolsExist` tests still exercise the trait through the stub).

4. **`crates/skill-loader/src/lib.rs`**
   - Change `pub use validation::{AllToolsExist, ToolExists, validate};` to:
     ```rust
     pub use tool_registry::ToolExists;
     pub use validation::{AllToolsExist, validate};
     ```
   - This preserves the `skill_loader::ToolExists` import path for all downstream consumers.

### Key types and interfaces

- **`ToolExists` trait** (unchanged signature):
  ```rust
  pub trait ToolExists {
      fn tool_exists(&self, name: &str) -> bool;
  }
  ```
- **`impl ToolExists for ToolRegistry`**: delegates to `self.assert_exists(name).is_ok()`.

### Integration points

- The `validate` function in `skill-loader` accepts `&dyn ToolExists`. After this change, a `&ToolRegistry` can be passed directly (instead of wrapping in a separate `Box<dyn ToolExists>`).
- `SkillLoader::new` currently takes `tool_checker: Box<dyn ToolExists + Send + Sync>` as a separate parameter. A future task ("Update `skill-loader` to use real `ToolRegistry` methods") may simplify this by using the registry itself as the checker.

## Dependencies

- **Blocked by:** "Implement `ToolRegistry` struct and methods" -- the `impl ToolExists for ToolRegistry` block requires the `assert_exists` method to exist on `ToolRegistry`.
- **Blocking:** "Update `skill-loader` to use real `ToolRegistry` methods" -- that task will update test files to use `ToolRegistry::new()` and may pass the registry directly as the tool checker.

## Risks & Edge Cases

- **Re-export path breakage:** If the re-export from `skill-loader/src/lib.rs` is missed, all three test files and any external consumers will fail to compile. The re-export `pub use tool_registry::ToolExists;` must be present.
- **`AllToolsExist` depends on trait import:** After removing the local trait definition from `validation.rs`, `AllToolsExist`'s `impl ToolExists` block depends on the `use tool_registry::ToolExists;` import. If that import is missing, compilation fails.
- **Object safety:** `ToolExists` is used as `&dyn ToolExists` and `Box<dyn ToolExists + Send + Sync>`. The trait must remain object-safe (no `Self: Sized` bounds, no generics). The current single-method signature is object-safe and must not change.
- **`Send + Sync` bounds:** `ToolRegistry` uses `Arc<RwLock<HashMap<...>>>` internally (per the predecessor task spec). `Arc<RwLock<T>>` is `Send + Sync` when `T: Send + Sync`, which `HashMap<String, ToolEntry>` satisfies. So `ToolRegistry` will be `Send + Sync`, satisfying `Box<dyn ToolExists + Send + Sync>`.
- **No new dependencies:** `tool-registry` does not need any new crate dependencies for the trait definition. `skill-loader` already depends on `tool-registry`.

## Verification

1. `cargo check` passes with no errors across the workspace.
2. `cargo clippy` produces no warnings.
3. `cargo test -p tool-registry` passes, including any new test for the `ToolExists` impl (e.g., the `tool_exists_trait_impl` test described in the issue-8 task breakdown under Group 5).
4. `cargo test -p skill-loader` passes -- all 20+ existing tests continue to work, confirming the re-export preserves backward compatibility.
5. Confirm that `tool-registry/Cargo.toml` does NOT list `skill-loader` as a dependency (no circular dependency).
6. Confirm that `skill_loader::ToolExists` resolves correctly by verifying `crates/skill-loader/tests/validation_test.rs` compiles (it imports `ToolExists` from `skill_loader` and implements it on `RejectTools`).
