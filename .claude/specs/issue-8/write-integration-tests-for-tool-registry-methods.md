# Spec: Write integration tests for `ToolRegistry` methods

> From: .claude/tasks/issue-8.md

## Objective

Create a comprehensive integration test suite for the `ToolRegistry` struct that exercises every public method (`new`, `register`, `get`, `assert_exists`, `resolve_for_skill`) and the `ToolExists` trait implementation. These tests validate that the registry correctly stores, retrieves, and resolves tool entries, rejects duplicates, and returns the appropriate errors for missing tools. They serve as the primary regression safety net before the crate is consumed by downstream code (e.g., `skill-loader`).

## Current State

- The `tool-registry` crate currently contains only a placeholder unit struct `pub struct ToolRegistry;` in `crates/tool-registry/src/lib.rs`.
- Prior tasks in this issue (Groups 1-4) will have created:
  - `crates/tool-registry/src/tool_entry.rs` -- `ToolEntry { name, version, endpoint }` with `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`.
  - `crates/tool-registry/src/registry_error.rs` -- `RegistryError` enum with `ToolNotFound`, `DuplicateEntry`, `ConnectionFailed` variants.
  - `crates/tool-registry/src/tool_registry.rs` -- `ToolRegistry` with `entries: Arc<RwLock<HashMap<String, ToolEntry>>>` and methods `new()`, `register()`, `get()`, `assert_exists()`, `resolve_for_skill()`, `connect()`.
  - `ToolExists` trait either defined in or re-exported from `tool-registry`, implemented for `ToolRegistry` by delegating to `assert_exists`.
  - `crates/tool-registry/src/lib.rs` wired up with `pub use` re-exports for `ToolEntry`, `RegistryError`, `ToolRegistry`, and `ToolExists`.
- No test files exist under `crates/tool-registry/tests/`.
- The `skill-loader` crate's `validation_test.rs` uses a `valid_manifest()` helper and custom `ToolExists` stub (`RejectTools`) as a testing pattern. The integration test file `validation_integration_test.rs` demonstrates external test file conventions (importing from crate public API, no `#[cfg(test)]` wrapper needed).
- `SkillManifest` is constructed with fields: `name`, `version`, `description`, `model: ModelConfig`, `preamble`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`. All are imported from `agent_sdk`.

## Requirements

- Create `crates/tool-registry/tests/tool_registry_test.rs` with exactly eight test functions.
- Each test must use only the public API of `tool-registry` and `agent-sdk` (no `pub(crate)` or internal access).
- Tests must not depend on `tokio` or any async runtime -- all `ToolRegistry` methods use `std::sync::RwLock`, not async locks.
- Each test function must be self-contained: create its own `ToolRegistry::new()` instance, register its own entries, and assert independently.
- Provide a `make_entry(name: &str) -> ToolEntry` helper to reduce boilerplate, using fixed values for `version` and `endpoint`.
- Provide a `make_manifest(tools: Vec<&str>) -> SkillManifest` helper to construct valid `SkillManifest` instances with only the `tools` field varying. Use valid placeholder values for all other fields (matching the pattern in `validation_test.rs`).
- All assertions must be deterministic (no ordering assumptions on HashMap iteration, etc.).

### Test 1: `register_and_get`
- Create a registry, register an entry with name `"web_search"`.
- Call `get("web_search")`.
- Assert `Some(entry)` is returned and the entry's fields match what was registered.

### Test 2: `assert_exists_returns_ok_for_registered_tool`
- Create a registry, register an entry with name `"file_read"`.
- Call `assert_exists("file_read")`.
- Assert the result is `Ok(())`.

### Test 3: `assert_exists_returns_error_for_missing_tool`
- Create an empty registry (no registrations).
- Call `assert_exists("nonexistent")`.
- Assert the result is `Err(RegistryError::ToolNotFound { name })` where `name == "nonexistent"`.
- Use direct `PartialEq` comparison: `assert_eq!(err, RegistryError::ToolNotFound { name: "nonexistent".into() })`.

### Test 4: `register_duplicate_returns_error`
- Create a registry, register an entry with name `"web_search"`.
- Register another entry with the same name `"web_search"` (different version/endpoint is fine).
- Assert the second registration returns `Err(RegistryError::DuplicateEntry { name })` where `name == "web_search"`.

### Test 5: `resolve_for_skill_returns_matching_entries`
- Create a registry, register three entries: `"web_search"`, `"file_read"`, `"shell_exec"`.
- Create a `SkillManifest` whose `tools` field contains `["web_search", "file_read"]` (a subset).
- Call `resolve_for_skill(&manifest)`.
- Assert the result is `Ok(entries)` where `entries` has length 2.
- Assert the returned entries contain exactly the `"web_search"` and `"file_read"` entries (compare by sorting the result by name to avoid order dependence).

### Test 6: `resolve_for_skill_fails_on_missing_tool`
- Create a registry, register only `"web_search"`.
- Create a `SkillManifest` whose `tools` field contains `["web_search", "missing_tool"]`.
- Call `resolve_for_skill(&manifest)`.
- Assert the result is `Err(RegistryError::ToolNotFound { name })` where `name == "missing_tool"`.

### Test 7: `get_returns_none_for_missing_tool`
- Create an empty registry.
- Call `get("nonexistent")`.
- Assert the result is `None`.

### Test 8: `tool_exists_trait_impl`
- Create a registry, register an entry with name `"web_search"`.
- Bind the registry to a `&dyn ToolExists` reference to prove object safety.
- Call `tool_exists("web_search")` via the trait object; assert it returns `true`.
- Call `tool_exists("nonexistent")` via the trait object; assert it returns `false`.

## Implementation Details

### File to create

**`crates/tool-registry/tests/tool_registry_test.rs`**

Imports needed:
```rust
use std::collections::HashMap;
use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};
use tool_registry::{RegistryError, ToolEntry, ToolRegistry};
```

For the `ToolExists` trait import:
- If `ToolExists` is re-exported from `tool_registry`: `use tool_registry::ToolExists;`
- If `ToolExists` remains in `skill_loader` and is only implemented on `ToolRegistry` there: `use skill_loader::ToolExists;` (would require `skill-loader` as a dev-dependency of `tool-registry`, which is unlikely). The task description says the trait is implemented for `ToolRegistry`, so it will almost certainly be accessible via `tool_registry::ToolExists`.

### Helper functions

```rust
fn make_entry(name: &str) -> ToolEntry {
    ToolEntry {
        name: name.to_string(),
        version: "1.0".to_string(),
        endpoint: format!("mcp://localhost:7001/{name}"),
    }
}
```

```rust
fn make_manifest(tools: Vec<&str>) -> SkillManifest {
    SkillManifest {
        name: "test-skill".to_string(),
        version: "1.0".to_string(),
        description: "A test skill".to_string(),
        model: ModelConfig {
            provider: "anthropic".to_string(),
            name: "claude-3-haiku".to_string(),
            temperature: 0.5,
        },
        preamble: "You are a test assistant.".to_string(),
        tools: tools.into_iter().map(String::from).collect(),
        constraints: Constraints {
            max_turns: 5,
            confidence_threshold: 0.8,
            escalate_to: None,
            allowed_actions: vec!["search".to_string()],
        },
        output: OutputSchema {
            format: "json".to_string(),
            schema: HashMap::from([("result".to_string(), "string".to_string())]),
        },
    }
}
```

These helpers match the pattern from `crates/skill-loader/tests/validation_test.rs` (`valid_manifest()` and `RejectTools`), adapted for `ToolRegistry` needs.

### Key patterns to follow
- Each test is a standalone `#[test]` function (not `#[tokio::test]` -- no async needed).
- Error assertions use `PartialEq`-based `assert_eq!` where the error variant can be constructed directly, following the `RegistryError` design which derives `PartialEq`.
- `Option` assertions use `assert_eq!(result, Some(expected))` or `assert!(result.is_none())`.
- For `resolve_for_skill` ordering: sort the returned `Vec<ToolEntry>` by `name` before comparing, since `HashMap` iteration order is non-deterministic and `resolve_for_skill` iterates `manifest.tools` in order (which is deterministic), but being explicit about ordering in tests is safer.

