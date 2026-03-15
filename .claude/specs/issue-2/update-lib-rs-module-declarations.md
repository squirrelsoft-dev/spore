# Spec: Update `lib.rs` module declarations and re-exports

> From: .claude/tasks/issue-2.md

## Objective

Replace the placeholder `add()` function and its test in `crates/agent-sdk/src/lib.rs` with proper module declarations and public re-exports. After this change, downstream crates can import types directly from the crate root (e.g., `use agent_sdk::SkillManifest;`) without needing to navigate internal module paths.

## Current State

`crates/agent-sdk/src/lib.rs` contains only a placeholder `add(left, right)` function and a single `#[test]` asserting `add(2, 2) == 4`. There are no module declarations and no re-exports. No other `.rs` files exist under `crates/agent-sdk/src/` yet (the module files will be created by the prerequisite tasks in Group 2 and Group 3).

## Requirements

1. **Remove all placeholder code.** Delete the `add()` function and the `#[cfg(test)] mod tests` block entirely. No commented-out code should remain.

2. **Declare four submodules.** Add `mod` declarations for each of the following, in this order:
   - `mod model_config;`
   - `mod constraints;`
   - `mod output_schema;`
   - `mod skill_manifest;`

   These correspond to the files `model_config.rs`, `constraints.rs`, `output_schema.rs`, and `skill_manifest.rs` that will already exist under `crates/agent-sdk/src/` by the time this task executes.

3. **Add public re-exports.** Add `pub use` statements that re-export each module's primary struct from the crate root:
   - `pub use model_config::ModelConfig;`
   - `pub use constraints::Constraints;`
   - `pub use output_schema::OutputSchema;`
   - `pub use skill_manifest::SkillManifest;`

   After this change, consumers can write `use agent_sdk::SkillManifest;` (and likewise for the other three types) without referencing the submodule path.

4. **No other public API surface.** The submodules themselves should remain private (`mod`, not `pub mod`). Only the four struct types are re-exported. This keeps the public API flat and intentional -- internal module structure is an implementation detail.

5. **No new dependencies.** This task modifies only `lib.rs` and adds no dependencies to `Cargo.toml`.

6. **No tests in `lib.rs`.** The old placeholder test must be removed. Integration tests live in `crates/agent-sdk/tests/` and are handled by the separate "Write deserialization tests" task.

## Implementation Details

The final `crates/agent-sdk/src/lib.rs` should contain exactly:

```rust
mod model_config;
mod constraints;
mod output_schema;
mod skill_manifest;

pub use model_config::ModelConfig;
pub use constraints::Constraints;
pub use output_schema::OutputSchema;
pub use skill_manifest::SkillManifest;
```

No additional code, doc comments, or feature flags are needed at this stage. The file should be minimal and focused solely on wiring the module tree and establishing the public API.

### Module visibility rationale

Using private `mod` with selective `pub use` re-exports (the "facade" pattern) means:
- The crate's public API is exactly four types: `ModelConfig`, `Constraints`, `OutputSchema`, `SkillManifest`.
- Internal module names can be refactored later without breaking downstream consumers.
- Consumers never need to write `use agent_sdk::skill_manifest::SkillManifest;` -- the shorter `use agent_sdk::SkillManifest;` is the canonical import path.

## Dependencies

- **Blocked by:** All four module files must exist before this task can compile:
  - `crates/agent-sdk/src/model_config.rs` (defines `pub struct ModelConfig`)
  - `crates/agent-sdk/src/constraints.rs` (defines `pub struct Constraints`)
  - `crates/agent-sdk/src/output_schema.rs` (defines `pub struct OutputSchema`)
  - `crates/agent-sdk/src/skill_manifest.rs` (defines `pub struct SkillManifest`)
- **Blocking:** "Write deserialization tests" -- tests import types through these re-exports.

## Risks & Edge Cases

1. **Struct visibility mismatch.** Each struct in its respective module file must be declared `pub` (not `pub(crate)` or private), otherwise the `pub use` re-exports will fail to compile. If a prerequisite task declares a struct as `pub(crate)`, this task will surface a compile error. The fix belongs in the module file, not in `lib.rs`.

2. **Field visibility.** For `serde::Deserialize` to work on types constructed outside the crate (e.g., in integration tests), all struct fields must also be `pub`. This is not enforced by `lib.rs` itself but is a contract the module files must uphold.

3. **Module ordering.** Rust does not require a specific declaration order for `mod` statements, but listing them in dependency order (leaf types before composite types) improves readability. `skill_manifest` depends on the other three, so it is listed last.

4. **Name collisions.** None of the four type names conflict with standard library types or common crate names. No risk of ambiguity.

5. **Compilation without all modules present.** If this task is attempted before all four `.rs` files exist, `rustc` will emit "file not found" errors for the missing modules. This is the expected guard rail -- the dependency chain in the task breakdown prevents this in practice.

## Verification

1. **`cargo check -p agent-sdk`** must succeed with no errors.
2. **`cargo clippy -p agent-sdk`** must produce no warnings.
3. **`cargo test -p agent-sdk`** must pass (there will be zero tests in `lib.rs` at this point; integration tests are added by the next task).
4. **Manual inspection:** confirm `lib.rs` contains no remnant of the `add()` function or the placeholder test block.
5. **Import check:** a downstream crate (or a doc-test / integration test added later) should be able to write `use agent_sdk::SkillManifest;` and have it resolve correctly.
