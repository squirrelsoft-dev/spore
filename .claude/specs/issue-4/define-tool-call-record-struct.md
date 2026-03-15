# Spec: Define `ToolCallRecord` struct

> From: .claude/tasks/issue-4.md

## Objective

Create a `ToolCallRecord` struct in `crates/agent-sdk/src/tool_call_record.rs` that records a single tool invocation within an agent's turn. This is a pure data carrier with no methods or impl blocks. It captures the tool name, the JSON input sent to the tool, and the JSON output returned by the tool. `ToolCallRecord` will be composed into `AgentResponse` (as `Vec<ToolCallRecord>`) to provide an audit trail of all tool calls made during a response.

## Current State

- `crates/agent-sdk/src/lib.rs` declares four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) and re-exports their primary types.
- The crate follows a one-struct-per-file pattern. Each file has `use serde::{Deserialize, Serialize};` and `use schemars::JsonSchema;` at the top, followed by a single `pub struct` with derived traits and public fields.
- `crates/agent-sdk/Cargo.toml` currently has `serde` (with `derive` feature) and `schemars` (with `derive` feature) as dependencies. It does NOT yet have `serde_json` as a direct dependency.
- No `tool_call_record.rs` file exists in the crate.
- The `serde_json` crate dependency (needed for `serde_json::Value` fields) is handled by a separate Group 1 task ("Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`") that must complete before this file will compile.

## Requirements

1. Create the file `crates/agent-sdk/src/tool_call_record.rs`.
2. Define a public struct `ToolCallRecord` with exactly three public fields:
   - `tool_name: String` -- the name of the tool that was invoked (e.g., `"web_search"`, `"code_interpreter"`).
   - `input: serde_json::Value` -- the JSON payload passed as input to the tool. Uses `serde_json::Value` to accommodate arbitrary tool-specific schemas without requiring a concrete type.
   - `output: serde_json::Value` -- the JSON payload returned by the tool. Uses `serde_json::Value` for the same reason as `input`.
3. Derive the following traits on `ToolCallRecord`: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
4. Field names in the struct must use snake_case and will serialize to JSON with matching keys (`tool_name`, `input`, `output`), so no `#[serde(rename)]` attributes are needed.
5. The file must include the necessary `use` imports for the derived traits and field types:
   - `use serde::{Deserialize, Serialize};`
   - `use schemars::JsonSchema;`
6. No methods, impl blocks, or trait implementations beyond the derives.
7. No `#[serde(deny_unknown_fields)]` attribute -- keep the struct forward-compatible.

## Implementation Details

- **File path:** `crates/agent-sdk/src/tool_call_record.rs`
- **Struct visibility:** `pub struct ToolCallRecord`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use schemars::JsonSchema;`
  - `use serde::{Deserialize, Serialize};`
- **No `Option` wrappers** -- all three fields are required. A tool call record without a tool name, input, or output is not meaningful.
- **No default values** -- there is no sensible default for a tool call record; instances should always be constructed with explicit values.
- **`serde_json::Value` usage:** The `input` and `output` fields use the fully qualified path `serde_json::Value` rather than a `use serde_json::Value;` import, following the pattern of keeping imports minimal and the type origin clear.
- This module will later be declared in `lib.rs` via `mod tool_call_record;` and re-exported via `pub use tool_call_record::ToolCallRecord;`, but that wiring is handled by a separate task ("Update `lib.rs` module declarations and re-exports" in Group 4).

### Reference: existing pattern from `model_config.rs`

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

The `ToolCallRecord` struct follows this identical layout pattern, substituting its own fields.

### Expected file content

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input: serde_json::Value,
    pub output: serde_json::Value,
}
```

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1). The `serde_json::Value` type will not resolve without `serde_json` being listed as a direct dependency in `Cargo.toml`.
- **Blocks:** "Define `AgentResponse` struct" (Group 3). `AgentResponse` composes `ToolCallRecord` as its `tool_calls: Vec<ToolCallRecord>` field.

## Risks & Edge Cases

1. **`serde_json` not yet a direct dependency:** If this task is attempted before the Group 1 dependency task completes, `cargo check` will fail with an unresolved import for `serde_json::Value`. The implementation must not proceed until `Cargo.toml` is updated. While `serde_json` is a transitive dependency via `schemars`, relying on transitive dependencies for public API types is fragile and incorrect.
2. **`PartialEq` on `serde_json::Value`:** `serde_json::Value` implements `PartialEq`, so deriving `PartialEq` on `ToolCallRecord` will compile. However, JSON object key ordering and floating-point representation can produce surprising equality results (e.g., `1.0` vs `1`). This is acceptable for a data carrier; consumers should be aware of `Value` equality semantics.
3. **`JsonSchema` for `serde_json::Value`:** The `schemars` crate provides a `JsonSchema` implementation for `serde_json::Value` that generates an unrestricted schema (accepts any JSON value). This is the correct behavior for arbitrarily-typed tool inputs and outputs.
4. **Future extensibility:** Additional fields (e.g., `duration_ms: Option<u64>`, `error: Option<String>`) may be needed later. The absence of `#[serde(deny_unknown_fields)]` ensures that a newer serialized form with extra fields can still be deserialized by older code that lacks those fields, as long as the new fields are `Option`-wrapped.
5. **Large payloads:** `serde_json::Value` can hold arbitrarily large JSON trees. No size limits are enforced at the type level. If size constraints become necessary, they should be enforced at the application layer, not in this data struct.

## Verification

After implementation (and after the Group 1 dependency task is complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Additionally, verify:

- The file contains exactly one `pub struct` with three `pub` fields.
- The derive line includes all six required traits: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
- No `impl` blocks or methods are present in the file.
- The file follows the same structure as `model_config.rs` (imports, derive, struct definition, nothing else).

Full `cargo test` validation (including JSON round-trip serialization) is covered by the separate test task ("Write serialization and construction tests" in Group 5).
