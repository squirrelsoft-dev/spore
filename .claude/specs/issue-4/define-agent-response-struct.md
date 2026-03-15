# Spec: Define `AgentResponse` struct

> From: .claude/tasks/issue-4.md

## Objective

Create the `AgentResponse` struct that serves as the standardized response envelope for inter-agent communication. An `AgentResponse` carries the output of a single agent invocation along with metadata about the agent's confidence in its answer, whether it escalated to a higher-level agent, and a record of any tool calls it made during execution. This type, together with `AgentRequest`, forms the core messaging contract of the agent-sdk.

## Current State

- `crates/agent-sdk/src/lib.rs` declares four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) and re-exports their primary types. No envelope types exist yet.
- The crate's `Cargo.toml` currently depends on `serde` (with `derive` feature) and `schemars` (with `derive` feature). The `uuid` and `serde_json` dependencies required by this struct are being added by a separate Group 1 task ("Add `uuid` and `serde_json` dependencies").
- The `ToolCallRecord` type that this struct depends on does not yet exist; it is being created by a parallel Group 2 task.
- Existing structs in the crate follow a consistent pattern: one struct per file, `use schemars::JsonSchema` and `use serde::{Deserialize, Serialize}` at the top, a single `#[derive(...)]` line, all fields `pub`, no `#[serde(...)]` attributes unless needed.

## Requirements

1. Create the file `crates/agent-sdk/src/agent_response.rs`.
2. Define a public struct `AgentResponse` with exactly five public fields:
   - `id: uuid::Uuid` -- a unique identifier correlating this response to its originating `AgentRequest`.
   - `output: serde_json::Value` -- the agent's output payload, represented as arbitrary JSON to allow flexible return shapes.
   - `confidence: f32` -- a value between 0.0 and 1.0 representing the agent's self-assessed confidence in its output.
   - `escalated: bool` -- whether the agent escalated the request to a parent or supervisor agent.
   - `tool_calls: Vec<ToolCallRecord>` -- an ordered log of tool invocations the agent performed while producing this response.
3. Derive the following traits on `AgentResponse`: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
4. Implement a convenience constructor `AgentResponse::success(id: uuid::Uuid, output: serde_json::Value) -> Self` that returns an `AgentResponse` with `confidence: 1.0`, `escalated: false`, and `tool_calls: vec![]`.
5. Import `ToolCallRecord` from `crate::tool_call_record`.
6. Field names must match their intended JSON/YAML serialization keys exactly (no `#[serde(rename)]` needed).

## Implementation Details

- **File path:** `crates/agent-sdk/src/agent_response.rs`
- **Struct visibility:** `pub struct AgentResponse`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use schemars::JsonSchema;`
  - `use serde::{Deserialize, Serialize};`
  - `use crate::tool_call_record::ToolCallRecord;`
- **`impl AgentResponse` block:** Contains a single public associated function:
  - `pub fn success(id: uuid::Uuid, output: serde_json::Value) -> Self` -- constructs a successful response with default confidence (1.0), no escalation, and an empty tool call log.
- This module will later be declared in `lib.rs` via `mod agent_response;` and re-exported via `pub use agent_response::AgentResponse;`, but that wiring is handled by a separate Group 4 task.

### Reference: existing pattern (from `constraints.rs`)

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,
    pub escalate_to: String,
    pub allowed_actions: Vec<String>,
}
```

The `AgentResponse` struct follows this same pattern but adds an `impl` block for the `success` constructor and uses cross-module imports (`crate::tool_call_record::ToolCallRecord`) following the pattern established in `skill_manifest.rs`.

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1) -- the `uuid::Uuid` and `serde_json::Value` types will not resolve without these dependencies in `Cargo.toml`.
- **Blocked by:** "Define `ToolCallRecord` struct" (Group 2) -- the `tool_calls` field's type depends on `ToolCallRecord` existing in `crate::tool_call_record`.
- **Blocking:** "Update `lib.rs` module declarations and re-exports" (Group 4) -- `lib.rs` cannot declare and re-export `AgentResponse` until this file exists.

## Risks & Edge Cases

1. **`confidence` as `f32` vs `f64`:** The task specifies `f32`, which is sufficient for a 0.0--1.0 range. However, `Constraints::confidence_threshold` uses `f64`. Comparisons between `AgentResponse::confidence` and a threshold from `Constraints` will require an explicit cast (`as f64` or `as f32`). This is an acceptable tradeoff documented here for awareness; changing the type would require updating the task definition.
2. **No validation on `confidence` range:** The struct does not enforce that `confidence` falls within [0.0, 1.0]. Runtime validation, if desired, should be added at the call site or in a future builder pattern. For now, keeping the struct as a plain data carrier is consistent with the crate's existing style.
3. **`PartialEq` with `f32`:** Deriving `PartialEq` on a struct containing `f32` uses bitwise float comparison, which works correctly for the `success` constructor's `1.0` literal but can be surprising for values produced by arithmetic. Tests should be aware of this; exact comparison is fine for known literal values.
4. **`PartialEq` with `serde_json::Value`:** `serde_json::Value` implements `PartialEq`, so deriving `PartialEq` on the struct will compile and work correctly.
5. **`JsonSchema` for `uuid::Uuid` and `serde_json::Value`:** The `uuid` crate (with the `serde` feature) and `serde_json` both provide `JsonSchema` implementations via `schemars` integration, so the derive will compile without issue.
6. **Dependency ordering:** If this task is attempted before `ToolCallRecord` or the `Cargo.toml` dependency tasks are complete, `cargo check` will fail. The implementation must not proceed until both blocking tasks are done.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Additionally, verify:

- The struct has exactly five fields with the specified names and types.
- The `success` constructor compiles and returns correct default values (this will be formally tested in the Group 5 test task).
- The file follows the existing one-struct-per-file pattern with no extraneous code, commented-out lines, or debug statements.
