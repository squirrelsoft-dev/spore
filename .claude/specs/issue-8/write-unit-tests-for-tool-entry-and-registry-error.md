# Spec: Write unit tests for `ToolEntry` and `RegistryError`

> From: .claude/tasks/issue-8.md

## Objective

Add unit tests for the `ToolEntry` struct and `RegistryError` enum in the `tool-registry` crate. These tests verify serialization correctness, equality semantics, and Display output -- catching regressions before the types are consumed by higher-level components like `ToolRegistry` methods and `skill-loader` integration. The tests follow the established patterns from `crates/agent-sdk/tests/envelope_types_test.rs` (serialization round-trips, equality checks) and the `agent_error_display_contains_expected_substrings` test in that same file (Display assertion pattern).

## Current State

- `ToolEntry` will be defined in `crates/tool-registry/src/tool_entry.rs` (spec: `define-tool-entry-struct.md`) with three `String` fields (`name`, `version`, `endpoint`) and derives `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.
- `RegistryError` will be defined in `crates/tool-registry/src/registry_error.rs` (spec: `define-registry-error-enum.md`) with three variants (`ToolNotFound`, `DuplicateEntry`, `ConnectionFailed`), derives `Debug, Clone, PartialEq`, and has manual `Display` and `Error` implementations.
- Both types will be re-exported from `crates/tool-registry/src/lib.rs` as `pub use tool_entry::ToolEntry;` and `pub use registry_error::RegistryError;`.
- The `tool-registry` crate currently has no `[dev-dependencies]` section in its `Cargo.toml`. The `serde_json` crate is listed as a regular dependency (per the `add-dependencies-to-tool-registry-cargo-toml` spec), so it is available for tests without adding a dev-dependency.
- The `crates/tool-registry/tests/` directory does not yet exist.
- The codebase uses external integration test files (e.g., `crates/agent-sdk/tests/envelope_types_test.rs`) rather than inline `#[cfg(test)]` modules for this category of type-level unit tests. This spec follows that convention.

## Requirements

### `ToolEntry` tests

- **JSON serialization round-trip**: Construct a `ToolEntry` with representative values, serialize to JSON with `serde_json::to_string`, deserialize back with `serde_json::from_str`, and assert equality with the original. Follows the `tool_call_record_json_round_trip_with_nested_values` pattern from `envelope_types_test.rs`.
- **JSON round-trip with special characters in endpoint**: Construct a `ToolEntry` whose `endpoint` contains characters that must survive JSON encoding (e.g., a unix socket path like `"/tmp/tool.sock"` or a URL with port like `"mcp://localhost:7001"`). Serialize and deserialize, assert equality.
- **Equality with matching entries**: Construct two `ToolEntry` values with identical field values, assert `==` returns true.
- **Inequality with non-matching entries**: Construct two `ToolEntry` values that differ in one field (test each field independently: `name`, `version`, `endpoint`), assert `!=` returns true for each.
- **Clone produces equal value**: Clone a `ToolEntry` and assert equality with the original, confirming the `Clone` derive works correctly.

### `RegistryError` tests

- **Display output for `ToolNotFound`**: Format the variant with `format!("{}", err)` and assert the output contains both the substring `"tool not found"` and the tool name value. Use the `assert!(display.contains(...), "expected '...' in: {display}")` pattern from `agent_error_display_contains_expected_substrings`.
- **Display output for `DuplicateEntry`**: Format and assert the output contains `"duplicate tool entry"` and the tool name value.
- **Display output for `ConnectionFailed`**: Format and assert the output contains `"connection to"`, the endpoint value, `"failed"`, and the reason value.
- **Display exact format verification**: For each variant, assert the full `to_string()` output matches the exact expected string. This catches format drift (e.g., missing single quotes). Exact strings:
  - `ToolNotFound { name: "my-tool" }` -> `"tool not found: 'my-tool'"`
  - `DuplicateEntry { name: "my-tool" }` -> `"duplicate tool entry: 'my-tool'"`
  - `ConnectionFailed { endpoint: "mcp://localhost:7001", reason: "timeout" }` -> `"connection to 'mcp://localhost:7001' failed: timeout"`
- **Error trait implementation**: Verify `RegistryError` can be used as `&dyn std::error::Error` (construct a variant and take a reference as `&dyn std::error::Error`, call `.to_string()` on it to confirm it compiles and works).
- **Equality between matching variants**: Construct two identical `ToolNotFound` variants and assert equality.
- **Inequality between different variant types**: Assert `ToolNotFound` != `DuplicateEntry` even if they share the same `name` value.
- **Inequality between same variant with different data**: Assert `ToolNotFound { name: "a" }` != `ToolNotFound { name: "b" }`.

## Implementation Details

### File to create

**`crates/tool-registry/tests/tool_entry_test.rs`**

This single file contains all tests for both `ToolEntry` and `RegistryError`. The agent-sdk crate groups related type tests into one file (`envelope_types_test.rs` covers `AgentRequest`, `AgentResponse`, `ToolCallRecord`, `HealthStatus`, and `AgentError`), so combining `ToolEntry` and `RegistryError` tests in one file is consistent.

