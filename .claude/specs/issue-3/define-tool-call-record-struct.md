# Spec: Define `ToolCallRecord` struct

> From: .claude/tasks/issue-3.md

## Objective

Create a `ToolCallRecord` struct in `crates/agent-sdk/src/tool_call_record.rs` that represents a single tool invocation made during agent execution. This struct captures the tool name, its JSON input, and its JSON output. It will be composed into `AgentResponse` as a `Vec<ToolCallRecord>`, providing an audit trail of tool usage for each agent invocation.

## Current State

- The `agent-sdk` crate already has four struct modules (`model_config.rs`, `constraints.rs`, `output_schema.rs`, `skill_manifest.rs`) following a consistent pattern: one struct per file, public fields, derives for `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`, and minimal imports.
- `lib.rs` declares each module with `mod <name>;` and re-exports the primary type with `pub use <name>::<Type>;`.
- `Cargo.toml` currently has `serde = { version = "1", features = ["derive"] }` and `schemars = { version = "0.8", features = ["derive"] }` as dependencies. Notably, `serde_json` is **not** yet a dependency -- a separate Group 1 task ("Add `uuid` and `serde_json` dependencies") must add `serde_json = "1"` before this struct can compile.
- No `tool_call_record.rs` file exists in the crate.

## Requirements

1. Create the file `crates/agent-sdk/src/tool_call_record.rs`.
2. Define a public struct `ToolCallRecord` with exactly three public fields:
   - `tool_name: String` -- the name of the tool that was invoked (e.g., `"search"`, `"calculator"`).
   - `input: serde_json::Value` -- the JSON payload sent to the tool.
   - `output: serde_json::Value` -- the JSON payload returned by the tool.
3. Derive the following traits on `ToolCallRecord`: `Debug`, `Clone`, `Serialize`, `Deserialize`.
4. Additionally derive `PartialEq` and `JsonSchema` to maintain consistency with every other struct in the crate (`ModelConfig`, `Constraints`, `OutputSchema`, `SkillManifest` all derive these).
5. The file must include the necessary `use` imports for the derived traits (`serde::Serialize`, `serde::Deserialize`, `schemars::JsonSchema`).
6. No `#[serde(rename)]` attributes are required -- the Rust field names should be used as-is for serialization.
7. Do **not** add `#[serde(deny_unknown_fields)]` -- keep the struct simple and forward-compatible.

## Implementation Details

- **File path:** `crates/agent-sdk/src/tool_call_record.rs`
- **Struct visibility:** `pub struct ToolCallRecord`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use schemars::JsonSchema;`
  - `use serde::{Deserialize, Serialize};`
- **No constructor method needed** -- the struct is simple enough for direct construction.
- This module will later be declared in `lib.rs` via `mod tool_call_record;` and re-exported via `pub use tool_call_record::ToolCallRecord;`, but that wiring is handled by the "Update `lib.rs` with new module declarations and re-exports" task in Group 3.

### Reference pattern (from `model_config.rs`)

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelConfig {
    pub provider: String,
    pub name: String,
    pub temperature: f64,
}
```

The `ToolCallRecord` struct should follow this exact pattern, substituting the appropriate fields and using `serde_json::Value` for the `input` and `output` fields.

### Example serialized form (JSON)

```json
{
  "tool_name": "search",
  "input": { "query": "weather in London" },
  "output": { "result": "15C, partly cloudy" }
}
```

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1). The `serde_json::Value` type will not resolve without `serde_json` in `Cargo.toml` dependencies. `serde_json::Value` also requires `schemars` to have its `JsonSchema` implementation available, which is provided by the `schemars` crate's built-in support for `serde_json::Value` when both crates are present.
- **Blocking:** "Define `AgentResponse` struct" (Group 2). `AgentResponse` composes `ToolCallRecord` as its `tool_calls: Vec<ToolCallRecord>` field.

## Risks & Edge Cases

1. **`serde_json` dependency ordering:** If this task is attempted before the Group 1 dependency task adds `serde_json = "1"` to `Cargo.toml`, `cargo check` will fail with an unresolved import. The implementation must not proceed until that dependency is available.
2. **`serde_json::Value` and `PartialEq`:** `serde_json::Value` implements `PartialEq`, so deriving `PartialEq` on `ToolCallRecord` will compile correctly. However, floating-point values inside JSON may exhibit the usual IEEE 754 comparison caveats (e.g., `NaN != NaN`). This is an inherent property of `serde_json::Value` and not something this struct needs to mitigate.
3. **`JsonSchema` for `serde_json::Value`:** The `schemars` crate provides a `JsonSchema` implementation for `serde_json::Value` out of the box (the schema is effectively "any valid JSON"). This derive will compile without additional configuration, but the generated schema for `input` and `output` will be permissive (accepts any JSON). This is the correct behavior for an untyped tool I/O record.
4. **Field naming -- `tool_name` vs `name`:** The field is named `tool_name` rather than `name` to avoid ambiguity when the struct is used alongside other types that also have a `name` field (e.g., `ModelConfig::name`, `SkillManifest::name`). This is a deliberate design choice from the task description.
5. **Large payloads:** `serde_json::Value` stores the entire JSON tree in memory. For agents making tool calls with very large inputs or outputs, this could consume significant memory. This is acceptable for the current design and can be revisited if needed.

## Verification

After implementation (and after the dependency task is complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Full `cargo test` validation (including round-trip serialization tests) is covered by the separate "Write serialization tests for envelope types" task in Group 4.
