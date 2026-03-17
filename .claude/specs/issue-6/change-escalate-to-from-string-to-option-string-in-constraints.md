# Spec: Change `escalate_to` from `String` to `Option<String>` in `Constraints`

> From: .claude/tasks/issue-6.md

## Objective
Change the `Constraints.escalate_to` field from `String` to `Option<String>` so that skill files can omit the field entirely when no escalation target exists. This allows the future validator (Group 3 of issue #6) to distinguish between "no escalation configured" (`None`) and "an empty escalation target was provided" (`Some("")`), which is required for meaningful validation. Without this change, skills without an escalation target must use an arbitrary placeholder string like `"nobody"` or an empty string, both of which are semantically incorrect.

## Current State

**`crates/agent-sdk/src/constraints.rs`** defines the `Constraints` struct:

```rust
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,
    pub escalate_to: String,
    pub allowed_actions: Vec<String>,
}
```

The `escalate_to` field is a required `String`. Every YAML fixture in the test file provides a value:
- `CANONICAL_YAML` uses `escalate_to: human_reviewer`
- `serialize_deserialize_round_trip` uses `escalate_to: "senior_analyst".to_string()`
- `deserialize_empty_tools_list` uses `escalate_to: nobody`
- `deserialize_empty_schema_map` uses `escalate_to: fallback`

**`crates/agent-sdk/tests/micro_agent_test.rs`** also constructs a `Constraints` with `escalate_to: "human".to_string()`.

There are no existing `skip_serializing_if` or `serde(default)` attributes on any field in `Constraints`.

## Requirements

1. The `escalate_to` field in `Constraints` must be changed from `String` to `Option<String>`.
2. The field must have the serde attribute `#[serde(default, skip_serializing_if = "Option::is_none")]` so that:
   - Omitting `escalate_to` from a YAML skill file deserializes to `None` (via `default`).
   - Serializing a `Constraints` with `escalate_to: None` omits the field from the output (via `skip_serializing_if`).
   - Providing `escalate_to: some_agent` deserializes to `Some("some_agent".to_string())`.
3. All existing test assertions referencing `escalate_to` in `skill_manifest_test.rs` must be updated to use `Some(...)`.
4. A new test must verify that omitting `escalate_to` from YAML deserializes to `None`.
5. The `serialize_deserialize_round_trip` test must continue to pass, demonstrating that `Some("value")` round-trips correctly through serialize/deserialize.
6. A new round-trip test (or extension of the existing one) must verify that `None` round-trips correctly: serializing with `escalate_to: None` produces YAML without the key, and deserializing that YAML produces `None`.
7. The `micro_agent_test.rs` fixture must also be updated to use `Some("human".to_string())`.
8. `cargo check`, `cargo clippy`, and `cargo test` must all pass after the change.

## Implementation Details

### Files to modify

**`crates/agent-sdk/src/constraints.rs`**
- Add the `#[serde(default, skip_serializing_if = "Option::is_none")]` attribute to the `escalate_to` field.
- Change the type from `String` to `Option<String>`.

**`crates/agent-sdk/tests/skill_manifest_test.rs`**
- In `deserialize_readme_skill_file`: Change the assertion from `assert_eq!(manifest.constraints.escalate_to, "human_reviewer")` to `assert_eq!(manifest.constraints.escalate_to, Some("human_reviewer".to_string()))`.
- In `serialize_deserialize_round_trip`: Change the struct construction from `escalate_to: "senior_analyst".to_string()` to `escalate_to: Some("senior_analyst".to_string())`.
- In `deserialize_empty_tools_list` and `deserialize_empty_schema_map`: No assertion changes needed on `escalate_to` (the field is present in the YAML but not asserted). However, if the tests are left as-is, they will still work since the YAML provides a value. No changes required unless assertions are added.
- Add a new test `deserialize_missing_escalate_to` that uses a YAML fixture without an `escalate_to` key in the `constraints` block, and asserts that `manifest.constraints.escalate_to` is `None`.
- Add a new test (or extend existing) `round_trip_none_escalate_to` that constructs a `Constraints` with `escalate_to: None`, serializes to YAML, verifies the YAML does not contain the `escalate_to` key, then deserializes and asserts `escalate_to` is `None`.

**`crates/agent-sdk/tests/micro_agent_test.rs`**
- Update the `Constraints` construction in the test helper to use `escalate_to: Some("human".to_string())`.

### Key type changes

Before:
```rust
pub escalate_to: String,
```

After:
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub escalate_to: Option<String>,
```

### Integration points
- Any code that reads `constraints.escalate_to` as a `String` will need to handle `Option<String>` instead. Currently, only test code accesses this field directly. The future `validate` function (Group 3) will consume this as `Option<String>` by design.
- The `JsonSchema` derive will automatically generate the correct schema for `Option<String>` (nullable string).

## Dependencies

- **Blocked by:** Nothing. This is in Group 1 of issue #6 and has no prerequisites.
- **Blocking:**
  - "Implement validate function" (Group 3) -- the validator checks `escalate_to` as `Option<String>`, distinguishing `None` from `Some("")`.
  - "Write validation tests" (Group 5) -- tests include `escalate_to` of `Some("")` failing and `None` passing, which requires the `Option` type.

## Risks & Edge Cases

1. **Downstream consumers of `Constraints.escalate_to`:** Any code (inside or outside the test files) that accesses `escalate_to` as a `String` will fail to compile. The `micro_agent_test.rs` file is a known instance and must be updated. A codebase-wide grep for `escalate_to` should be performed to catch all sites.
2. **YAML with `escalate_to: ~` or `escalate_to: null`:** YAML's null literal deserializes to `None` for `Option<String>` with serde_yaml. This is acceptable and correct behavior.
3. **YAML with `escalate_to: ""`:** This will deserialize to `Some("".to_string())`, not `None`. This is intentional -- the validator will reject `Some("")` as invalid, which is the whole point of this change.
4. **Serialization of `Some("")`:** Will produce `escalate_to: ''` in YAML output. This is technically valid but semantically wrong; the validator (not this task) is responsible for rejecting it.
5. **JsonSchema output change:** The generated JSON schema will change from `{"type": "string"}` to a nullable string. Consumers of the schema (if any) should be aware, though no schema consumers exist today.

## Verification

1. `cargo check` passes with no errors across all crates.
2. `cargo clippy` passes with no warnings.
3. `cargo test` passes, including:
   - All four existing tests in `skill_manifest_test.rs` (updated for `Option`).
   - The new `deserialize_missing_escalate_to` test confirming `None` deserialization.
   - The new `round_trip_none_escalate_to` test confirming `None` survives round-trip.
   - The `micro_agent_test.rs` tests still pass with the updated fixture.
4. Serializing a `Constraints` with `escalate_to: None` produces YAML that does not contain the string `escalate_to`.
5. Serializing a `Constraints` with `escalate_to: Some("agent_name".to_string())` produces YAML containing `escalate_to: agent_name`.
