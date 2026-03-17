# Spec: Write validation unit tests

> From: .claude/tasks/issue-6.md

## Objective

Create a comprehensive suite of unit tests for the `validate` function in the `skill-loader` crate. These tests verify that every semantic validation rule is correctly enforced: confidence threshold bounds, max turns minimum, tool existence checks, required-field non-emptiness, output format recognition, escalate_to constraints, and the accumulation of multiple violations into a single error. The tests ensure the validator catches invalid skill files at startup rather than at runtime.

## Current State

- **`SkillManifest`** is defined in `crates/agent-sdk/src/skill_manifest.rs` with fields: `name: String`, `version: String`, `description: String`, `model: ModelConfig`, `preamble: String`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`. It derives `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.

- **`ModelConfig`** (`crates/agent-sdk/src/model_config.rs`): `provider: String`, `name: String`, `temperature: f64`.

- **`Constraints`** (`crates/agent-sdk/src/constraints.rs`): `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: String`, `allowed_actions: Vec<String>`. Note: `escalate_to` is currently `String` and must be changed to `Option<String>` by a blocking task before these tests can compile.

- **`OutputSchema`** (`crates/agent-sdk/src/output_schema.rs`): `format: String`, `schema: HashMap<String, String>`. The `ALLOWED_OUTPUT_FORMATS` constant (`&["json", "structured_json", "text"]`) will be added by a blocking task.

- **`SkillError`** (specified in `.claude/specs/issue-5/define-skill-error-enum.md`): An enum in `crates/skill-loader/src/error.rs` with variant `ValidationError { skill_name: String, reasons: Vec<String> }`. The `reasons` vector collects all validation failures into a single error.

- **`ToolExists` trait** (specified in `.claude/tasks/issue-6.md` Group 2): Will be defined in `skill-loader` as `pub trait ToolExists { fn tool_exists(&self, name: &str) -> bool; }` with a provided stub `struct AllToolsExist;` that always returns `true`.

- **`validate` function** (specified in `.claude/tasks/issue-6.md` Group 3): `pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError>`. Collects all violations into `Vec<String>` and returns `Err(SkillError::ValidationError { skill_name, reasons })` if any are found.

- **Existing test patterns** in `crates/agent-sdk/tests/skill_manifest_test.rs`: Integration-style tests that construct `SkillManifest` values directly (field-by-field struct literals) and assert on field values. Uses `std::collections::HashMap` for output schema maps.

- **`skill-loader` crate** (`crates/skill-loader/`): Currently a skeleton with a placeholder `add()` function. `Cargo.toml` has edition 2024 and no dependencies. Dependencies (`agent-sdk`, `serde`, `serde_yaml`, etc.) will be added by a sibling task from issue #5.

## Requirements

1. **File location:** `crates/skill-loader/tests/validation_test.rs` (new integration test file).

2. **Test helper -- `valid_manifest()`:** A private helper function that returns a fully valid `SkillManifest` with all fields populated to pass every validation rule. This serves as a baseline that individual tests mutate one field at a time. The manifest must use:
   - Non-empty `name`, `version`, `description`, `preamble`
   - Valid `ModelConfig` with non-empty `provider` and `name`
   - `confidence_threshold` within `[0.0, 1.0]`
   - `max_turns >= 1`
   - `tools` containing only names that the default `AllToolsExist` checker accepts
   - `output.format` set to a recognized format (e.g., `"json"`)
   - `escalate_to` set to `None` (valid default after the type change)

3. **Test helper -- `RejectTools` struct:** A custom `ToolExists` implementation that rejects a configurable set of tool names. Used for test case #7 (unknown tool name). Should accept a `Vec<String>` or `HashSet<String>` of rejected tool names in its constructor.

4. **Assertion helper -- `expect_validation_error()`:** A helper that unwraps a `Result` into the `ValidationError` variant and returns the `reasons` vector, panicking with a clear message if the result is `Ok` or a different error variant.

