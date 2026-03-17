# Spec: Write integration test for load-with-validation

> From: .claude/tasks/issue-6.md

## Objective

Create an integration test that verifies `SkillLoader::load()` rejects skill files with invalid frontmatter at load time by returning `SkillError::ValidationError`. This test exercises the end-to-end path: write a `.md` skill file with semantically invalid data to a temp directory, call `load()`, and confirm the error variant and violation message. This is the integration-level acceptance gate for the "validate on load" behavior introduced by issue #6 -- it proves that validation is wired into the loader, not just available as a standalone function.

## Current State

- **`crates/skill-loader/src/lib.rs`** currently contains only a placeholder `add()` function. Once issue #5 lands, it will define `SkillLoader` with `new()` and `pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>`. The loader reads a `.md` file, extracts YAML frontmatter, deserializes it into `SkillFrontmatter`, and constructs a `SkillManifest`.
- **`SkillError`** (to be defined in `crates/skill-loader/src/error.rs` by issue #5) has three variants: `IoError { path: PathBuf, source: String }`, `ParseError { path: PathBuf, source: String }`, and `ValidationError { skill_name: String, reasons: Vec<String> }`. It follows the manual `Display + Error` impl pattern from `crates/agent-sdk/src/agent_error.rs` (no `thiserror`).
- **Validation integration** (the "Integrate validation into `SkillLoader::load()`" task in issue-6 Group 4) will add a `validate(&manifest, &self.tool_checker)?` call inside `load()` after successfully constructing the `SkillManifest`. The `SkillLoader` struct will gain a `tool_checker` field of type `Arc<dyn ToolExists>` (or `Box<dyn ToolExists>`).
- **The `validate` function** (issue-6 Group 3) checks: required string fields non-empty, preamble non-empty, `confidence_threshold` in `[0.0, 1.0]`, `max_turns > 0`, tool existence via `ToolExists` trait, output format against `ALLOWED_OUTPUT_FORMATS`, and `escalate_to` non-empty if `Some`.
- **`ToolExists` trait** (issue-6 Group 2) will be defined in the skill-loader crate with a method `fn tool_exists(&self, name: &str) -> bool`. A stub `AllToolsExist` struct that always returns `true` is provided for tests that do not exercise tool-name validation.
- **`Constraints.escalate_to`** will be `Option<String>` (issue-6 Group 1 task, already specced).
- **`ALLOWED_OUTPUT_FORMATS`** will be `&["json", "structured_json", "text"]` in `agent_sdk::ALLOWED_OUTPUT_FORMATS` (issue-6 Group 1 task, already specced).
- **Existing test patterns**: `crates/agent-sdk/tests/skill_manifest_test.rs` uses `const` YAML strings, individual field assertions, and `serde_yaml::from_str`. `crates/skill-loader/tests/skill_loader_test.rs` (to be created by issue #5) will use `tempfile::tempdir()`, `tokio::fs::write`, `#[tokio::test]`, and `matches!`/`match` for error variant assertions.
- **`SkillManifest`** fields: `name`, `version`, `description`, `model` (`ModelConfig`), `preamble`, `tools` (`Vec<String>`), `constraints` (`Constraints`), `output` (`OutputSchema`). All structs derive `PartialEq`.
- **`ModelConfig`** fields: `provider` (String), `name` (String), `temperature` (f64).
- **`SkillLoader` constructor** will accept the skill directory path and a tool checker. After issue-6 integration, the signature will be approximately: `SkillLoader::new(skill_dir: PathBuf, tool_checker: Arc<dyn ToolExists>)` (the `Arc<ToolRegistry>` from issue-5 gets replaced or supplemented by the `ToolExists` trait object).

## Requirements

### 1. Test: `confidence_threshold` out of range triggers `ValidationError`

Create an async test named `load_invalid_confidence_threshold_returns_validation_error` that:
- Creates a `tempfile::tempdir()`.
- Writes a file named `bad-threshold.md` into the temp directory with valid `---` delimiters and complete YAML frontmatter where all fields are valid except `confidence_threshold: 2.0` (above the allowed `[0.0, 1.0]` range).
- The markdown body (preamble) must be non-empty to avoid triggering a separate "preamble is blank" validation violation.
- Constructs a `SkillLoader` with `skill_dir` pointing to the temp directory and a `tool_checker` that accepts all tools (the `AllToolsExist` stub).
- Calls `loader.load("bad-threshold").await`.
- Asserts the result is `Err`.
- Pattern-matches the error to confirm it is `SkillError::ValidationError`.
- Asserts that `skill_name` equals `"bad-threshold-skill"` (or whatever `name` is set in the frontmatter).
- Asserts that `reasons` contains exactly one entry and that it includes a substring about `confidence_threshold` being out of range (e.g., contains `"confidence_threshold"`).

### 2. Test: `confidence_threshold` below range triggers `ValidationError`

Create an async test named `load_negative_confidence_threshold_returns_validation_error` that:
- Writes a skill file with `confidence_threshold: -0.5`.
- Asserts `SkillError::ValidationError` with a reason mentioning `confidence_threshold`.

### 3. Test: multiple validation violations collected in one error

Create an async test named `load_multiple_violations_returns_all_reasons` that:
- Writes a skill file with multiple invalid fields simultaneously: `confidence_threshold: 2.0`, `max_turns: 0`, and an empty `name` field.
- Calls `loader.load(...)`.
- Asserts the result is `SkillError::ValidationError`.
- Asserts that `reasons.len() >= 3` (at least one reason per violation).
- Asserts that the reasons collectively mention `confidence_threshold`, `max_turns`, and `name`.

### 4. Test: valid skill file passes validation and returns `Ok`

Create an async test named `load_valid_skill_passes_validation` that:
- Writes a fully valid skill file with `confidence_threshold: 0.85`, `max_turns: 5`, non-empty `name`, valid output format (`"json"`), non-empty preamble, etc.
- Calls `loader.load(...)`.
- Asserts the result is `Ok`.
- Asserts the returned `SkillManifest` fields match the frontmatter values.

### 5. Test: unrecognized output format triggers `ValidationError`

Create an async test named `load_invalid_output_format_returns_validation_error` that:
- Writes a skill file where all fields are valid except `output.format` is `"raw"` (not in `ALLOWED_OUTPUT_FORMATS`).
- Asserts `SkillError::ValidationError` with a reason mentioning the output format.

### 6. Test: `max_turns` of zero triggers `ValidationError`

Create an async test named `load_zero_max_turns_returns_validation_error` that:
- Writes a skill file where all fields are valid except `max_turns: 0`.
- Asserts `SkillError::ValidationError` with a reason mentioning `max_turns`.

### 7. Test: empty preamble triggers `ValidationError`

Create an async test named `load_empty_preamble_returns_validation_error` that:
- Writes a skill file with valid frontmatter but an empty markdown body (nothing after the closing `---`, or only whitespace).
- Asserts `SkillError::ValidationError` with a reason mentioning `preamble`.

## Implementation Details

### File to create

```
crates/skill-loader/tests/validation_test.rs
```

This is a Rust integration test file (lives in `tests/`, not `src/`). It imports from the crate's public API. It is separate from `skill_loader_test.rs` (created by issue #5) to keep validation-specific tests organized.

### Imports

```rust
use std::sync::Arc;

use skill_loader::{SkillLoader, SkillError, AllToolsExist};
use tempfile::tempdir;
use tokio::fs;
```

The `AllToolsExist` stub must be exported from the skill-loader crate's public API for use in tests. If it is not exported, the test file will need to define its own `ToolExists` implementation that returns `true` for all tools:

```rust
use skill_loader::ToolExists;

struct StubAllToolsExist;
impl ToolExists for StubAllToolsExist {
    fn tool_exists(&self, _name: &str) -> bool {
        true
    }
}
```

### Helper function

Define a helper to reduce boilerplate:

```rust
fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let tool_checker = Arc::new(AllToolsExist);
    SkillLoader::new(dir.to_path_buf(), tool_checker)
}
```

### Canonical valid frontmatter YAML for baseline

Define a helper function or `const` that builds a valid frontmatter string. Tests that exercise a single invalid field should start from this baseline and modify only the field under test. This avoids accidentally triggering unrelated validation errors.

```rust
fn valid_frontmatter() -> String {
    r#"---
name: test-skill
version: "1.0"
description: A test skill
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
tools:
  - web_search
constraints:
  max_turns: 5
  confidence_threshold: 0.85
  allowed_actions:
    - search
output:
  format: json
  schema:
    result: string
---
You are a helpful test assistant."#.to_string()
}
```

For tests that need a specific invalid field, either:
- Build the YAML string directly with the invalid value inlined, OR
- Use `str::replace()` on the baseline to swap out the single field value.

The `str::replace()` approach is preferred because it makes each test's intent explicit (what changed from the valid baseline) while reducing duplication.

### Error matching pattern

Since `SkillError` does not derive `PartialEq`, use `match` or `matches!` for variant assertions:

```rust
let err = result.unwrap_err();
match err {
    SkillError::ValidationError { skill_name, reasons } => {
        assert_eq!(skill_name, "test-skill");
        assert_eq!(reasons.len(), 1);
        assert!(
            reasons[0].contains("confidence_threshold"),
            "expected reason about confidence_threshold, got: {}",
            reasons[0]
        );
    }
    other => panic!("expected ValidationError, got: {:?}", other),
}
```

Include the actual error in `panic!` messages to aid debugging when tests fail.

### Writing fixture files

Use `std::fs::write` (sync is acceptable in tests) or `tokio::fs::write` (async) to create `.md` files inside the `tempdir`. Each test creates its own `tempdir()` for isolation.

### Async test pattern

All tests use `#[tokio::test]` since `SkillLoader::load()` is async.

### `SkillLoader` constructor after issue-6 integration

After the "Integrate validation into `SkillLoader::load()`" task, the `SkillLoader::new()` signature will accept a tool checker implementing `ToolExists`. The exact parameter type may be `Arc<dyn ToolExists>`, `Box<dyn ToolExists>`, or a generic `impl ToolExists`. The tests should construct the loader with `AllToolsExist` (or a local stub) so that tool-name validation passes and does not interfere with the field-level validation being tested. If a specific test needs to exercise tool-name validation failure, it can provide a custom `ToolExists` implementation that rejects certain names.

### Key considerations for the `escalate_to` field

After the "Change `escalate_to` to `Option<String>`" task, the `Constraints` struct uses `Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`. In YAML frontmatter:
- Omitting `escalate_to` entirely deserializes to `None` (valid).
- Including `escalate_to: some_agent` deserializes to `Some("some_agent".to_string())`.
- The baseline valid frontmatter should omit `escalate_to` (setting it to `None`) to avoid accidental validation failures.

## Dependencies

- **Blocked by:**
  - "Integrate validation into `SkillLoader::load()`" (issue-6 Group 4) -- the `load()` method must call `validate()` before returning. Without this, `load()` will return `Ok` for invalid manifests.
  - Transitively blocked by all issue-5 tasks (the `SkillLoader` struct, `SkillError` enum, frontmatter extraction, and dependencies must all exist).
  - Transitively blocked by issue-6 Groups 1-3 (the `ToolExists` trait, `validate` function, `Option<String>` change to `escalate_to`, and `ALLOWED_OUTPUT_FORMATS` constant).
- **Blocking:** None. This is a leaf task in the dependency graph.

## Risks & Edge Cases

1. **`SkillLoader::new()` signature uncertainty.** After issue-6 integration, `SkillLoader::new()` will accept a tool checker parameter, but the exact type (`Arc<dyn ToolExists>`, `Box<dyn ToolExists>`, or generic) is not yet finalized. The test helper `make_loader` may need adjustment once the implementation lands. Mitigation: keep the helper simple and centralized so it is easy to update.

2. **`AllToolsExist` export availability.** The `AllToolsExist` struct may or may not be re-exported from the skill-loader crate's public API. If it is only `pub(crate)`, integration tests cannot import it. Mitigation: define a local stub implementing `ToolExists` as a fallback.

3. **Validation reason message format.** The exact wording of validation violation messages is determined by the `validate` function (a separate task). This spec asserts on substrings (e.g., `contains("confidence_threshold")`) rather than exact strings to avoid brittle coupling to message wording. Tests should assert on the field name being mentioned, not the full sentence.

4. **Multiple violations ordering.** When testing multiple violations in a single error, the order of `reasons` entries is determined by the `validate` function's check ordering. Tests should not depend on a specific ordering; instead, assert that the `reasons` vector contains at least one entry mentioning each expected field, using `reasons.iter().any(|r| r.contains("field_name"))`.

5. **Interaction with `ParseError` vs `ValidationError`.** A file with syntactically valid YAML but semantically invalid values (like `confidence_threshold: 2.0`) should produce `ValidationError`, not `ParseError`. `ParseError` is for malformed YAML or missing delimiters. The test must ensure the YAML is syntactically correct so the error path reaches validation.

6. **Tempdir lifetime.** The `TempDir` value returned by `tempdir()` must be held in a variable for the duration of each test. If only `tempdir().path()` is captured, the `TempDir` drops immediately and the directory is deleted before `load()` reads the file.

7. **Empty preamble edge case.** The validator checks that `preamble` is non-blank. A skill file with nothing after the closing `---` will have an empty preamble after `body.trim().to_string()`. The test for this case must ensure only the preamble violation fires -- all other frontmatter fields must be valid.

8. **`confidence_threshold` boundary values.** The `validate` function accepts `[0.0, 1.0]` inclusive. Tests use `2.0` and `-0.5` which are clearly out of range. Boundary tests for `0.0` and `1.0` are covered by the validation unit tests (issue-6 Group 5), not this integration test. This integration test focuses on proving the validation is wired into the loader, not exhaustively testing every boundary.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p skill-loader    # Ensure the test file compiles
cargo clippy -p skill-loader   # No warnings
cargo test -p skill-loader     # All tests pass
```

Specifically, confirm these tests exist and pass:

- `validation_test::load_invalid_confidence_threshold_returns_validation_error`
- `validation_test::load_negative_confidence_threshold_returns_validation_error`
- `validation_test::load_multiple_violations_returns_all_reasons`
- `validation_test::load_valid_skill_passes_validation`
- `validation_test::load_invalid_output_format_returns_validation_error`
- `validation_test::load_zero_max_turns_returns_validation_error`
- `validation_test::load_empty_preamble_returns_validation_error`

Additionally, verify that existing tests across the workspace continue to pass:

```bash
cargo test    # Full workspace test suite
```
