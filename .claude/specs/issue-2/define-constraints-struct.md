# Spec: Define `Constraints` struct

> From: .claude/tasks/issue-2.md

## Objective

Create a `Constraints` struct in `crates/agent-sdk/src/constraints.rs` that represents the `constraints` section of a YAML skill file. This struct captures the guardrails that the runtime enforces on an agent: how many conversation turns it may take, the minimum confidence required before returning a result, where to escalate when confidence is insufficient, and which actions the agent is permitted to perform.

## Current State

- `crates/agent-sdk/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No domain types exist yet.
- `crates/agent-sdk/Cargo.toml` has no dependencies listed. The `serde`, `schemars` dependencies required for derive macros have not been added yet (that is a prerequisite task).
- No other `.rs` files exist under `crates/agent-sdk/src/`.
- The canonical YAML skill file example in `README.md` (lines 38-44) defines the constraints section:
  ```yaml
  constraints:
    max_turns: 5
    confidence_threshold: 0.75
    escalate_to: general-finance-agent
    allowed_actions:
      - read
      - query
  ```

## Requirements

1. **File location:** `crates/agent-sdk/src/constraints.rs`.

2. **Struct definition:** A public struct named `Constraints` with the following fields, all public:
   - `max_turns: u32` -- Maximum number of LLM conversation turns the agent may execute before it must return or escalate.
   - `confidence_threshold: f64` -- Minimum confidence score (0.0 to 1.0) the agent must reach to return a result directly rather than escalating.
   - `escalate_to: String` -- The name of the agent to hand off to when confidence is below threshold or max turns are exhausted.
   - `allowed_actions: Vec<String>` -- Whitelist of action identifiers (e.g., `"read"`, `"query"`) that this agent is permitted to perform.

3. **Derive macros:** `Debug`, `Clone`, `Serialize`, `Deserialize`, `JsonSchema`.

4. **Serde field naming:** Field names must serialize/deserialize to match the YAML keys exactly. Since Rust uses `snake_case` and the YAML keys also use `snake_case` (`max_turns`, `confidence_threshold`, `escalate_to`, `allowed_actions`), no `#[serde(rename)]` attributes are needed -- the default behavior is correct.

5. **Imports:** The file must bring `serde::{Serialize, Deserialize}` and `schemars::JsonSchema` into scope.

6. **No additional methods or trait implementations** beyond the derives. This is a plain data struct.

## Implementation Details

The file should contain:

- `use serde::{Serialize, Deserialize};`
- `use schemars::JsonSchema;`
- A `#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]` attribute on the struct.
- The struct itself with four public fields as specified above.

No `impl` block is required. No builder pattern, no `Default` implementation, no validation logic. The struct is a pure data carrier whose correctness is enforced by the `skill-loader` crate at parse time, not by the SDK.

The file will **not** be wired into `lib.rs` by this task. That is handled by a separate downstream task ("Update `lib.rs` module declarations and re-exports") which will add `pub mod constraints;` and `pub use constraints::Constraints;`.

## Dependencies

- **Blocked by:** "Add `serde`, `schemars` dependencies to `agent-sdk/Cargo.toml`" (Group 1). The derive macros `Serialize`, `Deserialize`, and `JsonSchema` will not compile without those crate dependencies present.
- **Parallel with:** "Define `ModelConfig` struct", "Define `OutputSchema` struct" (all in Group 2).
- **Blocks:** "Define `SkillManifest` struct" (Group 3), which composes `Constraints` as a field.

## Risks & Edge Cases

1. **Dependency ordering:** If this task is attempted before `serde` and `schemars` are added to `Cargo.toml`, the file will not compile. The implementer must verify the dependencies exist first or coordinate with the Group 1 task.

2. **Field name mismatch with YAML:** The YAML key `confidence_threshold` uses an underscore, matching Rust's default `snake_case` serde behavior. No risk here, but any future YAML key using hyphens (e.g., `max-turns`) would require `#[serde(rename = "max-turns")]`. Currently all keys are underscore-separated, so no rename is needed.

3. **Type precision for `confidence_threshold`:** Using `f64` matches the task description. The canonical example uses `0.75`, which is representable in both `f32` and `f64`. `f64` is the safer default and aligns with standard serde behavior for YAML floats.

4. **`escalate_to` as empty string:** If a skill file omits `escalate_to` or provides an empty string, this struct will accept it without complaint. Validation is not the SDK's responsibility -- it belongs in `skill-loader`. However, a future iteration may want to make this field `Option<String>` for skills that have no escalation target. That change is out of scope for this task.

5. **`allowed_actions` as empty vec:** An empty `allowed_actions` list is valid at the struct level. Whether it is semantically valid (an agent that can do nothing) is a validation concern for `skill-loader`.

## Verification

1. After creating the file, run `cargo check -p agent-sdk` to confirm the struct compiles (requires Group 1 dependencies to be in place).
2. Run `cargo clippy -p agent-sdk` to confirm no lint warnings.
3. Run `cargo test -p agent-sdk` to confirm existing tests (if any) still pass.
4. Full round-trip deserialization testing is handled by the Group 4 task ("Write deserialization tests"), not this task.
