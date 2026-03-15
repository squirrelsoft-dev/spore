# Spec: Write deserialization tests

> From: .claude/tasks/issue-2.md

## Objective

Add an integration test file at `crates/agent-sdk/tests/skill_manifest_test.rs` that validates serde round-tripping of `SkillManifest` and its sub-types (`ModelConfig`, `Constraints`, `OutputSchema`) against the canonical YAML skill file example from the README. Also add `PartialEq` derive to all four structs to support equality assertions.

## Current State

- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No types are defined yet.
- `crates/agent-sdk/Cargo.toml` has no dependencies listed (no `serde`, `serde_yaml`, or `schemars`).
- No `tests/` directory exists under `crates/agent-sdk/`.
- The four structs (`SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`) do not exist yet. This task is blocked by Groups 1-3 in the task breakdown (dependency additions, type definitions, and `lib.rs` re-exports).

## Requirements

### 1. Add `PartialEq` derive to all four structs

Each of the four structs must have `PartialEq` added to its derive list:
- `ModelConfig` in `crates/agent-sdk/src/model_config.rs`
- `Constraints` in `crates/agent-sdk/src/constraints.rs`
- `OutputSchema` in `crates/agent-sdk/src/output_schema.rs`
- `SkillManifest` in `crates/agent-sdk/src/skill_manifest.rs`

This is required for `assert_eq!` comparisons in the round-trip test.

### 2. Ensure `serde_yaml` is in dev-dependencies

`crates/agent-sdk/Cargo.toml` must include `serde_yaml` under `[dev-dependencies]`. This should already be done by the Group 1 task, but verify before proceeding.

### 3. Test: Deserialize the README YAML example

Create a test named `deserialize_readme_skill_file` that:
- Embeds the exact YAML from README lines 19-53 as a string literal (reproduced below for reference):
  ```yaml
  name: cogs-analyst
  version: 1.0.0
  description: Handles COGS-related finance queries

  model:
    provider: anthropic
    name: claude-sonnet-4-6
    temperature: 0.1

  preamble: |
    You are a finance analyst specializing in Cost of Goods Sold analysis.
    Never speculate. If confidence is below threshold, escalate.

  tools:
    - get_account_groups
    - execute_sql
    - query_store_lookup

  constraints:
    max_turns: 5
    confidence_threshold: 0.75
    escalate_to: general-finance-agent
    allowed_actions:
      - read
      - query

  output:
    format: structured_json
    schema:
      sql: string
      explanation: string
      confidence: float
      source: string
  ```
- Calls `serde_yaml::from_str::<SkillManifest>(yaml_str)` and unwraps.
- Asserts every field individually:
  - `manifest.name == "cogs-analyst"`
  - `manifest.version == "1.0.0"`
  - `manifest.description == "Handles COGS-related finance queries"`
  - `manifest.model.provider == "anthropic"`
  - `manifest.model.name == "claude-sonnet-4-6"`
  - `manifest.model.temperature == 0.1` (use `f64` comparison; 0.1 is exact in YAML)
  - `manifest.preamble` starts with `"You are a finance analyst"` and contains `"Never speculate"` (the YAML block scalar will include a trailing newline)
  - `manifest.tools == vec!["get_account_groups", "execute_sql", "query_store_lookup"]`
  - `manifest.constraints.max_turns == 5`
  - `manifest.constraints.confidence_threshold == 0.75`
  - `manifest.constraints.escalate_to == "general-finance-agent"`
  - `manifest.constraints.allowed_actions == vec!["read", "query"]`
  - `manifest.output.format == "structured_json"`
  - `manifest.output.schema` contains exactly 4 entries: `sql -> "string"`, `explanation -> "string"`, `confidence -> "float"`, `source -> "string"`

### 4. Test: Serialize then deserialize round-trip

Create a test named `serialize_deserialize_round_trip` that:
- Constructs a `SkillManifest` programmatically with known values (can differ from the README example).
- Serializes it to a YAML string via `serde_yaml::to_string(&manifest)`.
- Deserializes the resulting YAML string back via `serde_yaml::from_str::<SkillManifest>(&yaml_str)`.
- Asserts `original == deserialized` using `assert_eq!` (relies on `PartialEq` derive).

### 5. Test: Empty tools list

Create a test named `deserialize_empty_tools_list` that:
- Provides a minimal valid YAML with `tools: []`.
- Deserializes successfully.
- Asserts `manifest.tools.is_empty()`.

### 6. Test: Empty schema map

