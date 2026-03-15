# Spec: Write serialization tests for envelope types

> From: .claude/tasks/issue-3.md

## Objective

Create integration tests that validate JSON round-trip serialization for the four envelope types (`AgentRequest`, `AgentResponse`, `ToolCallRecord`, `AgentError`), verify `AgentError`'s `Display` trait implementation produces correct human-readable messages, and confirm that the `AgentRequest::new()` constructor properly generates a UUID and sets defaults. These tests ensure the types survive serialization boundaries (critical for agent-to-orchestrator communication) and that error messages are useful for debugging.

## Current State

- `crates/agent-sdk/src/lib.rs` currently re-exports only the issue #2 types: `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`.
- The envelope types (`AgentRequest`, `AgentResponse`, `ToolCallRecord`, `AgentError`) do not exist yet. They are defined in Group 2 of the issue-3 task breakdown and must be created before this test file.
- `crates/agent-sdk/Cargo.toml` has `serde` and `schemars` in `[dependencies]` and `serde_yaml` in `[dev-dependencies]`. The Group 1 tasks will add `uuid`, `serde_json`, `async-trait`, and `tokio`.
- The existing test file `crates/agent-sdk/tests/skill_manifest_test.rs` demonstrates the project's round-trip testing pattern: construct a value, serialize it, deserialize it, assert equality with `assert_eq!`.
- The envelope types use `serde_json::Value` (not YAML), so these tests will serialize/deserialize via `serde_json` rather than `serde_yaml`.

## Requirements

### 1. Ensure `serde_json` is available for tests

`serde_json` is being added as a direct `[dependencies]` entry (per the "Add uuid and serde_json dependencies" task), so it is automatically available in integration tests. No additional dev-dependency is needed.

### 2. Test: `ToolCallRecord` JSON round-trip

Create a test named `tool_call_record_round_trip` that:
- Constructs a `ToolCallRecord` with fields: `tool_name: "web_search"`, `input: json!({"query": "rust async traits"})`, `output: json!({"results": ["link1", "link2"]})`.
- Serializes to a JSON string via `serde_json::to_string(&record)`.
- Deserializes the JSON string back via `serde_json::from_str::<ToolCallRecord>(&json_str)`.
- Asserts `original == deserialized` using `assert_eq!` (requires `PartialEq` on `ToolCallRecord`).

### 3. Test: `AgentRequest` JSON round-trip

Create a test named `agent_request_round_trip` that:
- Constructs an `AgentRequest` with all fields explicitly set: `id` as a known UUID (e.g., `Uuid::nil()` or `Uuid::parse_str(...)`), `input: "What are the COGS for Q3?"`, `context: Some(json!({"session_id": "abc123"}))`, `caller: Some("finance-router")`.
- Serializes to JSON, deserializes back, and asserts equality.

### 4. Test: `AgentRequest` with `None` optional fields

Create a test named `agent_request_none_fields_round_trip` that:
- Constructs an `AgentRequest` with `context: None` and `caller: None`.
- Serializes to JSON, deserializes back, and asserts equality.
- This verifies that `Option` fields serialize as `null` (or are omitted, depending on serde configuration) and deserialize correctly.

### 5. Test: `AgentRequest::new()` constructor

Create a test named `agent_request_new_constructor` that:
- Calls `AgentRequest::new("Hello agent".to_string())`.
- Asserts `request.input == "Hello agent"`.
- Asserts `request.id != Uuid::nil()` (i.e., a non-nil UUID was generated).
- Asserts `request.context.is_none()`.
- Asserts `request.caller.is_none()`.

### 6. Test: `AgentResponse` JSON round-trip

Create a test named `agent_response_round_trip` that:
- Constructs an `AgentResponse` with: `id` as a known UUID, `output: json!({"summary": "Q3 COGS were $1.2M", "confidence": 0.92})`, `confidence: 0.75_f32`, `escalated: false`, `tool_calls` containing one `ToolCallRecord`.
- Serializes to JSON, deserializes back, and asserts equality.
- Note: uses `0.75_f32` for the confidence field because `0.75` is exactly representable in f32 (it is `3/4`), avoiding floating-point precision issues during JSON round-tripping.

### 7. Test: `AgentResponse` with empty `tool_calls`

Create a test named `agent_response_empty_tool_calls_round_trip` that:
- Constructs an `AgentResponse` with `tool_calls: vec![]` and `escalated: true`.
- Serializes to JSON, deserializes back, and asserts equality.
- Verifies that empty `Vec` round-trips correctly and that the `escalated` flag is preserved.

