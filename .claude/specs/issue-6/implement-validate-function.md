# Spec: Implement `validate` function

> From: .claude/tasks/issue-6.md

## Objective

Create the core validation function for the `skill-loader` crate that checks a `SkillManifest` for semantic correctness at startup time. The function collects all violations into a single error rather than failing on the first one, giving operators a complete picture of what needs fixing. Each validation rule is implemented as a small, focused helper function (respecting the project's 50-line rule). This is the central piece of issue #6's "startup-time validation" goal.

## Current State

- **`crates/skill-loader/src/lib.rs`** is a placeholder with only an `add()` function and a trivial test. No real types, traits, or validation logic exist. The `Cargo.toml` has no dependencies.

- **`SkillManifest`** (in `crates/agent-sdk/src/skill_manifest.rs`) is a flat struct with fields: `name: String`, `version: String`, `description: String`, `model: ModelConfig`, `preamble: String`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`.

- **`ModelConfig`** (in `crates/agent-sdk/src/model_config.rs`) has fields: `provider: String`, `name: String`, `temperature: f64`.

- **`Constraints`** (in `crates/agent-sdk/src/constraints.rs`) has fields: `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: String`, `allowed_actions: Vec<String>`. Note: `escalate_to` is currently `String`, not `Option<String>`. A prerequisite task will change it to `Option<String>` before this task is implemented.

- **`OutputSchema`** (in `crates/agent-sdk/src/output_schema.rs`) has fields: `format: String`, `schema: HashMap<String, String>`. No `ALLOWED_OUTPUT_FORMATS` constant exists yet. A prerequisite task will add `pub const ALLOWED_OUTPUT_FORMATS: &[&str] = &["json", "structured_json", "text"];` to this file.

- **`SkillError`** does not exist yet. Per the issue-5 spec (`/.claude/specs/issue-5/define-skill-error-enum.md`), it will be defined in `crates/skill-loader/src/error.rs` with three variants: `IoError { path: PathBuf, source: String }`, `ParseError { path: PathBuf, source: String }`, and `ValidationError { skill_name: String, reasons: Vec<String> }`. The `ValidationError` variant is the one this function produces.

- **`ToolExists` trait** does not exist yet. A prerequisite task (Group 2 of issue-6) will define it in the `skill-loader` crate as: `pub trait ToolExists { fn tool_exists(&self, name: &str) -> bool; }`, along with a stub `struct AllToolsExist;` that always returns `true`.

- **Error pattern:** The project uses manual `Display + Error` impls (no `thiserror`), as seen in `crates/agent-sdk/src/agent_error.rs`.

- **`tool-registry` crate** is a placeholder with only an `add()` function. The `ToolExists` trait is intentionally decoupled from it via a trait in `skill-loader`.

## Requirements

1. **Function signature:** `pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError>`. The function is synchronous (no async needed -- it performs no I/O).

2. **Collect-all-violations pattern:** The function must accumulate all validation failures into a `Vec<String>` and return `Err(SkillError::ValidationError { skill_name: manifest.name.clone(), reasons })` only if the vector is non-empty. On success, return `Ok(())`.

3. **Check 1 -- Required string fields non-empty:** `manifest.name`, `manifest.version`, `manifest.model.provider`, and `manifest.model.name` must not be empty or whitespace-only. Each failing field produces a distinct reason string (e.g., `"'name' must not be empty"`). Use `.trim().is_empty()` to detect whitespace-only strings.

4. **Check 2 -- Preamble non-empty:** `manifest.preamble` must not be blank (empty or whitespace-only). Reason: `"'preamble' must not be empty"`.

5. **Check 3 -- `confidence_threshold` in [0.0, 1.0]:** The value must satisfy `0.0 <= value <= 1.0` (inclusive bounds). Reason: `"'confidence_threshold' must be between 0.0 and 1.0, got {value}"`.

6. **Check 4 -- `max_turns` > 0:** The value must be greater than zero. Since `max_turns` is `u32`, it cannot be negative, but it can be zero. Reason: `"'max_turns' must be greater than 0"`.

7. **Check 5 -- Tool existence:** For each name in `manifest.tools`, call `tool_checker.tool_exists(name)`. Collect all names that return `false`. If any are missing, produce a single reason: `"tools not found: {comma-separated list}"`. The list should preserve the order tools appear in the manifest.

8. **Check 6 -- Output format recognized:** `manifest.output.format` must be one of the values in `ALLOWED_OUTPUT_FORMATS` (imported from `agent_sdk::output_schema` or `agent_sdk`). Reason: `"unrecognized output format '{format}'"`.

9. **Check 7 -- `escalate_to` non-empty if present:** If `manifest.constraints.escalate_to` is `Some(name)`, validate that `name` is not empty or whitespace-only. Reason: `"'escalate_to' must not be empty when provided"`. Include a `// TODO: full cross-agent escalation validation deferred to orchestrator` comment near this check.

10. **Helper functions:** Each check (or logical group of checks) must be a separate private helper function. No single function may exceed 50 lines.

11. **Deterministic ordering:** Violations must be added to the `Vec<String>` in the order the checks are listed above (checks 1-7). Within check 1, the order is: `name`, `version`, `model.provider`, `model.name`. This ensures tests can assert on reason ordering.

12. **No new external dependencies:** The validation logic uses only `std` types and types from `agent-sdk`. The `skill-loader` crate will need `agent-sdk` as a dependency (added by a sibling task), but this task adds no additional crate dependencies.

## Implementation Details

### File: `crates/skill-loader/src/validation.rs` (new)

Create a new module dedicated to validation logic. This keeps `lib.rs` focused on module wiring and the `SkillLoader` struct.

**Imports:**
- `use agent_sdk::{SkillManifest, output_schema::ALLOWED_OUTPUT_FORMATS};` (or import `ALLOWED_OUTPUT_FORMATS` from wherever it is re-exported).
- `use crate::error::SkillError;`
- `use crate::ToolExists;` (the trait defined by the prerequisite task).

**Public function:**
```
pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError>
```
- Creates a `let mut reasons: Vec<String> = Vec::new();`.
- Calls each helper in order, passing `&mut reasons` (and relevant manifest fields).
- After all helpers, checks `if reasons.is_empty() { Ok(()) } else { Err(SkillError::ValidationError { ... }) }`.

**Private helper functions (all take `reasons: &mut Vec<String>` plus the data they need):**

1. `fn check_required_strings(manifest: &SkillManifest, reasons: &mut Vec<String>)` -- Checks `name`, `version`, `model.provider`, `model.name`. Uses a local array of `(&str, &str)` tuples (field label, field value) and iterates, pushing a reason for each blank field.

2. `fn check_preamble(preamble: &str, reasons: &mut Vec<String>)` -- Single check on `preamble.trim().is_empty()`.

3. `fn check_confidence_threshold(value: f64, reasons: &mut Vec<String>)` -- Checks `!(0.0..=1.0).contains(&value)`.

4. `fn check_max_turns(value: u32, reasons: &mut Vec<String>)` -- Checks `value == 0`.

5. `fn check_tools_exist(tools: &[String], tool_checker: &dyn ToolExists, reasons: &mut Vec<String>)` -- Iterates tools, collects missing names, formats a single reason if any.

6. `fn check_output_format(format: &str, reasons: &mut Vec<String>)` -- Checks `!ALLOWED_OUTPUT_FORMATS.contains(&format)`.

7. `fn check_escalate_to(escalate_to: &Option<String>, reasons: &mut Vec<String>)` -- Pattern matches on `Some(name)` and checks `name.trim().is_empty()`. Includes the TODO comment about cross-agent validation.

### File: `crates/skill-loader/src/lib.rs` (modified)

Add the module declaration and re-export. The exact content depends on what other tasks have already landed, but the validation-specific additions are:

```rust
mod validation;
pub use validation::validate;
```

Note: The `ToolExists` trait is defined in `lib.rs` (or a dedicated module) by the prerequisite Group 2 task. The `validate` function imports it via `crate::ToolExists`.

### Integration points

- **`SkillLoader::load()`** (defined by issue-5, Group 3 task) will call `validate(&manifest, &*self.tool_checker)?` after constructing the manifest and before returning `Ok(manifest)`. This integration is handled by the separate "Integrate validation into `SkillLoader::load()`" task (issue-6, Group 4).
- **`SkillError::ValidationError`** is the only error variant this function produces. It is defined in `crates/skill-loader/src/error.rs` by the issue-5 "Define SkillError enum" task.
- **`ALLOWED_OUTPUT_FORMATS`** is defined in `crates/agent-sdk/src/output_schema.rs` by the issue-6 Group 1 task "Define allowed output format constants."
- **`ToolExists` trait** is defined in the `skill-loader` crate by the issue-6 Group 2 task.

## Dependencies

- **Blocked by:**
  - "Change `escalate_to` from `String` to `Option<String>` in `Constraints`" (issue-6, Group 1) -- check 7 pattern-matches on `Option<String>`.
  - "Define allowed output format constants" (issue-6, Group 1) -- check 6 references `ALLOWED_OUTPUT_FORMATS`.
  - "Define `ToolExists` trait" (issue-6, Group 2) -- the `validate` function signature depends on it.
  - "Define `SkillError` enum" (issue-5, Group 1) -- the function returns `SkillError::ValidationError`.
  - `agent-sdk` dependency in `skill-loader/Cargo.toml` (issue-5, Group 1) -- needed to import `SkillManifest` and `ALLOWED_OUTPUT_FORMATS`.

- **Blocking:**
  - "Integrate validation into `SkillLoader::load()`" (issue-6, Group 4) -- calls `validate` from within `load`.
  - "Write validation unit tests" (issue-6, Group 5) -- tests exercise `validate` directly.

## Risks & Edge Cases

1. **`ALLOWED_OUTPUT_FORMATS` import path:** The constant is defined in `crates/agent-sdk/src/output_schema.rs`, but `agent-sdk`'s `lib.rs` currently only re-exports `OutputSchema`, not the constant. The implementation must either: (a) add a `pub use output_schema::ALLOWED_OUTPUT_FORMATS;` to `agent-sdk/src/lib.rs`, or (b) access it via `agent_sdk::output_schema::ALLOWED_OUTPUT_FORMATS` if the `output_schema` module is made `pub`. The prerequisite task spec should clarify the re-export strategy. If it does not, the implementer should add the re-export in `agent-sdk/src/lib.rs`.

2. **`NaN` for `confidence_threshold`:** If `confidence_threshold` is `NaN` (possible with `f64`), the range check `(0.0..=1.0).contains(&value)` returns `false` because `NaN` comparisons are always false. This is correct behavior -- `NaN` should be rejected. No special handling needed.

3. **Empty `manifest.name` in error:** If `manifest.name` is empty or whitespace-only, the `SkillError::ValidationError { skill_name, .. }` will contain an empty/whitespace `skill_name`. This is acceptable because the error also carries the `reasons` vector which explicitly says `"'name' must not be empty"`. The `Display` impl for `SkillError` will show `"validation error for skill '': ..."` which is ugly but diagnosable.

4. **Deterministic reason ordering:** The spec mandates a fixed check order. This is important because the test task (Group 5) will assert on the exact contents and order of the `reasons` vector. If a future contributor reorders the checks, tests will break. This is intentional -- it enforces a stable contract.

5. **Tool order in "tools not found" message:** Missing tools are listed in the order they appear in `manifest.tools`. If a tool appears multiple times in the list and is missing, it will appear multiple times in the error message. This is acceptable; deduplication is not required.

6. **`escalate_to` field type transition:** Until the prerequisite "Change `escalate_to` to `Option<String>`" task lands, this code will not compile. The implementer must not work around this by accepting `String` -- the `Option<String>` type is a hard requirement.

7. **Thread safety:** The `validate` function takes shared references only (`&SkillManifest`, `&dyn ToolExists`) and has no interior mutability. It is safe to call from multiple threads concurrently.

8. **Future extensibility:** New validation checks can be added by writing a new helper function and calling it from `validate`. The `reasons` vector pattern is open for extension without modifying existing helpers.

## Verification

1. **Compilation:** Run `cargo check -p skill-loader` -- must pass with no errors (requires all blocked-by tasks to be complete first).
2. **Lint:** Run `cargo clippy -p skill-loader` -- must pass with no warnings.
3. **Tests:** Run `cargo test -p skill-loader` -- must pass. Note: the validation unit tests are a separate task (Group 5); this task verifies only that the code compiles and does not break existing tests.
4. **Code structure audit:**
   - `crates/skill-loader/src/validation.rs` exists and contains exactly one `pub fn validate(...)` and seven private helper functions.
   - No function exceeds 50 lines.
   - No commented-out code or debug statements.
   - A `// TODO: full cross-agent escalation validation deferred to orchestrator` comment exists near the `check_escalate_to` helper.
5. **Module wiring:** `crates/skill-loader/src/lib.rs` contains `mod validation;` and `pub use validation::validate;`, making the function accessible as `skill_loader::validate` from external crates.
6. **No new dependencies:** `crates/skill-loader/Cargo.toml` has no dependencies added by this task beyond what the prerequisite tasks already added.
