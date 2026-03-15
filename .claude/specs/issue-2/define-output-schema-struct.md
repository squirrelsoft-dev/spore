# Spec: Define `OutputSchema` struct

> From: .claude/tasks/issue-2.md

## Objective

Create a new module `output_schema.rs` in `crates/agent-sdk/src/` containing an `OutputSchema` struct that represents the `output` section of a YAML skill file. This struct captures the output format and a string-keyed, string-valued schema map describing the expected output fields.

## Current State

- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No domain types exist yet.
- `crates/agent-sdk/Cargo.toml` declares the crate with edition 2024 and has no dependencies listed. The `serde`, `schemars` dependencies required by derive macros do not exist yet and must be added by the prerequisite task "Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`" before this task can compile.
- The canonical YAML skill file example in `README.md` (lines 46-52) defines the output section as:
  ```yaml
  output:
    format: structured_json
    schema:
      sql: string
      explanation: string
      confidence: float
      source: string
  ```
  The `schema` field is a flat string-to-string map. Note that value types like `float` are represented as strings in the schema definition, not as native Rust types.

## Requirements

1. Create the file `crates/agent-sdk/src/output_schema.rs`.
2. Import `std::collections::HashMap`.
3. Import derive macros: `serde::Serialize`, `serde::Deserialize`, and `schemars::JsonSchema`.
4. Define a public struct `OutputSchema` with:
   - `format: String` -- the output format identifier (e.g., `"structured_json"`).
   - `schema: HashMap<String, String>` -- maps output field names to their type descriptors as plain strings.
5. Derive `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema` on the struct.
6. Both fields must be public.
7. No additional methods, trait implementations, or helper functions.

## Implementation Details

- The file should contain only the necessary imports, the struct definition, and its derive attributes. No `mod` declaration or `pub use` re-export should be added in `lib.rs` by this task -- that is handled by the later "Update lib.rs module declarations and re-exports" task.
- Field names (`format`, `schema`) must match the YAML keys exactly so `serde` can deserialize without rename attributes.
- `HashMap<String, String>` is the correct representation because the YAML schema map values are type descriptor strings (e.g., `"string"`, `"float"`), not Rust types. This keeps the struct simple and avoids introducing a custom enum for type descriptors at this stage.
- No `#[serde(rename_all = ...)]` attribute is needed since the Rust field names already match the lowercase YAML keys.
- No default values or `Option` wrapping is specified -- both fields are required.

## Dependencies

- **Blocked by:** "Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`" (Group 1). Without these crate dependencies, the derive macros will not resolve.
- **Blocks:** "Define `SkillManifest` struct" (Group 3), which composes `OutputSchema` as its `output` field.
- **Parallel with:** "Define `ModelConfig` struct" and "Define `Constraints` struct" (both Group 2). These three tasks have no interdependencies and can be implemented in any order.

## Risks & Edge Cases

- **`format` is a reserved-adjacent name:** `format` is not a Rust keyword, but it shadows the `std::fmt::format` function. This is harmless for a struct field and requires no mitigation.
- **Schema value types are stringly-typed:** Representing type descriptors as `String` means there is no compile-time validation of allowed values (e.g., `"string"`, `"float"`, `"int"`). This is intentional for this task -- type validation, if desired, would be a separate concern.
- **Empty schema map:** A skill file could have an `output` section with `format` but an empty `schema` map. `HashMap` handles this naturally (an empty map), so no special handling is needed. The deserialization test task (Group 4) will cover this edge case.
- **HashMap ordering:** `HashMap` does not preserve insertion order. If round-trip YAML serialization tests require stable key ordering, the test should compare field-by-field rather than comparing serialized strings. This is a concern for the test task, not this task.

## Verification

Since this task only creates a new file and does not wire it into `lib.rs`, the file cannot be independently compiled or tested in isolation via `cargo build`. Verification at this stage is limited to:

1. Confirm the file exists at `crates/agent-sdk/src/output_schema.rs`.
2. Confirm the struct has the correct fields, types, and derive attributes by reading the file.
3. Full compilation verification (`cargo check`, `cargo clippy`, `cargo test`) will occur after the Group 3 task wires the module into `lib.rs` and the Group 1 dependency task adds `serde`/`schemars` to `Cargo.toml`.
