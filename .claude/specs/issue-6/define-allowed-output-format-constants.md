# Spec: Define allowed output format constants

> From: .claude/tasks/issue-6.md

## Objective
Establish a single source of truth for valid output format values by defining a public constant array in the `output_schema` module. Downstream code (the `validate` function in `skill-loader`) will check `OutputSchema.format` against this constant instead of hard-coding format strings in multiple places.

## Current State
- `OutputSchema` is defined in `crates/agent-sdk/src/output_schema.rs` with a `format: String` field and a `schema: HashMap<String, String>` field. No validation or enumeration of legal format values exists.
- `crates/agent-sdk/src/lib.rs` re-exports `OutputSchema` from the `output_schema` module. It does not currently re-export any constants from that module.
- The README skill-file example uses `format: structured_json`.
- Existing tests use `format: "json"` (in `skill_manifest_test.rs` and `micro_agent_test.rs`) and `format: "text"` (in `skill_manifest_test.rs`).
- One test (`deserialize_empty_schema_map`) uses `format: "raw"`, which is not in the canonical set. This test is only exercising empty-schema deserialization and does not assert on the format value, so the format string is incidental and should be updated to a valid value (but that update is outside the scope of this task).

## Requirements
1. Add a public constant `ALLOWED_OUTPUT_FORMATS` of type `&[&str]` to `crates/agent-sdk/src/output_schema.rs` containing exactly three entries: `"json"`, `"structured_json"`, and `"text"`, in that order.
2. Re-export `ALLOWED_OUTPUT_FORMATS` from `crates/agent-sdk/src/lib.rs` so downstream crates (particularly `skill-loader`) can import it via `agent_sdk::ALLOWED_OUTPUT_FORMATS`.
3. The constant must be usable at compile time (i.e., `const` or `static`, not computed at runtime).
4. No new dependencies are required.
5. All existing tests must continue to pass without modification (the constant is purely additive).

## Implementation Details
- **File to modify: `crates/agent-sdk/src/output_schema.rs`**
  - Add the following line above or below the `OutputSchema` struct definition:
    ```rust
    pub const ALLOWED_OUTPUT_FORMATS: &[&str] = &["json", "structured_json", "text"];
    ```
- **File to modify: `crates/agent-sdk/src/lib.rs`**
  - Add a re-export for the constant:
    ```rust
    pub use output_schema::ALLOWED_OUTPUT_FORMATS;
    ```
- No new files are created.
- No changes to `OutputSchema` struct fields, derives, or serde attributes.

## Dependencies
- Blocked by: nothing (this is a Group 1 task with no predecessors)
- Blocking: "Implement validate function" (Group 3) which will use `ALLOWED_OUTPUT_FORMATS` to check `manifest.output.format`; "Write validation tests" (Group 5) which will assert that recognized and unrecognized formats produce the correct validation outcome

## Risks & Edge Cases
- **Future format additions**: If a new format is introduced, it must be added to this constant. This is intentional -- a single place to update is the goal.
- **Case sensitivity**: The constant stores lowercase strings. The validator (a separate task) must decide whether to normalize case before comparison. This task does not enforce casing policy; it only defines the canonical lowercase values.
- **`"raw"` format in existing test**: The `deserialize_empty_schema_map` test uses `format: "raw"`. This will not break because the constant is additive and no validation runs during deserialization. However, a follow-up task (writing validation tests or updating test fixtures) should change `"raw"` to a valid format to avoid confusion.

## Verification
1. `cargo check -p agent-sdk` compiles without errors or warnings.
2. `cargo clippy -p agent-sdk` reports no new warnings.
3. `cargo test -p agent-sdk` passes all existing tests (the change is purely additive).
4. `ALLOWED_OUTPUT_FORMATS` is importable from `agent_sdk` in downstream crates (verified by `cargo check -p skill-loader` if it imports the symbol, or by manual inspection of `lib.rs` re-exports).