Create a test named `deserialize_empty_schema_map` that:
- Provides a minimal valid YAML with `output.schema` as an empty map (`schema: {}`).
- Deserializes successfully.
- Asserts `manifest.output.schema.is_empty()`.

## Implementation Details

### File structure

```
crates/agent-sdk/tests/skill_manifest_test.rs
```

This is a Rust integration test file (lives in `tests/`, not `src/`). It imports from the crate's public API:

```rust
use agent_sdk::SkillManifest;
use std::collections::HashMap;
```

### Test naming convention

Follow `.claude/rules/testing.md`: use descriptive test names that explain what is being verified. Each test should have a single primary assertion focus, though multiple field assertions within the README deserialization test are acceptable since they collectively verify one operation (deserializing the canonical example).

### Preamble field handling

The YAML block scalar (`|`) preserves newlines and adds a trailing newline. The assertion for `manifest.preamble` must account for this. Use `.contains()` or `.trim()` comparisons rather than exact string equality to avoid brittleness around trailing whitespace.

### HashMap ordering

`OutputSchema.schema` is a `HashMap<String, String>`. Do not assert insertion order. Assert individual key-value pairs using `.get("key")` or assert the length and check each entry individually.

### Floating-point comparison

For `temperature` and `confidence_threshold`, direct `==` comparison is acceptable because `0.1` and `0.75` are the exact values serialized by serde_yaml and are representable with sufficient precision in f64. If this proves flaky, fall back to an epsilon comparison, but start with direct equality.

### Minimal valid YAML for edge case tests

The edge case tests (empty tools, empty schema) need all required fields present. Construct a minimal but complete YAML string with placeholder values for all non-optional fields. Example skeleton:

```yaml
name: test-agent
version: 0.1.0
description: test
model:
  provider: test
  name: test-model
  temperature: 0.5
preamble: test prompt
tools: []
constraints:
  max_turns: 1
  confidence_threshold: 0.5
  escalate_to: nobody
  allowed_actions: []
output:
  format: json
  schema: {}
```

## Dependencies

- **Blocked by (all must be complete first):**
  - Group 1: `serde`, `schemars` in `[dependencies]` and `serde_yaml` in `[dev-dependencies]` of `crates/agent-sdk/Cargo.toml`
  - Group 2: `ModelConfig`, `Constraints`, `OutputSchema` struct definitions with `Serialize`, `Deserialize` derives
  - Group 3: `SkillManifest` struct definition and `lib.rs` re-exports so that `use agent_sdk::SkillManifest` works from integration tests

- **Blocking:** Nothing. This is the final functional task before verification.

- **New dependency justification:** `serde_yaml` is added as a dev-dependency only. It is required to test YAML deserialization, which is the primary use case of these types. It does not affect the production binary.

## Risks & Edge Cases

1. **Preamble trailing newline:** YAML block scalars (`|`) include a trailing newline. If the struct field is compared with an exact string that lacks the trailing newline, the test will fail. Mitigation: use `starts_with` / `contains` or compare against a string that includes the trailing `\n`.

2. **HashMap key ordering in serialization:** When round-tripping through YAML, `HashMap` serialization order is nondeterministic. The `PartialEq` derive on `HashMap` compares by content regardless of order, so `assert_eq!` on the deserialized struct is safe. However, comparing raw YAML strings would fail. Mitigation: only compare deserialized structs, never raw YAML strings.

3. **Floating-point precision:** `serde_yaml` may serialize `0.1` as `0.1` or with additional decimal digits. Round-trip equality depends on `f64` bit-exact representation surviving serialization. `0.1` and `0.75` are standard enough that this is unlikely to be an issue, but it is worth noting.

4. **All fields are required:** The current struct definitions (per the task breakdown) have no `Option` fields. Every edge case test must still provide all fields even when testing one specific empty value. If any field becomes optional later, these tests may need updating.

5. **`PartialEq` on `f64`:** Deriving `PartialEq` on structs containing `f64` fields works (Rust's `f64` implements `PartialEq`), but it does not implement `Eq` (because `NaN != NaN`). The tests should not derive `Eq` unless explicitly needed, and should avoid `NaN` values in test data.

## Verification

After implementation, run the following commands (per CLAUDE.md):

```bash
cargo check          # Ensure all types compile with PartialEq derive
cargo clippy         # No warnings
cargo test           # All four tests pass
```

Specifically, confirm these four tests exist and pass:
- `skill_manifest_test::deserialize_readme_skill_file`
- `skill_manifest_test::serialize_deserialize_round_trip`
- `skill_manifest_test::deserialize_empty_tools_list`
- `skill_manifest_test::deserialize_empty_schema_map`