5. **17 test cases**, each as a separate `#[test]` function:

   | # | Test name | Setup | Assertion |
   |---|-----------|-------|-----------|
   | 1 | `valid_manifest_passes` | `valid_manifest()` unchanged | `validate` returns `Ok(())` |
   | 2 | `confidence_threshold_above_one_fails` | Set `confidence_threshold = 1.5` | Returns `ValidationError` with reason mentioning confidence threshold |
   | 3 | `confidence_threshold_negative_fails` | Set `confidence_threshold = -0.1` | Returns `ValidationError` with reason mentioning confidence threshold |
   | 4 | `confidence_threshold_boundary_zero_passes` | Set `confidence_threshold = 0.0` | Returns `Ok(())` |
   | 4b | `confidence_threshold_boundary_one_passes` | Set `confidence_threshold = 1.0` | Returns `Ok(())` |
   | 5 | `max_turns_zero_fails` | Set `max_turns = 0` | Returns `ValidationError` with reason mentioning max_turns |
   | 6 | `max_turns_one_passes` | Set `max_turns = 1` | Returns `Ok(())` |
   | 7 | `unknown_tool_name_fails` | Add `"nonexistent_tool"` to tools; use `RejectTools` that rejects it | Returns `ValidationError` with reason mentioning the tool name |
   | 8 | `empty_name_fails` | Set `name = ""` | Returns `ValidationError` with reason mentioning name |
   | 9 | `empty_version_fails` | Set `version = ""` | Returns `ValidationError` with reason mentioning version |
   | 10 | `empty_preamble_fails` | Set `preamble = ""` | Returns `ValidationError` with reason mentioning preamble |
   | 11 | `empty_model_provider_fails` | Set `model.provider = ""` | Returns `ValidationError` with reason mentioning provider |
   | 12 | `empty_model_name_fails` | Set `model.name = ""` | Returns `ValidationError` with reason mentioning model name |
   | 13 | `unrecognized_output_format_fails` | Set `output.format = "raw"` | Returns `ValidationError` with reason mentioning output format |
   | 14 | `recognized_output_formats_pass` | Loop over `["json", "structured_json", "text"]`, set `output.format` to each | Each returns `Ok(())` |
   | 15 | `multiple_violations_collected` | Set `name = ""` AND `confidence_threshold = 2.0` | Returns `ValidationError` with `reasons.len() >= 2`, containing both violation messages |
   | 16 | `escalate_to_empty_string_fails` | Set `escalate_to = Some("")` | Returns `ValidationError` with reason mentioning escalate_to |
   | 17 | `escalate_to_none_passes` | Set `escalate_to = None` (baseline) | Returns `Ok(())` |

6. **Imports:** The test file imports from `skill_loader` (for `validate`, `ToolExists`, `AllToolsExist`, `SkillError`) and from `agent_sdk` (for `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`). It also uses `std::collections::HashMap`.

7. **No `#[tokio::test]`:** The `validate` function is synchronous, so all tests use plain `#[test]`.

8. **Assertion style:** Tests that expect failure should assert that the returned error is specifically `SkillError::ValidationError` (not `IoError` or `ParseError`). Tests should verify the `skill_name` field matches the manifest's name. Tests should check that at least one entry in `reasons` contains a substring relevant to the violated rule (e.g., `"confidence_threshold"` or `"threshold"`), rather than asserting exact error message text, to avoid brittle tests.

9. **No mutation of `description` or `allowed_actions`:** The task breakdown does not list these as validated fields, so no tests for empty `description` or empty `allowed_actions` are included.

## Implementation Details

### File to create: `crates/skill-loader/tests/validation_test.rs`

- **`valid_manifest()` helper:** Constructs a `SkillManifest` with:
  ```
  name: "test-skill"
  version: "1.0"
  description: "A test skill"
  model: { provider: "anthropic", name: "claude-3-haiku", temperature: 0.5 }
  preamble: "You are a test assistant."
  tools: vec!["web_search"]
  constraints: { max_turns: 5, confidence_threshold: 0.8, escalate_to: None, allowed_actions: vec!["search"] }
  output: { format: "json", schema: { "result": "string" } }
  ```

- **`RejectTools` struct:**
  ```rust
  struct RejectTools {
      rejected: Vec<String>,
  }
  impl ToolExists for RejectTools {
      fn tool_exists(&self, name: &str) -> bool {
          !self.rejected.iter().any(|r| r == name)
      }
  }
  ```

