# Spec: Define `AgentRequest` struct

> From: .claude/tasks/issue-4.md

## Objective

Create an `AgentRequest` struct in `crates/agent-sdk/src/agent_request.rs` that serves as the standardized inbound message envelope for inter-agent communication. Every request to an agent will arrive as an `AgentRequest`, carrying a unique identifier, the user/caller input, optional structured context, and an optional caller identity. A convenience constructor simplifies the common case of creating a request with only an input string.

## Current State

- `crates/agent-sdk/src/lib.rs` declares four modules (`constraints`, `model_config`, `output_schema`, `skill_manifest`) and re-exports their public types. No request/response envelope types exist yet.
- The crate already depends on `serde` (with `derive` feature) and `schemars` (with `derive` feature) in `Cargo.toml`.
- The crate does NOT yet depend on `uuid` or `serde_json`, which are required by this struct. Those dependencies are being added by the separate "Add `uuid` and `serde_json` dependencies" task in Group 1 of issue-4.
- Existing structs (e.g., `ModelConfig`) follow a consistent one-struct-per-file pattern: imports at the top, a single `#[derive(...)]` public struct with public fields, no impl blocks for pure data carriers.

## Requirements

1. Create the file `crates/agent-sdk/src/agent_request.rs`.
2. Define a public struct `AgentRequest` with exactly four public fields:
   - `id: uuid::Uuid` -- a unique identifier for this request, used for correlation with the corresponding `AgentResponse`.
   - `input: String` -- the primary input payload (e.g., a user prompt or inter-agent instruction).
   - `context: Option<serde_json::Value>` -- optional structured context (e.g., prior conversation state, environment metadata). `None` when no context is provided.
   - `caller: Option<String>` -- optional identifier of the calling agent or system. `None` for top-level (human-initiated) requests.
3. Derive the following traits: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
4. Implement `AgentRequest::new(input: String) -> Self` as a convenience constructor that:
   - Auto-generates a v4 UUID via `uuid::Uuid::new_v4()`.
   - Sets `context` to `None`.
   - Sets `caller` to `None`.
5. Field names must match their intended JSON/YAML serialization keys verbatim -- no `#[serde(rename)]` attributes should be needed.

## Implementation Details

- **File path:** `crates/agent-sdk/src/agent_request.rs`
- **Struct visibility:** `pub struct AgentRequest`
- **Field visibility:** All fields `pub`
- **Derive line:** `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`
- **Imports needed:**
  - `use serde::{Serialize, Deserialize};`
  - `use schemars::JsonSchema;`
  - `use uuid::Uuid;` (used in the `id` field type and in the `new()` constructor)
  - `use serde_json::Value;` (used in the `context` field type)
- **Impl block:** A single `impl AgentRequest` block with one associated function:
  ```
  pub fn new(input: String) -> Self
  ```
  This constructor creates an `AgentRequest` with a fresh `Uuid::new_v4()`, the provided `input`, and `context: None`, `caller: None`.
- **No `#[serde(deny_unknown_fields)]`** -- keep the struct forward-compatible for future field additions.
- **No `Default` impl** -- `AgentRequest` has no meaningful default (the `input` field is required and has no sensible zero value).
- This module will be declared in `lib.rs` via `mod agent_request;` and re-exported via `pub use agent_request::AgentRequest;`, but that wiring is handled by the separate "Update `lib.rs` module declarations and re-exports" task in Group 4.

## Dependencies

- **Blocked by:** "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1). The `uuid::Uuid` and `serde_json::Value` types will not resolve without those crate dependencies.
- **Blocking:** "Update `lib.rs` module declarations and re-exports" (Group 4). The module must exist before `lib.rs` can declare and re-export it.

## Risks & Edge Cases

1. **UUID v4 randomness in tests:** `Uuid::new_v4()` produces a random value on each call. Tests that use `AgentRequest::new()` cannot assert an exact UUID value; they should verify the UUID is non-nil (`!id.is_nil()`) and is version 4 (`id.get_version() == Some(uuid::Version::Random)`). This is a concern for the test task, not this struct definition.
2. **`serde_json::Value` in `context`:** This is intentionally untyped to allow arbitrary structured data. Consumers must handle the `Value` variants they expect. Misuse (e.g., passing a `Value::Null` instead of `None`) is semantically confusing but not harmful -- `Option<Value>` with `Some(Value::Null)` and `None` serialize differently in JSON (`"context": null` vs. field absent with `skip_serializing_if`). For simplicity, do NOT add `#[serde(skip_serializing_if = "Option::is_none")]` in this task -- keep serialization explicit and consistent.
3. **`uuid` crate `serde` feature:** The `uuid` dependency must be added with `features = ["v4", "serde"]`. Without the `serde` feature, `Uuid` will not implement `Serialize`/`Deserialize`, causing compile errors on the derive. Without `v4`, `Uuid::new_v4()` will not be available. This is the responsibility of the Group 1 dependency task.
4. **`uuid` crate `JsonSchema` support:** The `schemars` crate has built-in support for `uuid::Uuid` via an optional feature (`schemars/uuid1`). If `schemars` is not compiled with this feature, deriving `JsonSchema` on a struct containing `Uuid` will fail. Verify that `schemars = { version = "0.8", features = ["derive"] }` includes `Uuid` support (it does for `uuid` 1.x by default in `schemars` 0.8). If not, the dependency task must add the `uuid1` feature to `schemars`.
5. **Dependency not yet added:** If this task is attempted before the Group 1 dependency task completes, `cargo check` will fail on missing `uuid` and `serde_json` crates.

## Verification

After implementation (and after the dependency task is complete), run:

```bash
cargo check -p agent-sdk
cargo clippy -p agent-sdk
```

Both commands must pass with no errors and no warnings. Additionally, verify:

- The struct has exactly four fields with the specified names and types.
- The derive line includes all six traits: `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize`, `JsonSchema`.
- `AgentRequest::new("test".into())` compiles and produces a value where `context.is_none()` and `caller.is_none()`.
- Full `cargo test` validation (round-trip serialization, constructor assertions) is covered by the separate "Write serialization and construction tests" task in Group 5.