Structure:
```
use tool_registry::{ToolEntry, RegistryError};
use serde_json;

// --- ToolEntry tests ---

#[test]
fn tool_entry_json_round_trip() { ... }

#[test]
fn tool_entry_json_round_trip_with_unix_socket_endpoint() { ... }

#[test]
fn tool_entry_equality_with_matching_entries() { ... }

#[test]
fn tool_entry_inequality_differs_by_name() { ... }

#[test]
fn tool_entry_inequality_differs_by_version() { ... }

#[test]
fn tool_entry_inequality_differs_by_endpoint() { ... }

#[test]
fn tool_entry_clone_produces_equal_value() { ... }

// --- RegistryError tests ---

#[test]
fn registry_error_display_contains_expected_substrings() { ... }

#[test]
fn registry_error_display_exact_format() { ... }

#[test]
fn registry_error_implements_std_error_trait() { ... }

#[test]
fn registry_error_equality_matching_variants() { ... }

#[test]
fn registry_error_inequality_different_variant_types() { ... }

#[test]
fn registry_error_inequality_same_variant_different_data() { ... }
```

### Key patterns to follow

- **Imports**: `use tool_registry::{ToolEntry, RegistryError};` -- import from the crate's public API, not from internal modules.
- **Serialization**: Use `serde_json::to_string` and `serde_json::from_str`, assert `original == deserialized`. This is the exact pattern from `agent_request_json_round_trip` and `tool_call_record_json_round_trip_with_nested_values`.
- **Display assertions**: Use `format!("{}", err)` and `assert!(display.contains("substring"), "expected 'substring' in: {display}")`. This is the exact pattern from `agent_error_display_contains_expected_substrings`.
- **No async**: All tests are synchronous `#[test]` functions. No `tokio::test` needed since these are pure in-memory operations.
- **Test function naming**: Use snake_case descriptive names following the existing convention (e.g., `tool_entry_json_round_trip`, not `test_json_round_trip`).

### Test data values

Use realistic representative values for `ToolEntry`:
- Primary: `name: "web_search"`, `version: "1.0.0"`, `endpoint: "mcp://localhost:7001"`
- Unix socket variant: `name: "file_read"`, `version: "2.1.0"`, `endpoint: "/tmp/file-read.sock"`

Use realistic representative values for `RegistryError`:
- `ToolNotFound`: `name: "nonexistent_tool"`
- `DuplicateEntry`: `name: "web_search"`
- `ConnectionFailed`: `endpoint: "mcp://localhost:7001"`, `reason: "connection refused"`
- Exact format tests: use the values from the `define-registry-error-enum.md` verification section (`"my-tool"`, `"mcp://localhost:7001"`, `"timeout"`) to stay consistent.

### Cargo.toml changes

No changes needed. `serde_json` is already a regular dependency of `tool-registry` (added in the `add-dependencies-to-tool-registry-cargo-toml` task), so it is available in integration test files. No additional `[dev-dependencies]` are required.

### Directory creation

Create the `crates/tool-registry/tests/` directory if it does not already exist.

## Dependencies

- Blocked by: "Update `skill-loader` to use real `ToolRegistry` methods" -- all Group 1-4 tasks must be complete so that `ToolEntry` and `RegistryError` are defined, wired into `lib.rs`, and the crate compiles successfully.
- Blocking: "Run verification suite" (Group 6)

## Risks & Edge Cases

- **Display format drift**: If the `RegistryError::Display` implementation deviates from the exact format specified in `define-registry-error-enum.md`, the exact-format test will catch it immediately. The substring-based test provides a softer fallback that survives minor wording changes while still catching missing field values.
- **Serialization field rename**: If a `#[serde(rename = "...")]` attribute is added to `ToolEntry` fields in the future, the round-trip test will still pass (it tests serialize-then-deserialize identity, not specific JSON key names). If testing specific JSON structure is desired later, a separate test can inspect the intermediate JSON string.
- **No `Serialize`/`Deserialize` on `RegistryError`**: The spec for `RegistryError` explicitly omits serde derives. Tests must NOT attempt to serialize/deserialize `RegistryError` -- only `Display` and equality are tested.
- **Test file location**: Using `tests/tool_entry_test.rs` (external integration test) rather than inline `#[cfg(test)]` modules means the tests can only access the crate's public API. This is intentional and consistent with the codebase pattern, but means internal-only methods cannot be tested here.

## Verification

- `cargo test -p tool-registry` passes with all new tests green.
- `cargo test -p tool-registry -- --list` shows all expected test function names.
- `cargo clippy -p tool-registry -- -D warnings` produces no warnings in test code.
- Each `ToolEntry` round-trip test produces a valid JSON intermediate string (can manually inspect by adding a temporary `println!` if needed during implementation).
- Each `RegistryError` Display test confirms the exact format strings from the `define-registry-error-enum.md` verification section.
