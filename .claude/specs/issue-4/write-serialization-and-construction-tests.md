# Spec: Write serialization and construction tests

> From: .claude/tasks/issue-4.md

## Objective
Create an integration test file that validates construction helpers and JSON round-trip serialization for the five new envelope types introduced in issue #4: `AgentRequest`, `AgentResponse`, `ToolCallRecord`, `AgentError`, and `HealthStatus`. This is the final functional task in the issue and serves as the acceptance gate — if these tests pass, the types are correctly defined, wired into `lib.rs`, and interoperable with `serde_json`.

## Current State
- `crates/agent-sdk/tests/skill_manifest_test.rs` exists and demonstrates the project's integration test patterns: YAML literals, field-by-field assertions, round-trip tests, and edge-case coverage.
- The crate currently exports `SkillManifest`, `ModelConfig`, `Constraints`, and `OutputSchema` from `lib.rs`.
- `Cargo.toml` has `serde_yaml = "0.9"` as the sole dev-dependency. `serde_json` will be a regular dependency (added by the "Add `uuid` and `serde_json` dependencies" task), so it is available for use in tests without needing a separate dev-dependency entry.
- The five new types (`AgentRequest`, `AgentResponse`, `ToolCallRecord`, `AgentError`, `HealthStatus`) do not exist yet. This task is blocked until Groups 1-4 are complete.

## Requirements

### 1. Test: `AgentRequest::new()` construction defaults
Create a test named `agent_request_new_sets_uuid_and_defaults` that:
- Calls `AgentRequest::new("hello".to_string())`.
- Asserts `request.id != uuid::Uuid::nil()` (the auto-generated v4 UUID is non-nil).
- Asserts `request.input == "hello"`.
- Asserts `request.context.is_none()`.
- Asserts `request.caller.is_none()`.

### 2. Test: `AgentRequest` JSON round-trip with all fields
Create a test named `agent_request_json_round_trip` that:
- Constructs an `AgentRequest` with all fields explicitly populated: a known `Uuid`, a non-empty `input`, a `Some(serde_json::json!({...}))` for `context`, and a `Some("orchestrator".to_string())` for `caller`.
- Serializes to a JSON string via `serde_json::to_string(&request)`.
- Deserializes back via `serde_json::from_str::<AgentRequest>(&json)`.
- Asserts `original == deserialized` using `assert_eq!`.

### 3. Test: `AgentResponse::success()` construction defaults
Create a test named `agent_response_success_sets_defaults` that:
- Calls `AgentResponse::success(uuid::Uuid::new_v4(), serde_json::json!("result"))`.
- Asserts `response.confidence == 1.0`.
- Asserts `response.escalated == false`.
- Asserts `response.tool_calls.is_empty()`.
- Asserts `response.output == serde_json::json!("result")`.

### 4. Test: `AgentResponse` JSON round-trip with tool calls
Create a test named `agent_response_json_round_trip_with_tool_calls` that:
- Constructs an `AgentResponse` with `tool_calls` containing at least one `ToolCallRecord` (with a `tool_name`, a JSON object `input`, and a JSON object `output`).
- Sets non-default values for `confidence` (e.g., `0.85`) and `escalated` (`true`).
- Serializes to JSON and deserializes back.
- Asserts `original == deserialized`.

### 5. Test: `AgentError` Display output for each variant
Create a test named `agent_error_display_contains_expected_substrings` that:
- Constructs each `AgentError` variant:
  - `ToolCallFailed { tool: "web_search".into(), reason: "timeout".into() }` — assert Display output contains `"web_search"` and `"timeout"`.
  - `ConfidenceTooLow { confidence: 0.3, threshold: 0.75 }` — assert Display output contains `"0.3"` and `"0.75"`.
  - `MaxTurnsExceeded { turns: 10 }` — assert Display output contains `"10"`.
  - `Internal("something broke".into())` — assert Display output contains `"something broke"`.
- Uses `format!("{}", error)` or `.to_string()` and asserts via `.contains()`.

### 6. Test: `HealthStatus` JSON round-trip for each variant
Create a test named `health_status_serialize_deserialize_each_variant` that:
- For each variant (`Healthy`, `Degraded("slow db".into())`, `Unhealthy("disk full".into())`):
  - Serializes to JSON via `serde_json::to_string`.
  - Deserializes back via `serde_json::from_str::<HealthStatus>`.
  - Asserts `original == deserialized`.

### 7. Test: `ToolCallRecord` JSON round-trip with nested JSON values
Create a test named `tool_call_record_json_round_trip_with_nested_values` that:
- Constructs a `ToolCallRecord` with:
  - `tool_name: "execute_sql".to_string()`
  - `input: serde_json::json!({"query": "SELECT * FROM orders", "params": [1, 2, 3]})`
  - `output: serde_json::json!({"rows": [{"id": 1, "name": "Widget"}], "count": 1})`
- Serializes to JSON and deserializes back.
- Asserts `original == deserialized`.

## Implementation Details

### File to create
```
crates/agent-sdk/tests/envelope_types_test.rs
```

### Imports
```rust
use agent_sdk::{AgentError, AgentRequest, AgentResponse, HealthStatus, ToolCallRecord};
use serde_json::json;
use uuid::Uuid;
```

