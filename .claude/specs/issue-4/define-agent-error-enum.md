# Spec: Define `AgentError` enum

> From: .claude/tasks/issue-4.md

## Objective

Create an `AgentError` enum in `crates/agent-sdk/src/agent_error.rs` that represents the set of typed failure modes an agent can encounter during execution. This enum covers tool invocation failures, insufficient confidence, turn-limit exhaustion, and catch-all internal errors. Unlike the other SDK types, `AgentError` implements `std::fmt::Display` and `std::error::Error` so it can participate in Rust's standard error-handling ecosystem (e.g., `Result<T, AgentError>`, `?` operator, `anyhow` interop).

## Current State

- `crates/agent-sdk/src/lib.rs` currently declares four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) with corresponding `pub use` re-exports. No error types exist yet.
- The existing `Constraints` struct (in `constraints.rs`) defines `max_turns: u32` and `confidence_threshold: f64`. The `AgentError` enum references these same concepts: `MaxTurnsExceeded` carries `turns: u32` to mirror `max_turns`, and `ConfidenceTooLow` carries `confidence: f32` and `threshold: f32`.
- Note the intentional type difference: `Constraints` uses `f64` for `confidence_threshold` (matching YAML precision), while `AgentError::ConfidenceTooLow` uses `f32` for both fields. This is specified in the task description and is acceptable because the error variant captures a runtime snapshot for display purposes, not a configuration value.
- All existing structs derive `JsonSchema`. `AgentError` intentionally does **not** derive `JsonSchema` since error types are not part of schema generation.
- `Cargo.toml` already has `serde = { version = "1", features = ["derive"] }` and `schemars` -- only `serde` is needed for this task.

## Requirements

1. **File location:** `crates/agent-sdk/src/agent_error.rs`.

2. **Enum definition:** A public enum named `AgentError` with the following variants:
   - `ToolCallFailed { tool: String, reason: String }` -- A named tool invocation failed. `tool` is the tool name, `reason` is a human-readable explanation.
   - `ConfidenceTooLow { confidence: f32, threshold: f32 }` -- The agent's confidence score fell below the required threshold. `confidence` is the actual score, `threshold` is the minimum required.
   - `MaxTurnsExceeded { turns: u32 }` -- The agent exhausted its turn budget. `turns` is the number of turns consumed.
   - `Internal(String)` -- A catch-all for unexpected internal errors. The `String` carries a human-readable description.

3. **Derive macros:** `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`. Do **not** derive `JsonSchema`.

4. **`Display` implementation:** Implement `std::fmt::Display` for `AgentError` with the following format strings:
   - `ToolCallFailed` -> `"Tool call '{}' failed: {}"` (tool, reason)
   - `ConfidenceTooLow` -> `"Confidence {:.2} is below threshold {:.2}"` (confidence, threshold)
   - `MaxTurnsExceeded` -> `"Max turns exceeded: {} turns used"` (turns)
   - `Internal` -> `"Internal error: {}"` (message)

5. **`Error` implementation:** Implement `std::error::Error` for `AgentError`. Since the `Error` trait only requires `Display + Debug`, and both are already provided (via derive and manual impl), the impl block can be empty: `impl std::error::Error for AgentError {}`.

6. **Imports:** The file must bring `serde::{Serialize, Deserialize}` and `std::fmt` into scope.

7. **No `JsonSchema` derive.** Error types are not part of the public API schema surface.

8. **No additional methods** beyond the required trait implementations. No constructors, no builder, no `From` impls. This is a data-carrying error enum.

## Implementation Details

### File: `crates/agent-sdk/src/agent_error.rs` (new)

The file should contain:

- `use serde::{Serialize, Deserialize};` and `use std::fmt;`
- A `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` attribute on the enum.
- The four variants as described above.
- An `impl fmt::Display for AgentError` block with a single `fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result` method that matches on `self` and writes the appropriate message.
- An `impl std::error::Error for AgentError {}` block (empty body -- the blanket implementation from `Display + Debug` is sufficient).

This file will **not** be wired into `lib.rs` by this task. A separate downstream task ("Update `lib.rs` module declarations and re-exports") will add `mod agent_error;` and `pub use agent_error::AgentError;`.

### Serde representation

With the default serde behavior for enums, the serialized JSON will use an externally tagged representation:
- `ToolCallFailed` -> `{"ToolCallFailed": {"tool": "...", "reason": "..."}}`
- `ConfidenceTooLow` -> `{"ConfidenceTooLow": {"confidence": 0.5, "threshold": 0.75}}`
- `MaxTurnsExceeded` -> `{"MaxTurnsExceeded": {"turns": 5}}`
- `Internal` -> `{"Internal": "some message"}`

This default representation is acceptable. No `#[serde(tag = ...)]` or `#[serde(rename_all = ...)]` attributes are needed.

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1). Although `AgentError` itself does not use `uuid` or `serde_json`, the task ordering in the issue requires Group 1 to complete before Group 2. In practice, `AgentError` only needs `serde` which is already present, so it can compile independently.
- **Parallel with:** "Define `ToolCallRecord` struct", "Define `HealthStatus` enum" (all in Group 2).
- **Blocking:** "Update `lib.rs` module declarations and re-exports" (Group 4), which will add the `mod` declaration and `pub use` re-export for this type.

## Risks & Edge Cases

1. **`f32` vs `f64` for confidence fields:** The `Constraints` struct uses `f64` for `confidence_threshold`, but `AgentError::ConfidenceTooLow` uses `f32` per the task specification. This means code that constructs this variant from a `Constraints` value will need an explicit `as f32` cast. This is intentional per the task description and is not a bug, but implementers of higher-level code should be aware of the lossy conversion.

2. **`PartialEq` with `f32`:** The `PartialEq` derive on an enum containing `f32` fields means that `NaN` comparisons will behave unexpectedly (`NaN != NaN`). In practice, confidence scores should always be in the `[0.0, 1.0]` range, so `NaN` values should not arise. No mitigation is needed.

3. **Serde backward compatibility:** If a future iteration needs to add new variants, the externally tagged representation will handle it gracefully for serialization. However, deserialization of an unknown variant will fail. If forward compatibility is needed later, a `#[serde(other)]` catch-all variant could be added, but that is out of scope for this task.

4. **`Display` format stability:** Downstream code (tests, logging, user-facing messages) may depend on the exact format strings. Changing them later is a non-breaking API change but could break snapshot tests. The specified formats should be treated as stable.

5. **Empty `tool` or `reason` strings:** The enum accepts empty strings without complaint. Validation of field contents is not the responsibility of the error type.

## Verification

1. After creating the file, run `cargo check -p agent-sdk` to confirm the enum compiles (requires `serde` dependency, which is already present).
2. Run `cargo clippy -p agent-sdk` to confirm no lint warnings.
3. Run `cargo test -p agent-sdk` to confirm existing tests still pass.
4. Full serialization and `Display` output testing is handled by the Group 5 task ("Write serialization and construction tests"), not this task. However, the implementer should informally verify that `format!("{}", error_variant)` produces the expected strings.
