# Task Breakdown: Implement startup-time skill validation

> Add strict validation to the skill-loader so that invalid skill files cause the agent to fail at startup with collected error messages, not at runtime.

## Group 1 — Type changes in agent-sdk

_Tasks in this group can be done in parallel._

- [x] **Change `escalate_to` from `String` to `Option<String>` in `Constraints`** `[S]`
      The `Constraints.escalate_to` field is currently `String`, but skills without an escalation target have no meaningful value to provide. Change it to `Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` so it can be omitted from skill files. This is a prerequisite for validation because the validator needs to distinguish "no escalation" from "empty string escalation target." Update the existing test fixtures in `crates/agent-sdk/tests/skill_manifest_test.rs` to wrap `escalate_to` values in `Some(...)` and verify that omitting the field deserializes to `None`.
      Files: `crates/agent-sdk/src/constraints.rs`, `crates/agent-sdk/tests/skill_manifest_test.rs`
      Blocking: "Implement validate function", "Write validation tests"

- [x] **Define allowed output format constants** `[S]`
      Based on the README (`structured_json`), existing tests (`json`, `text`), define a canonical list as a public constant array in `output_schema.rs`: `pub const ALLOWED_OUTPUT_FORMATS: &[&str] = &["json", "structured_json", "text"];`. This gives the validator a single source of truth to check against.
      Files: `crates/agent-sdk/src/output_schema.rs`
      Blocking: "Implement validate function", "Write validation tests"

## Group 2 — Define validation trait for tool registry

_Can be done in parallel with Group 1._

- [x] **Define `ToolExists` trait in skill-loader** `[S]`
      Tool name validation requires checking if a tool is registered, but `tool-registry` (issue #8) is a placeholder. To decouple, define a trait in `crates/skill-loader/src/lib.rs` (or a dedicated `validation.rs` module): `pub trait ToolExists { fn tool_exists(&self, name: &str) -> bool; }`. The `validate` function will accept `&dyn ToolExists` rather than a concrete `ToolRegistry`. Also provide a `struct AllToolsExist;` stub that always returns `true`, for use in tests that are not exercising tool-name validation specifically.
      Files: `crates/skill-loader/src/lib.rs` (or `crates/skill-loader/src/validation.rs`)
      Blocking: "Implement validate function"

## Group 3 — Core validation logic

_Depends on: Group 1 and Group 2._

- [x] **Implement `validate` function** `[M]`
      Create the core validation function in the skill-loader crate: `pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError>`. This function collects all violations into a `Vec<String>` and returns `Err(SkillError::ValidationError { skill_name, reasons })` if any are found. Implement the following checks, each as a small helper function (respecting the 50-line rule):

      1. **Required string fields non-empty:** `manifest.name`, `manifest.version`, `manifest.model.provider`, `manifest.model.name` must not be empty or whitespace-only.
      2. **Preamble non-empty:** `manifest.preamble` must not be blank.
      3. **`confidence_threshold` in [0.0, 1.0]:** Check inclusive bounds.
      4. **`max_turns` > 0:** Check not zero (it is `u32` so cannot be negative).
      5. **Tool existence:** For each name in `manifest.tools`, call `tool_checker.tool_exists(name)`. Collect all missing tool names.
      6. **Output format recognized:** Check `manifest.output.format` against `ALLOWED_OUTPUT_FORMATS`.
      7. **`escalate_to` non-empty if present:** If `Some(name)`, validate `name` is non-empty. Full cross-agent validation deferred to orchestrator (TODO comment).

      Note: Depends on issue #5 having defined `SkillError` with a `ValidationError` variant. If not yet complete, define `SkillError` as part of this task.
      Files: `crates/skill-loader/src/lib.rs` (or `crates/skill-loader/src/validation.rs`)
      Blocked by: "Change `escalate_to` to `Option<String>`", "Define allowed output format constants", "Define `ToolExists` trait"
      Blocking: "Integrate validation into `SkillLoader::load()`", "Write validation tests"

## Group 4 — Integration

_Depends on: Group 3. Also depends on issue #5 (skill-loader `load()` method) being complete._

- [x] **Integrate validation into `SkillLoader::load()`** `[S]`
      After `load()` successfully parses frontmatter and constructs a `SkillManifest` (issue #5), call `validate(&manifest, &self.tool_checker)?` before returning. The `SkillLoader` struct needs access to a `tool_checker: Box<dyn ToolExists>` (or `Arc<dyn ToolExists>`). Update `SkillLoader::new()` to accept this parameter. If issue #5 is not yet landed, defer this task.
      Files: `crates/skill-loader/src/lib.rs`
      Blocked by: "Implement validate function", issue #5 completion
      Blocking: "Write integration tests"

## Group 5 — Tests and verification

_Depends on: Group 3 (unit tests) and Group 4 (integration tests)._

- [x] **Write validation unit tests** `[M]`
      Create `crates/skill-loader/tests/validation_test.rs`. Use a helper that builds a valid `SkillManifest` baseline, then mutate one field at a time:

      1. Valid manifest passes (happy path)
      2. `confidence_threshold` of 1.5 fails
      3. `confidence_threshold` of -0.1 fails
      4. `confidence_threshold` of 0.0 and 1.0 pass (boundary)
      5. `max_turns` of 0 fails
      6. `max_turns` of 1 passes (boundary)
      7. Unknown tool name fails (use a `ToolExists` impl that rejects specific names)
      8. Empty `name` field fails
      9. Empty `version` field fails
      10. Empty `preamble` field fails
      11. Empty `model.provider` fails
      12. Empty `model.name` fails
      13. Unrecognized output format (e.g., `"raw"`) fails
      14. Recognized output formats (`"json"`, `"structured_json"`, `"text"`) pass
      15. Multiple violations collected in one error (e.g., empty name AND threshold out of range)
      16. `escalate_to` of `Some("")` (empty string) fails
      17. `escalate_to` of `None` passes

      Files: `crates/skill-loader/tests/validation_test.rs`
      Blocked by: "Implement validate function", "Change `escalate_to` to `Option<String>`"
      Blocking: None

- [x] **Write integration test for load-with-validation** `[S]`
      Create a temp directory with a `.md` skill file containing invalid frontmatter (e.g., `confidence_threshold: 2.0`), call `SkillLoader::load()`, and assert it returns `SkillError::ValidationError` with the expected violation message.
      Files: `crates/skill-loader/tests/validation_test.rs` (or `crates/skill-loader/tests/loader_test.rs`)
      Blocked by: "Integrate validation into `SkillLoader::load()`"
      Blocking: None

- [x] **Run `cargo check`, `cargo clippy`, `cargo test`** `[S]`
      Run the full verification suite per CLAUDE.md. Ensure no clippy warnings, all tests pass, and all crates compile cleanly.
      Files: (none — command-line only)
      Blocked by: all previous tasks
      Blocking: None