### 8. Test: `AgentError` `Display` for `ToolCallFailed`

Create a test named `agent_error_display_tool_call_failed` that:
- Constructs `AgentError::ToolCallFailed { tool: "execute_sql".to_string(), reason: "connection timeout".to_string() }`.
- Calls `.to_string()` (which invokes `Display`).
- Asserts the output string contains both `"execute_sql"` and `"connection timeout"`.

### 9. Test: `AgentError` `Display` for `ConfidenceTooLow`

Create a test named `agent_error_display_confidence_too_low` that:
- Constructs `AgentError::ConfidenceTooLow { confidence: 0.45, threshold: 0.75 }`.
- Calls `.to_string()`.
- Asserts the output string contains both `"0.45"` and `"0.75"`.

### 10. Test: `AgentError` `Display` for `MaxTurnsExceeded`

Create a test named `agent_error_display_max_turns_exceeded` that:
- Constructs `AgentError::MaxTurnsExceeded { turns: 10 }`.
- Calls `.to_string()`.
- Asserts the output string contains `"10"`.

### 11. Test: `AgentError` `Display` for `Internal`

Create a test named `agent_error_display_internal` that:
- Constructs `AgentError::Internal("something went wrong".to_string())`.
- Calls `.to_string()`.
- Asserts the output string contains `"something went wrong"`.

### 12. Test: `AgentError` `PartialEq`

Create a test named `agent_error_equality` that:
- Asserts two identically-constructed `AgentError::MaxTurnsExceeded { turns: 5 }` values are equal.
- Asserts `AgentError::Internal("a".to_string()) != AgentError::Internal("b".to_string())`.
- Asserts cross-variant inequality, e.g., `AgentError::Internal("x".to_string()) != AgentError::MaxTurnsExceeded { turns: 1 }`.

## Implementation Details

### File to create

```
crates/agent-sdk/tests/envelope_types_test.rs
```

This is a Rust integration test file (lives in `tests/`, not `src/`). It imports from the crate's public API.

### Imports

```rust
use agent_sdk::{AgentError, AgentRequest, AgentResponse, ToolCallRecord};
use serde_json::json;
use uuid::Uuid;
```

The `serde_json` and `uuid` crates are direct dependencies of `agent-sdk`, so they are available in integration tests without additional dev-dependency declarations.

### Type definitions assumed (from issue-3 task breakdown)

- **`ToolCallRecord`**: `tool_name: String`, `input: serde_json::Value`, `output: serde_json::Value`. Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`. Must also derive `PartialEq` for `assert_eq!` in round-trip tests.

- **`AgentRequest`**: `id: uuid::Uuid`, `input: String`, `context: Option<serde_json::Value>`, `caller: Option<String>`. Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`. Must also derive `PartialEq`. Has `new(input: String)` constructor that generates `Uuid::new_v4()` and sets `context: None`, `caller: None`.

