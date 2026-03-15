# Spec: Define `AgentResponse` struct

> From: .claude/tasks/issue-3.md

## Objective

Create an `AgentResponse` struct in `crates/agent-sdk/src/agent_response.rs` that represents the standardized response envelope returned by an agent after processing a request. This struct is the return type of `MicroAgent::invoke()` and carries the agent's output, confidence level, escalation flag, and a log of tool calls made during the turn. It enables the orchestrator to inspect results, decide on escalation, and trace tool usage across agents.

## Current State

- The `agent-sdk` crate has four existing types defined in individual files following a one-struct-per-file pattern: `ModelConfig`, `Constraints`, `OutputSchema`, `SkillManifest`.
- Each struct file uses `use serde::{Deserialize, Serialize};` and `use schemars::JsonSchema;` imports, and derives `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.
- `lib.rs` currently declares modules `constraints`, `model_config`, `output_schema`, `skill_manifest` with corresponding `pub use` re-exports.
- `Cargo.toml` currently has `serde` (with `derive` feature) and `schemars` (with `derive` feature) as dependencies, and `serde_yaml` as a dev-dependency. It does **not** yet have `uuid` or `serde_json` as direct dependencies.
- `ToolCallRecord` does not yet exist. It is defined as a sibling task and must be implemented before this struct.
- No `agent_response.rs` file exists in the crate.

## Requirements

1. Create the file `crates/agent-sdk/src/agent_response.rs`.
2. Define a public struct `AgentResponse` with exactly five public fields:
   - `id: uuid::Uuid` -- unique identifier for the response, typically matching the corresponding request ID.
   - `output: serde_json::Value` -- the agent's output payload as arbitrary JSON. Using `Value` allows agents to return structured data of any shape without requiring a fixed schema at the SDK level.
   - `confidence: f32` -- the agent's self-assessed confidence in its output, in the range `[0.0, 1.0]`. Used by the orchestrator to decide whether to accept the result or escalate. Uses `f32` (not `f64`) because confidence scores do not require double precision.
   - `escalated: bool` -- whether the agent determined it should escalate this response to a higher-level agent or human reviewer.
   - `tool_calls: Vec<ToolCallRecord>` -- ordered log of tool invocations made during this turn. Empty if the agent made no tool calls.
3. Derive the following traits: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
   - Note: The task description in issue-3 says to derive `Debug, Clone, Serialize, Deserialize`. However, the issue-4 task description (which defines the same struct) adds `PartialEq` and `JsonSchema`, consistent with all existing structs in the crate. Follow the issue-4 specification and the existing crate pattern: derive all six traits.
4. Add a convenience constructor `AgentResponse::success(id: uuid::Uuid, output: serde_json::Value) -> Self` that returns an `AgentResponse` with `confidence: 1.0`, `escalated: false`, and `tool_calls: vec![]`. This provides a quick way to construct a successful, fully-confident response without tool calls.
5. Import `ToolCallRecord` from `crate::tool_call_record`.
6. Field names must match their serialized JSON/YAML key names exactly -- no `#[serde(rename)]` attributes needed.
7. The file must include the necessary `use` imports for derived traits and field types.

## Implementation Details

- **File path:** `crates/agent-sdk/src/agent_response.rs`
- **Struct visibility:** `pub struct AgentResponse`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use serde::{Deserialize, Serialize};`
  - `use schemars::JsonSchema;`
  - `use crate::tool_call_record::ToolCallRecord;`
  - `uuid::Uuid` and `serde_json::Value` used via fully-qualified paths in field types, or imported at the top -- importing is preferred for consistency with how other crates typically use these types.
- **Constructor:**
  ```rust
  impl AgentResponse {
      pub fn success(id: uuid::Uuid, output: serde_json::Value) -> Self {
          Self {
              id,
              output,
              confidence: 1.0,
              escalated: false,
              tool_calls: vec![],
          }
      }
  }
  ```
- **No `#[serde(...)]` attributes required** -- the Rust field names (`id`, `output`, `confidence`, `escalated`, `tool_calls`) already match the expected serialized keys.
- **No `Option` wrappers** -- all five fields are required. An empty `tool_calls` is represented as an empty `Vec`, not as `None`.
- **No `#[serde(deny_unknown_fields)]`** -- keep the struct forward-compatible with future field additions.
- This module will be declared in `lib.rs` via `mod agent_response;` and re-exported via `pub use agent_response::AgentResponse;`, but that wiring is handled by a separate task ("Update `lib.rs` with new module declarations and re-exports").