`serde_json` and `uuid` are regular dependencies of the `agent-sdk` crate, so they are available to integration tests transitively. However, since the test file uses `serde_json::json!` and `uuid::Uuid` directly, these crates must also be resolvable in the test's extern scope. Because they are listed in `[dependencies]` (not `[dev-dependencies]`), Cargo makes them available to integration tests automatically.

### Test naming convention
Follow `.claude/rules/testing.md`: use descriptive test names that explain what is being verified. Each test has a single primary focus. The `AgentError` Display test checks multiple variants but they collectively verify one behavior (the Display implementation).

### Floating-point comparison
`confidence` is `f32`. Direct `==` comparison is acceptable for values like `1.0` which is exactly representable in f32. For values like `0.85`, use `assert!((value - expected).abs() < f32::EPSILON)` if direct comparison proves unreliable; start with `assert_eq!` for `1.0` and epsilon comparison for others.

### JSON vs YAML
Unlike the existing `skill_manifest_test.rs` which uses `serde_yaml`, all tests in this file use `serde_json`. This reflects the intended serialization format for envelope types (JSON over the wire), versus skill manifests (YAML config files on disk).

### Type signatures reference (from task breakdown)
- `AgentRequest { id: Uuid, input: String, context: Option<serde_json::Value>, caller: Option<String> }`
- `AgentRequest::new(input: String) -> Self`
- `AgentResponse { id: Uuid, output: serde_json::Value, confidence: f32, escalated: bool, tool_calls: Vec<ToolCallRecord> }`
- `AgentResponse::success(id: Uuid, output: serde_json::Value) -> Self`
- `ToolCallRecord { tool_name: String, input: serde_json::Value, output: serde_json::Value }`
- `AgentError` enum: `ToolCallFailed { tool, reason }`, `ConfidenceTooLow { confidence, threshold }`, `MaxTurnsExceeded { turns }`, `Internal(String)`
- `HealthStatus` enum: `Healthy`, `Degraded(String)`, `Unhealthy(String)`

## Dependencies
- **Blocked by:** "Update `lib.rs` module declarations and re-exports" (Group 4) — all five types must be publicly exported from the crate before integration tests can import them
- **Blocking:** None. This is the final functional task. The only remaining task is the verification step (run `cargo check`, `cargo clippy`, `cargo test`).

## Risks & Edge Cases

1. **`serde_json` and `uuid` availability in integration tests**: Integration tests in `tests/` are compiled as separate crates. They can use dependencies listed in the parent crate's `[dependencies]` and `[dev-dependencies]`. Since `serde_json` and `uuid` are in `[dependencies]`, they are available. However, if for any reason the extern crate resolution fails, add `serde_json = "1"` and `uuid = "1"` to `[dev-dependencies]` as well.

2. **`AgentError` does not derive `JsonSchema`**: Per the task breakdown, `AgentError` derives `Serialize, Deserialize` but NOT `JsonSchema`. Test 5 only tests `Display`, not serialization, so this is fine. If a future task adds serialization tests for `AgentError`, the derives are already present.

3. **`f32` precision for `confidence`**: `1.0_f32` is exactly representable and safe for `assert_eq!`. Values like `0.85_f32` may have minor representation differences after JSON round-trip (since JSON uses decimal text and `f32` is binary). If the round-trip test in requirement 4 fails on `confidence`, switch to an epsilon comparison for that field and compare the rest with `assert_eq!` on a struct with a known confidence value.

4. **UUID determinism**: Test 1 uses `AgentRequest::new()` which generates a random UUID. The test can only assert it is non-nil, not a specific value. Test 2 constructs a request with a known UUID (e.g., `Uuid::nil()` or a hardcoded value) for deterministic round-trip comparison.

5. **Enum serialization format for `HealthStatus`**: Serde's default enum serialization for externally tagged enums produces `{"Healthy":null}` for unit variants and `{"Degraded":"reason"}` for newtype variants. The round-trip test is format-agnostic (it only checks `original == deserialized`), so it works regardless of the specific JSON shape. If the enum has `#[serde(rename_all = "...")]` or other attributes, the round-trip still holds.

6. **`PartialEq` on `serde_json::Value`**: `serde_json::Value` implements `PartialEq`, so `assert_eq!` works on structs containing it. Floating-point numbers in JSON values are compared as `f64`, which is generally safe for the simple values used in tests.

## Verification
After implementation, run the following commands (per CLAUDE.md):

```bash
cargo check -p agent-sdk    # Ensure the test file compiles
cargo clippy -p agent-sdk   # No warnings
cargo test -p agent-sdk     # All tests pass
```

Specifically, confirm these seven tests exist and pass:
- `envelope_types_test::agent_request_new_sets_uuid_and_defaults`
- `envelope_types_test::agent_request_json_round_trip`
- `envelope_types_test::agent_response_success_sets_defaults`
- `envelope_types_test::agent_response_json_round_trip_with_tool_calls`
- `envelope_types_test::agent_error_display_contains_expected_substrings`
- `envelope_types_test::health_status_serialize_deserialize_each_variant`
- `envelope_types_test::tool_call_record_json_round_trip_with_nested_values`

Additionally, verify that existing tests in `skill_manifest_test.rs` continue to pass (no regressions).