- **`AgentResponse`**: `id: uuid::Uuid`, `output: serde_json::Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. Derives: `Debug`, `Clone`, `Serialize`, `Deserialize`. Must also derive `PartialEq`.

- **`AgentError`**: Enum with variants `ToolCallFailed { tool: String, reason: String }`, `ConfidenceTooLow { confidence: f32, threshold: f32 }`, `MaxTurnsExceeded { turns: u32 }`, `Internal(String)`. Derives: `Debug`, `Clone`, `PartialEq`. Implements `Display` and `std::error::Error`. Does NOT derive `Serialize`/`Deserialize` (per issue-3 task breakdown), so no JSON round-trip test for `AgentError`.

### Floating-point considerations

`AgentResponse.confidence` is `f32`. The `PartialEq` derive on `f32` works for non-NaN values. For round-trip tests, use values exactly representable in f32 such as `0.5`, `0.25`, `0.75`, or `0.875` (powers of 2 or sums thereof). Avoid values like `0.1` or `0.92` which have infinite binary representations in IEEE 754 and may not survive f32-to-JSON-to-f32 round-tripping. For `AgentError::ConfidenceTooLow` Display tests, the values `0.45` and `0.75` are acceptable because they are only checked as substrings in the Display output, not round-tripped through serialization.

### Test naming convention

Follow the pattern established in `skill_manifest_test.rs`: use snake_case descriptive names that explain what is being verified (e.g., `tool_call_record_round_trip`, `agent_error_display_tool_call_failed`).

### `PartialEq` requirement

The task breakdown for `ToolCallRecord`, `AgentRequest`, and `AgentResponse` lists derives of `Debug, Clone, Serialize, Deserialize` but does not explicitly list `PartialEq`. The implementer of those types MUST add `PartialEq` to the derive list (as was done for the issue-2 types) to support `assert_eq!` in these tests. If `PartialEq` cannot be derived on `AgentResponse` due to `f32` limitations, the round-trip tests should compare fields individually instead of using struct-level equality.

## Dependencies

- **Blocked by**:
  - "Add `uuid` and `serde_json` dependencies" (Group 1) -- these crates must be in `[dependencies]` for the test imports to resolve.
  - "Define `ToolCallRecord` struct" (Group 2) -- the type must exist.
  - "Define `AgentRequest` struct" (Group 2) -- the type and `new()` constructor must exist.
  - "Define `AgentResponse` struct" (Group 2) -- the type must exist.
  - "Define `AgentError` enum" (Group 2) -- the type and `Display` impl must exist.
  - "Update `lib.rs` with new module declarations and re-exports" (Group 3) -- `use agent_sdk::AgentRequest` etc. must resolve from integration tests.

- **Blocking**:
  - "Run verification suite" (Group 4 final step) -- all tests must pass before the issue can be marked complete.

## Risks & Edge Cases

1. **`PartialEq` not derived on envelope types**: The issue-3 task breakdown does not explicitly mention `PartialEq` for `ToolCallRecord`, `AgentRequest`, or `AgentResponse`. If those types are implemented without `PartialEq`, the round-trip tests will fail to compile. Mitigation: the spec for those types should be updated to include `PartialEq`, or this test file should compare fields individually.

2. **f32 JSON round-trip precision**: `serde_json` serializes `f32` values and then parses them back. For most "clean" decimal values this works, but `f32` has limited precision (about 7 decimal digits). If `0.92_f32` serializes to `0.9200000166893005` (its exact f64 representation), the deserialized f32 may differ from the original. Mitigation: choose test values that are exactly representable in f32 (e.g., `0.5`, `0.25`, `0.75`), or use `0.875` which is `7/8` and exactly representable. The spec uses `0.75` for round-trip tests to avoid this issue.

3. **`AgentError` is not `Serialize`/`Deserialize`**: Per the task breakdown, `AgentError` derives `Debug, Clone, PartialEq` and implements `Display` and `Error`, but does NOT derive `Serialize`/`Deserialize`. Therefore, no JSON round-trip test is written for `AgentError`. If the implementer adds serde derives, a round-trip test could be added as a bonus, but it is not required.

4. **`Option` field serialization strategy**: If `AgentRequest` uses `#[serde(skip_serializing_if = "Option::is_none")]` on `context` and `caller`, those fields will be absent from JSON when `None`. The `Deserialize` impl will still handle the missing fields correctly (defaulting to `None`). The round-trip test will pass either way, but the JSON structure will differ. This is not a problem for the test, just worth noting.

5. **`serde_json::Value` equality**: `serde_json::Value` implements `PartialEq`, so nested JSON values in `ToolCallRecord.input`, `ToolCallRecord.output`, `AgentRequest.context`, and `AgentResponse.output` will compare correctly in `assert_eq!`.

6. **UUID crate availability in tests**: `uuid` is a direct dependency of `agent-sdk`, so it is available in integration tests under `use uuid::Uuid`. If for some reason the re-export strategy changes, the test may need `use agent_sdk::uuid::Uuid` or the crate may need to re-export `Uuid`. The current plan (direct dependency) should work.

## Verification

After implementation, run the following commands (per CLAUDE.md):

```bash
cargo check -p agent-sdk    # Ensure all types compile
cargo clippy -p agent-sdk   # No warnings
cargo test -p agent-sdk     # All tests pass
```

Specifically, confirm these tests exist and pass:

- `envelope_types_test::tool_call_record_round_trip`
- `envelope_types_test::agent_request_round_trip`
- `envelope_types_test::agent_request_none_fields_round_trip`
- `envelope_types_test::agent_request_new_constructor`
- `envelope_types_test::agent_response_round_trip`
- `envelope_types_test::agent_response_empty_tool_calls_round_trip`
- `envelope_types_test::agent_error_display_tool_call_failed`
- `envelope_types_test::agent_error_display_confidence_too_low`
- `envelope_types_test::agent_error_display_max_turns_exceeded`
- `envelope_types_test::agent_error_display_internal`
- `envelope_types_test::agent_error_equality`
