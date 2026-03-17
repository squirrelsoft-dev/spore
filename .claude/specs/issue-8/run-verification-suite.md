# Spec: Run verification suite

> From: .claude/tasks/issue-8.md

## Objective
Run the full workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that all `tool-registry` implementation and integration code compiles cleanly, produces no warnings, and all tests pass. This is the final gate task for issue-8 -- it validates that every preceding task (dependency setup, `ToolEntry`, `RegistryError`, `ToolRegistry` struct, `ToolExists` trait migration, downstream `skill-loader` updates, unit tests, and integration tests) integrates correctly across the entire workspace.

## Current State
The workspace contains five crates defined in the root `Cargo.toml`:
- `agent-sdk` -- core types (`SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`, `MicroAgent` trait, etc.)
- `skill-loader` -- markdown frontmatter parsing, validation, `SkillLoader` struct; depends on `tool-registry`
- `tool-registry` -- currently a placeholder (`pub struct ToolRegistry;`); by the time this task runs, preceding tasks will have populated it with `ToolEntry`, `RegistryError`, `ToolRegistry` (backed by `Arc<RwLock<HashMap<String, ToolEntry>>>`), and the `ToolExists` trait implementation
- `agent-runtime` -- empty placeholder
- `orchestrator` -- empty placeholder

By the time this task runs, the preceding tasks will have:
1. Added dependencies (`serde`, `schemars`, `serde_json`, `agent-sdk`) to `crates/tool-registry/Cargo.toml`
2. Created `crates/tool-registry/src/tool_entry.rs` with the `ToolEntry` struct
3. Created `crates/tool-registry/src/registry_error.rs` with the `RegistryError` enum
4. Created `crates/tool-registry/src/tool_registry.rs` with the `ToolRegistry` struct and methods (`new`, `register`, `assert_exists`, `resolve_for_skill`, `connect` stub, `get`)
5. Moved the `ToolExists` trait from `skill-loader/src/validation.rs` to `tool-registry` and re-exported it from `skill-loader` for backward compatibility
6. Replaced `crates/tool-registry/src/lib.rs` placeholder with module declarations and re-exports
7. Updated `skill-loader` test files to use `ToolRegistry::new()` instead of the unit struct `ToolRegistry`
8. Created unit tests in `crates/tool-registry/tests/tool_entry_test.rs` (serialization round-trip, equality, error Display)
9. Created integration tests in `crates/tool-registry/tests/tool_registry_test.rs` (register, get, assert_exists, duplicate detection, resolve_for_skill, ToolExists trait impl)

## Requirements
- `cargo check` succeeds across the entire workspace with zero errors
- `cargo clippy` succeeds across the entire workspace with zero warnings (no `#[allow(...)]` suppressions added solely to silence legitimate warnings)
- `cargo test` succeeds across the entire workspace with all tests passing, including:
  - All existing `agent-sdk` tests (serialization, construction, object safety)
  - All existing `skill-loader` unit tests (`frontmatter.rs`, `validation.rs`)
  - All existing `skill-loader` integration tests (`skill_loader_test.rs`, `validation_integration_test.rs`, `validation_test.rs`) -- these depend on `ToolRegistry` and must work with the updated constructor `ToolRegistry::new()`
  - New `tool-registry` unit tests (`tool_entry_test.rs` -- serialization round-trip, equality, RegistryError Display)
  - New `tool-registry` integration tests (`tool_registry_test.rs` -- register_and_get, assert_exists_ok, assert_exists_error, duplicate_error, resolve_for_skill, resolve_for_skill_fails, get_none, tool_exists_trait_impl)
  - Any pre-existing tests in `agent-runtime` and `orchestrator` (currently none expected)
- No commented-out code or debug statements remain in `tool-registry` or modified `skill-loader` source files
- No unused imports, dead code, or other Clippy lint violations in the `tool-registry` crate or the modified portions of `skill-loader`

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo check`** from the workspace root (`/workspaces/spore`). This performs type-checking across all workspace members. If it fails, diagnose the root cause -- likely candidates include:
   - Type mismatch from the `ToolRegistry` constructor change (unit struct vs. `::new()`)
   - Missing or incorrect imports after `ToolExists` trait migration
   - Dependency version mismatch in `tool-registry/Cargo.toml`
   - Missing `pub` visibility on newly introduced types or methods

2. **Run `cargo clippy`** from the workspace root. This applies Rust's standard lints plus Clippy's extended checks. Pay attention to:
   - Unused imports or variables in `tool-registry` modules
   - Redundant clones in `ToolRegistry` methods (especially around `RwLock` read/write guards)
   - Missing `pub` visibility issues on re-exported types
   - Warnings in test modules (both `tool-registry` and updated `skill-loader` tests)
   - Dead code from the `connect()` stub -- should have appropriate `#[allow]` or `// TODO` annotation

