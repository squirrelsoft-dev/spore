# Spec: Define `ToolExists` trait in skill-loader

> From: .claude/tasks/issue-6.md

## Objective

Define a `ToolExists` trait in the `skill-loader` crate that abstracts over tool-name lookup, decoupling the validation logic from any concrete tool registry implementation. The `tool-registry` crate (issue #8) is currently a placeholder with no real types, so the `validate` function (a downstream task) will accept `&dyn ToolExists` instead of depending on a concrete `ToolRegistry`. This task also provides a `struct AllToolsExist` stub implementation that always returns `true`, enabling tests that exercise other validation rules without needing a real or mock tool registry.

## Current State

- **`crates/skill-loader/src/lib.rs`:** Skeleton file containing only a placeholder `add()` function and a trivial test. No real types, traits, or modules defined. No dependencies in `Cargo.toml` beyond the default.
- **`crates/tool-registry/src/lib.rs`:** Identical skeleton -- placeholder `add()` function and trivial test. No types exported. The crate is a pure placeholder for future work (issue #8).
- **`crates/skill-loader/Cargo.toml`:** Declares `edition = "2024"` with an empty `[dependencies]` section. Sibling tasks (issue #5) will add `serde`, `serde_yaml`, `agent-sdk`, etc., but this task requires no external dependencies.
- **`crates/agent-sdk/src/skill_manifest.rs`:** `SkillManifest` contains a `tools: Vec<String>` field. The `validate` function (downstream task) will iterate this vector and call `tool_exists(name)` for each entry.
- **Issue #5 spec for `SkillError`:** Defines a `ValidationError { skill_name: String, reasons: Vec<String> }` variant. Missing tool names discovered via `ToolExists` will be collected into the `reasons` vector by the downstream `validate` function.

## Requirements

1. **File location:** `crates/skill-loader/src/validation.rs` (new file). A dedicated module is preferred over adding to `lib.rs` because validation logic (the downstream `validate` function) will also live here, and grouping the trait with its primary consumer keeps the module cohesive.

2. **Trait definition:** A public trait named `ToolExists` with a single method:
   ```rust
   pub trait ToolExists {
       fn tool_exists(&self, name: &str) -> bool;
   }
   ```
   - The method takes `&self` (not `&mut self`) because lookup is a read-only operation.
   - The method takes `name: &str` (not `String` or `&String`) following Rust API guidelines for string parameters.
   - The trait must be object-safe so it can be used as `&dyn ToolExists` in the `validate` function signature.

3. **Stub implementation:** A public struct named `AllToolsExist` with a blanket `ToolExists` implementation that always returns `true`:
   ```rust
   pub struct AllToolsExist;

   impl ToolExists for AllToolsExist {
       fn tool_exists(&self, _name: &str) -> bool {
           true
       }
   }
   ```
   - `AllToolsExist` is a unit struct (no fields).
   - Derive `Debug`, `Clone`, `Copy` on `AllToolsExist` for ergonomic use in tests.
   - This struct exists specifically for tests that validate non-tool-related rules (e.g., confidence threshold, max turns, output format) and need a `ToolExists` implementor that does not interfere with their assertions.

4. **No external dependencies.** The file uses only standard Rust -- no crate imports. This means it compiles independently of any sibling dependency-adding tasks.

5. **Module registration:** Modify `crates/skill-loader/src/lib.rs` to add:
   ```rust
   pub mod validation;
   pub use validation::{ToolExists, AllToolsExist};
   ```
   Remove the placeholder `add()` function and its test, as they will be replaced by real content. (If other sibling tasks also need to modify `lib.rs`, coordinate via the module declaration convention: each task adds its own `mod` + `pub use` lines.)

6. **Unit tests:** Include a `#[cfg(test)] mod tests` block inside `validation.rs` with the following tests:
   - `all_tools_exist_returns_true`: Verify `AllToolsExist.tool_exists("any_name")` returns `true` for an arbitrary tool name.
   - `all_tools_exist_returns_true_for_empty_string`: Verify `AllToolsExist.tool_exists("")` returns `true` (edge case -- the stub does not validate input).
   - `trait_is_object_safe`: Construct a `&dyn ToolExists` from `&AllToolsExist` and call `tool_exists` through the trait object to confirm object safety at compile time.

## Implementation Details

### File: `crates/skill-loader/src/validation.rs` (new)

- **Imports:** None required. The file uses only primitive types (`bool`, `&str`) and standard trait mechanics.
- **`ToolExists` trait:** One method, `fn tool_exists(&self, name: &str) -> bool`. No associated types, no generics, no supertraits. This ensures object safety.
- **`AllToolsExist` struct:** Unit struct with `#[derive(Debug, Clone, Copy)]`. The `ToolExists` impl is a one-liner returning `true`.
- **Test module:** Three small tests as described in Requirements item 6.

### File: `crates/skill-loader/src/lib.rs` (modified)

- Replace the entire placeholder content with module declarations and re-exports.
- Add `pub mod validation;` and `pub use validation::{ToolExists, AllToolsExist};`.
- If the sibling task "Define SkillError enum" (issue #5) has already added `mod error; pub use error::SkillError;`, preserve those lines. If not, this task only adds the `validation` module lines.

### Integration points

- **Downstream `validate` function (issue #6, Group 3):** Will accept `tool_checker: &dyn ToolExists` as a parameter. For each tool name in `SkillManifest.tools`, it calls `tool_checker.tool_exists(name)` and collects failures.
- **Downstream `SkillLoader` struct (issue #5 / issue #6, Group 4):** Will store a `Box<dyn ToolExists>` (or `Arc<dyn ToolExists>`) to pass to `validate` during `load()`.
- **Future `tool-registry` crate (issue #8):** When the real `ToolRegistry` is implemented, it will `impl ToolExists for ToolRegistry`, and the skill-loader will accept it without any API changes.
- **Test code (issue #6, Group 5):** Validation unit tests will use `AllToolsExist` for tests that are not exercising tool-name validation. Tests that specifically exercise tool-name validation will define a local struct (e.g., `struct RejectTools(Vec<String>)`) that returns `false` for specific names.

## Dependencies

- **Blocked by:** Nothing. This task uses only standard Rust types and compiles independently.
- **Parallel with:** "Change `escalate_to` from `String` to `Option<String>` in `Constraints`" (Group 1), "Define allowed output format constants" (Group 1).
- **Blocking:** "Implement `validate` function" (Group 3) -- the `validate` function signature depends on the `ToolExists` trait.

## Risks & Edge Cases

1. **Object safety:** The trait must remain object-safe for use as `&dyn ToolExists`. The current design (single method, `&self` receiver, no generics, no associated types, no `Self`-returning methods) guarantees this. Any future extension must preserve object safety or introduce a separate trait.

2. **Thread safety:** `AllToolsExist` is a unit struct -- trivially `Send + Sync`. Any future `ToolExists` implementor that wraps shared state (e.g., a `HashMap` of registered tools) must ensure `Send + Sync` compatibility if used across async task boundaries. The trait itself does not require `Send + Sync` bounds; the `SkillLoader` task will add those bounds on the stored `Box<dyn ToolExists + Send + Sync>` if needed.

3. **Naming collision with `lib.rs` placeholder:** The placeholder `add()` function and test in `lib.rs` must be removed (or already removed by a sibling task). If two tasks modify `lib.rs` concurrently, the merge may require resolving a trivial conflict. This is low risk because the placeholder content is clearly disposable.

4. **`AllToolsExist` misuse in production:** The stub is intended for testing only. There is no compile-time enforcement preventing its use in production code. A `#[cfg(test)]` gate was considered but rejected because integration tests in separate `tests/` directories cannot see `#[cfg(test)]` items from the library. If stronger enforcement is desired later, a `testing` feature flag could gate the export.

5. **Empty tool name edge case:** `AllToolsExist` returns `true` even for `""`. This is correct behavior for a stub -- it deliberately does not validate. The `validate` function may choose to treat empty tool names as invalid independently of the `ToolExists` check.

## Verification

1. Run `cargo check -p skill-loader` to confirm the new module compiles without errors.
2. Run `cargo clippy -p skill-loader` to confirm no lint warnings.
3. Run `cargo test -p skill-loader` to confirm all three unit tests pass.
4. Verify that `validation.rs` contains no external crate imports -- only standard Rust.
5. Verify that `ToolExists` is object-safe by confirming the `trait_is_object_safe` test compiles and passes (constructing `&dyn ToolExists` is a compile-time check).
6. Verify that `AllToolsExist` derives `Debug`, `Clone`, `Copy` and that its `tool_exists` method unconditionally returns `true`.
7. Verify that `lib.rs` re-exports both `ToolExists` and `AllToolsExist` at the crate root.
