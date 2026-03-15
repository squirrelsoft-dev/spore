# Spec: Define `AgentError` enum

> From: .claude/tasks/issue-3.md

## Objective

Create an `AgentError` enum in `crates/agent-sdk/src/agent_error.rs` that represents the possible failure modes when an agent processes a request. This type will serve as the `Err` variant in `Result<AgentResponse, AgentError>` returned by the `MicroAgent::invoke` method, providing structured, matchable error information to the runtime and orchestrator.

## Current State

- The `agent-sdk` crate contains four modules: `constraints`, `model_config`, `output_schema`, and `skill_manifest`. All are plain data structs deriving `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, and `JsonSchema`.
- `lib.rs` declares those four modules and re-exports their public types.
- `Cargo.toml` depends on `serde` (with `derive` feature) and `schemars` (with `derive` feature). No `thiserror` or other error-handling crate is present.
- No error types exist in the crate yet.
- The `Constraints` struct already defines `confidence_threshold: f64` and `max_turns: u32`. The `AgentError` enum uses `f32` for confidence and `u32` for turns as specified in the task description. Note the type difference between `Constraints.confidence_threshold` (f64) and `AgentError::ConfidenceTooLow.confidence`/`.threshold` (f32) -- this is intentional per the task spec and should be preserved as-is.

## Requirements

1. **File location:** `crates/agent-sdk/src/agent_error.rs`.

2. **Enum definition:** A public enum named `AgentError` with exactly four variants:
   - `ToolCallFailed { tool: String, reason: String }` -- A tool invocation failed. `tool` identifies which tool, `reason` describes the failure.
   - `ConfidenceTooLow { confidence: f32, threshold: f32 }` -- The agent's confidence score fell below the required threshold. `confidence` is the achieved score, `threshold` is the minimum required.
   - `MaxTurnsExceeded { turns: u32 }` -- The agent exhausted its allowed conversation turns without producing a satisfactory result. `turns` is the number of turns consumed.
   - `Internal(String)` -- A catch-all for unexpected internal errors. The `String` carries a human-readable description.

3. **Derive macros:** `Debug`, `Clone`, `PartialEq`. Unlike the existing data structs in the crate, this type does **not** derive `Serialize`, `Deserialize`, or `JsonSchema`. Error types are not serialized over the wire in the current architecture. If serialization is needed later, it can be added without breaking changes.

4. **`std::fmt::Display` implementation:** A manual `impl std::fmt::Display for AgentError` that produces a human-readable message for each variant:
   - `ToolCallFailed` -- e.g., `"tool call failed: {tool}: {reason}"`
   - `ConfidenceTooLow` -- e.g., `"confidence too low: {confidence} < {threshold}"`
   - `MaxTurnsExceeded` -- e.g., `"max turns exceeded: {turns}"`
   - `Internal` -- e.g., `"internal error: {message}"`

   The exact wording is not load-bearing, but each variant must include all of its field values in the output, and the messages must be lowercase (no leading capital) to follow Rust error-message conventions.

5. **`std::error::Error` implementation:** An `impl std::error::Error for AgentError {}` block. Since none of the variants wrap another error type as a source, the default `source()` returning `None` is correct. No override is needed.

6. **No additional dependencies.** The `Display` and `Error` implementations use only `std`. Do not add `thiserror` or any other crate.

7. **No `#[non_exhaustive]` attribute.** The enum is internal to the `spore` workspace. Adding `#[non_exhaustive]` would require downstream `match` arms to include a wildcard, which hurts exhaustiveness checking. If the crate is published externally in the future, `#[non_exhaustive]` can be added at that time.

## Implementation Details

### File: `crates/agent-sdk/src/agent_error.rs` (new)

- **Imports:** `use std::fmt;` -- only standard library formatting is needed.
- **Enum declaration:** Four variants as specified above, with `#[derive(Debug, Clone, PartialEq)]`.
- **`Display` impl:** Use `match self` with `write!(f, ...)` for each variant. Format floating-point values with default precision (no fixed decimal places) so that `0.5_f32` renders as `"0.5"`, not `"0.50"`.
- **`Error` impl:** Empty impl block -- the default trait methods are sufficient.

### File: `crates/agent-sdk/src/lib.rs` (modified -- but NOT by this task)

This task does **not** modify `lib.rs`. The downstream task "Update `lib.rs` with new module declarations and re-exports" will add:
```rust
mod agent_error;
pub use agent_error::AgentError;
```

### Integration points

- `AgentError` is the `Err` type in the return of `MicroAgent::invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`.
- The `ConfidenceTooLow` variant corresponds to the `confidence_threshold` field in `Constraints`. The runtime will construct this variant when the agent's reported confidence falls below the configured threshold.
- The `MaxTurnsExceeded` variant corresponds to the `max_turns` field in `Constraints`.
- The `ToolCallFailed` variant will be constructed by tool-execution infrastructure (not yet built).

## Dependencies

- **Blocked by:** Nothing. This task has no compile-time dependency on other Group 2 tasks or on Group 1 dependency additions (it uses only `std`).
- **Parallel with:** "Define `ToolCallRecord` struct", "Define `AgentRequest` struct", "Define `AgentResponse` struct", "Define `HealthStatus` enum" (all Group 2).
- **Blocking:** "Define `MicroAgent` trait" (Group 3), which uses `AgentError` as the error type in the `invoke` method's return type.

## Risks & Edge Cases

1. **f32 vs f64 mismatch with `Constraints`:** The `Constraints` struct uses `f64` for `confidence_threshold`, while `AgentError::ConfidenceTooLow` uses `f32` for both `confidence` and `threshold`. This means the runtime code that constructs `ConfidenceTooLow` will need an `as f32` cast. This is specified by the task description and is acceptable -- confidence scores are inherently imprecise, and `f32` provides sufficient resolution for values in `[0.0, 1.0]`. A future task may align these types, but that is out of scope here.

2. **`PartialEq` with f32 fields:** The `PartialEq` derive on `AgentError` will use `f32::eq` for the `ConfidenceTooLow` variant, which means `NaN != NaN`. This is standard Rust behavior and is acceptable because confidence scores should never be `NaN`. Test code comparing `ConfidenceTooLow` values should use concrete, non-NaN floats.

3. **Display format stability:** Downstream code (tests, logging) may depend on the exact `Display` output. The format strings should be treated as semi-stable. Tests in Group 4 ("Write serialization tests for envelope types") will assert on `Display` output, so any format change must update those tests.

4. **No `From` conversions:** This task does not implement `From<std::io::Error>` or other `From` conversions. If needed, those will be added by future tasks that introduce fallible operations producing standard error types.

5. **Thread safety:** `AgentError` contains only `String`, `f32`, and `u32` -- all `Send + Sync`. The derived `Clone` is cheap for small error messages. No concerns for use across async task boundaries.

## Verification

1. After creating the file, run `cargo check -p agent-sdk` to confirm the enum and its trait impls compile.
2. Run `cargo clippy -p agent-sdk` to confirm no lint warnings. In particular, clippy should not flag the manual `Display` impl (clippy only warns about `Display` on structs with a single field, not enums).
3. Run `cargo test -p agent-sdk` to confirm existing tests still pass.
4. Verify that the file contains no `use serde`, `use schemars`, or other external crate imports -- only `std::fmt`.
5. Full `Display` output testing and `Error` trait verification are handled by the Group 4 task ("Write serialization tests for envelope types"), not this task.