3. **Run `cargo test`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions. Verify:
   - All new `tool-registry` unit tests pass (ToolEntry serialization round-trip, ToolEntry equality, RegistryError Display output)
   - All new `tool-registry` integration tests pass (register_and_get, assert_exists variants, duplicate detection, resolve_for_skill variants, get_returns_none, tool_exists_trait_impl)
   - All existing `skill-loader` tests still pass -- critically, the `make_loader()` helper in both `skill_loader_test.rs` and `validation_integration_test.rs` must work with `Arc::new(ToolRegistry::new())` instead of `Arc::new(ToolRegistry)`
   - All `skill-loader` validation unit tests still pass (AllToolsExist, trait object safety)
   - All pre-existing `agent-sdk` tests still pass (no regressions)

4. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `tool-registry` and `skill-loader` crates unless a workspace-level issue is discovered.

### Files potentially touched (fixes only, if needed)
- `crates/tool-registry/Cargo.toml` -- dependency version or feature adjustments
- `crates/tool-registry/src/lib.rs` -- re-export or visibility fixes
- `crates/tool-registry/src/tool_entry.rs` -- derive or field corrections
- `crates/tool-registry/src/registry_error.rs` -- Display/Error impl corrections
- `crates/tool-registry/src/tool_registry.rs` -- method signature or import fixes
- `crates/tool-registry/tests/tool_entry_test.rs` -- test fixture corrections
- `crates/tool-registry/tests/tool_registry_test.rs` -- test fixture or assertion corrections
- `crates/skill-loader/src/validation.rs` -- import path fixes after ToolExists migration
- `crates/skill-loader/src/lib.rs` -- re-export fixes for ToolExists backward compatibility
- `crates/skill-loader/tests/skill_loader_test.rs` -- ToolRegistry constructor fix
- `crates/skill-loader/tests/validation_integration_test.rs` -- ToolRegistry constructor fix

## Dependencies
- Blocked by: All previous tasks (Groups 1-5: dependency setup, type definitions, core implementation, trait implementation, downstream updates, unit tests, integration tests)
- Blocking: None (this is the final task for issue-8)

## Risks & Edge Cases
- **Cross-crate breakage from ToolExists migration**: Moving the `ToolExists` trait from `skill-loader` to `tool-registry` changes the canonical source of the trait. If the re-export from `skill-loader` is not set up correctly, downstream code importing `skill_loader::ToolExists` will fail to compile. Mitigation: verify that `crates/skill-loader/src/lib.rs` contains `pub use tool_registry::ToolExists;` (or equivalent re-export through the validation module).
- **ToolRegistry constructor change breaking skill-loader tests**: Two test files (`skill_loader_test.rs`, `validation_integration_test.rs`) construct `Arc::new(ToolRegistry)` using the unit struct. If the Group 4 task did not update both files to `Arc::new(ToolRegistry::new())`, these tests will fail to compile. Mitigation: `cargo check` will catch this before `cargo test`.
- **Clippy warnings from `connect()` stub**: The no-op `connect()` method may trigger Clippy's `unused_variables` or similar warnings for the `_url` parameter. Mitigation: the underscore prefix on the parameter name should suppress this; verify it does.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions (e.g., stricter `unsafe` rules, let-else patterns). Mitigation: address each lint individually rather than blanket-suppressing with `#[allow]`.
- **std::sync::RwLock poisoning**: If a test panics while holding a write lock, subsequent tests in the same process may encounter a poisoned lock. This would not indicate a real bug but would cause cascading test failures. Mitigation: each integration test should create its own `ToolRegistry::new()` instance.
- **Regressions in other crates**: The verification runs workspace-wide, so a failing test in `agent-sdk` or another crate would block this task even though it is unrelated. Mitigation: if a pre-existing test fails, confirm it also fails on main before attributing it to tool-registry changes.

## Verification
- `cargo check` exits with code 0 and produces no error output
- `cargo clippy` exits with code 0 and produces no warning output
- `cargo test` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- The above three commands are run from the workspace root `/workspaces/spore` without any `--package` filter, confirming workspace-wide health