### Dev-dependencies

The `tool-registry` crate's `Cargo.toml` will need `agent-sdk` available for tests. Since `agent-sdk` is already a regular dependency (added in Group 1), its types are accessible in integration tests. No additional `[dev-dependencies]` are needed unless `ToolExists` remains in `skill-loader`, in which case:
```toml
[dev-dependencies]
skill-loader = { path = "../skill-loader" }
```
However, per the task plan (Group 3), `ToolExists` is being moved to `tool-registry`, so this should not be necessary.

## Dependencies

- Blocked by: "Update `skill-loader` to use real `ToolRegistry` methods" (Group 4) -- the `ToolRegistry` struct, `ToolEntry`, `RegistryError`, `ToolExists` trait, and `lib.rs` re-exports must all be in place before these tests can compile.
- Blocking: "Run verification suite" (Group 6) -- all tests must pass before the final `cargo test` gate.

## Risks & Edge Cases

- **`ToolExists` import path**: If the trait ends up being re-exported from a different location than expected, the import statement will need adjustment. The spec assumes `tool_registry::ToolExists` based on the Group 3 task description. Verify the actual re-export location before writing the test file.
- **`resolve_for_skill` ordering**: The method iterates `manifest.tools` and collects entries in that order, so the result order should match the manifest's `tools` order. Nevertheless, the spec recommends sorting before comparison to be resilient against future implementation changes.
- **Thread safety is not tested**: The task description does not require concurrency tests (spawning threads that register/read simultaneously). These tests are single-threaded. Thread safety is guaranteed by the `Arc<RwLock<...>>` design and can be tested separately if needed.
- **`connect()` stub**: The stub method is not tested here because it is a no-op. It will be tested when real MCP integration lands in issue #9.
- **No `[dev-dependencies]` for `tool_registry`**: If `agent-sdk` is only in `[dependencies]` (not `[dev-dependencies]`), its types are still accessible in integration tests under `tests/`. Confirm `agent-sdk` is listed in the crate's `[dependencies]` from the Group 1 task.

## Verification

- `cargo test -p tool-registry` passes with all eight tests green.
- `cargo test -p tool-registry -- --list` shows exactly eight test functions in `tool_registry_test`.
- `cargo clippy -p tool-registry` produces no warnings for the test file.
- Each test exercises exactly one behavior and has a descriptive name matching the task specification.
- The `tool_exists_trait_impl` test uses `&dyn ToolExists` to confirm object safety (the compiler will reject the test if the trait is not object-safe).
- No test depends on execution order or shared mutable state between tests.