- **`expect_validation_error()` helper:**
  Takes a `Result<(), SkillError>`, asserts it is `Err(SkillError::ValidationError { .. })`, and returns the `reasons` vector. Panics with descriptive messages on `Ok(())` or wrong error variant.

- **Each test function:** Follows the pattern:
  1. Call `valid_manifest()` and bind to `let mut m = valid_manifest();`
  2. Mutate the field under test
  3. Call `validate(&m, &AllToolsExist)` (or `&RejectTools { .. }` for test #7)
  4. Assert on the result

- **Test #14 (recognized formats):** Uses a `for` loop inside a single test function, iterating over `["json", "structured_json", "text"]` and asserting each returns `Ok(())`.

- **Test #15 (multiple violations):** Mutates both `name` to `""` and `confidence_threshold` to `2.0`, then asserts `reasons.len() >= 2` and that the reasons contain substrings for both violations.

### No other files created or modified

This task only creates the test file. It does not modify `lib.rs`, `Cargo.toml`, or any source files.

## Dependencies

- **Blocked by:**
  - "Implement validate function" (Group 3, issue #6) -- the `validate` function, `ToolExists` trait, `AllToolsExist` struct, and `SkillError` must exist and be publicly exported from `skill-loader` before these tests can compile.
  - "Change `escalate_to` to `Option<String>`" (Group 1, issue #6) -- the `Constraints.escalate_to` field must be `Option<String>` for test cases #16 and #17 to compile, and for `valid_manifest()` to set `escalate_to: None`.
  - "Define allowed output format constants" (Group 1, issue #6) -- while the tests do not directly import `ALLOWED_OUTPUT_FORMATS`, the validate function depends on it, so it must exist.
  - "Define SkillError enum" (issue #5) -- `SkillError::ValidationError` must be defined.
- **Blocking:** Nothing. This is a leaf task.

## Risks & Edge Cases

1. **`reasons` ordering:** Test #15 asserts that both violation messages are present in the `reasons` vector but does not assert a specific order. This avoids coupling to the internal iteration order of the `validate` function. The test should use `reasons.iter().any(|r| r.contains(...))` for each expected substring.

2. **`escalate_to` type change not yet landed:** Until the blocking task changes `escalate_to` from `String` to `Option<String>`, the `valid_manifest()` helper and tests #16/#17 will not compile. The test file should be written against the target type (`Option<String>`) as specified in the task breakdown.

3. **`max_turns` is `u32`:** It cannot be negative, so only the zero boundary needs testing. The test for `max_turns = 0` is sufficient; there is no test for negative values because the type prevents them.

4. **Whitespace-only strings:** The task breakdown specifies that required string fields must "not be empty or whitespace-only." The test cases use `""` (empty string). Additional tests for `"   "` (whitespace-only) could be added as a follow-up but are not in the 17 enumerated cases and are therefore out of scope.

5. **Error message substring matching:** Tests should use `contains()` with short, stable substrings (e.g., `"confidence"`, `"max_turns"`, `"provider"`, `"format"`) rather than exact string equality. This protects against minor wording changes in error messages while still verifying the right rule was triggered.

6. **`skill_name` field in `ValidationError`:** When `name` is empty (test #8), the `ValidationError.skill_name` will be `""`. Tests should account for this -- they can either skip the `skill_name` assertion for that case or assert it equals `""`.

7. **Test isolation:** Each test builds its own manifest from `valid_manifest()` and does not share mutable state, so tests are fully parallelizable by the Rust test harness.

## Verification

1. Once all blocking tasks are complete, `cargo test -p skill-loader --test validation_test` compiles and all 17+ test functions pass.
2. `cargo clippy -p skill-loader --tests` reports no warnings on the test file.
3. Each "fails" test case returns `Err(SkillError::ValidationError { .. })` with at least one reason containing a relevant substring.
4. Each "passes" test case returns `Ok(())`.
5. Test #15 confirms that multiple violations are accumulated (reasons length >= 2) rather than short-circuiting on the first failure.
6. The `valid_manifest()` baseline passes validation unchanged (test #1), confirming the helper itself is correct.