### Integration points

- `ToolCallRecord` (sibling module): referenced as a field type via `Vec<ToolCallRecord>`. Must be importable from `crate::tool_call_record`.
- `MicroAgent::invoke()` (downstream, issue #3 Group 3): returns `Result<AgentResponse, AgentError>`. This struct is the `Ok` variant.
- `Constraints::confidence_threshold` (existing): the orchestrator will compare `AgentResponse::confidence` against this threshold to decide on escalation. The types are `f32` vs `f64` -- the comparison site (not this struct) is responsible for casting.

## Dependencies

- **Blocked by:**
  - "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1) -- `uuid::Uuid` and `serde_json::Value` field types require these crate dependencies.
  - "Define `ToolCallRecord` struct" (Group 2) -- the `tool_calls` field uses `Vec<ToolCallRecord>`, so that type must exist for this file to compile.
- **Blocking:**
  - "Define `MicroAgent` trait" (Group 3) -- the trait's `invoke` method returns `Result<AgentResponse, AgentError>`.
  - "Update `lib.rs` with new module declarations and re-exports" (Group 3) -- cannot add the module declaration until the file exists.
  - "Write serialization and construction tests" (Group 4, issue #4) -- tests exercise round-trip serialization and the `success()` constructor.

## Risks & Edge Cases

1. **`confidence` precision (`f32`):** Using `f32` is intentional per the task description to signal that confidence does not need double precision. However, serde serializes `f32` with potential floating-point representation artifacts (e.g., `0.8500000238418579` instead of `0.85`). Tests that compare deserialized confidence values should use approximate matching (e.g., `(value - expected).abs() < f32::EPSILON`) rather than exact equality. The `PartialEq` derive uses bitwise float comparison, which is fine for identical round-trip values but can surprise if constructing from different literal sources.
2. **`PartialEq` with `serde_json::Value`:** `serde_json::Value` implements `PartialEq`, so the derive works. However, JSON number comparisons may behave unexpectedly for floating-point values within the `output` field. This is acceptable for the SDK's purposes.
3. **`PartialEq` with `f32`:** Deriving `PartialEq` on a struct with `f32` fields means `NaN != NaN`. If a confidence value is ever `NaN` (which should not happen in practice), two otherwise-identical responses would not be equal. This is standard Rust behavior and not worth mitigating at the struct level.
4. **Missing dependency crates:** If this task is attempted before `uuid` and `serde_json` are added to `Cargo.toml`, compilation will fail. The implementation must not proceed until those dependencies are available.
5. **`ToolCallRecord` not yet defined:** If this task is attempted before `ToolCallRecord` exists, the `use crate::tool_call_record::ToolCallRecord` import will fail. Respect the dependency ordering.
6. **Future fields:** Additional response metadata (e.g., `duration_ms`, `tokens_used`, `trace_id`) may be added later. The struct is kept simple for now; `#[serde(deny_unknown_fields)]` is intentionally omitted to allow forward-compatible deserialization.

## Verification

After implementation (and after all blocking dependency tasks are complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Additionally confirm:

- The struct has exactly five public fields with the specified names and types.
- All six traits (`Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`) are derived.
- The `success()` constructor compiles and returns correct defaults.
- The `ToolCallRecord` import resolves correctly.

Full `cargo test` validation of serialization round-trips and constructor behavior is covered by the separate test task in Group 4.
