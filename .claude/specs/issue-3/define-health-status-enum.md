# Spec: Define `HealthStatus` enum

> From: .claude/tasks/issue-3.md

## Objective
Define a `HealthStatus` enum that represents the three possible states of a micro-agent's health: fully operational, partially degraded with a reason, or unhealthy with a reason. This type is consumed by the `MicroAgent` trait's `health()` method, enabling the runtime and orchestrator to monitor agent availability and make routing decisions.

## Current State
The `agent-sdk` crate already contains several types (`ModelConfig`, `Constraints`, `OutputSchema`, `SkillManifest`) that follow a consistent pattern:
- Each type lives in its own module file under `crates/agent-sdk/src/`
- All types derive `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`
- Modules are declared in `lib.rs` with corresponding `pub use` re-exports
- `serde` and `schemars` are the only current dependencies (no `thiserror`, no `strum`)

There are no enums in the crate yet; all existing types are structs. This will be the first enum, so its pattern will set precedent for `AgentError` (another enum in Group 2).

## Requirements
- Define a `HealthStatus` enum in a new file `crates/agent-sdk/src/health_status.rs`
- The enum must have exactly three variants:
  - `Healthy` (unit variant, no associated data)
  - `Degraded(String)` (tuple variant carrying a human-readable reason)
  - `Unhealthy(String)` (tuple variant carrying a human-readable reason)
- Derive `Debug`, `Clone`, `PartialEq`, `Serialize`, `Deserialize` as specified in the task
- Also derive `JsonSchema` to stay consistent with every other type in the crate (all existing types derive it)
- The enum must be public (`pub enum`)
- Do NOT add the module declaration or re-export to `lib.rs` in this task; that is handled by the "Update `lib.rs`" task in Group 3
- Do NOT add any new dependencies; `serde` and `schemars` are already available

## Implementation Details

### File to create: `crates/agent-sdk/src/health_status.rs`

- Add `use schemars::JsonSchema;` and `use serde::{Deserialize, Serialize};` imports, matching the import style in `model_config.rs`
- Define `pub enum HealthStatus` with the derive macro `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]`, matching the exact derive order used by all other types in the crate
- Three variants as specified: `Healthy`, `Degraded(String)`, `Unhealthy(String)`
- No additional methods, trait implementations, or `impl` blocks are required for this task
- No `Default` impl; there is no obvious universal default health state

### Integration points
- `MicroAgent::health(&self) -> HealthStatus` (Group 3) will return this type
- `lib.rs` will later add `mod health_status;` and `pub use health_status::HealthStatus;` (Group 3)

## Dependencies
- Blocked by: None (this is a Group 2 task with only Group 1 as a prerequisite, and Group 1 adds `uuid`/`serde_json`/`async-trait` which are not needed here; existing `serde` and `schemars` deps suffice)
- Blocking: "Define `MicroAgent` trait" (the trait's `health()` method returns `HealthStatus`)

## Risks & Edge Cases
- **Derive consistency**: The task description lists `Serialize, Deserialize` but omits `JsonSchema`. Every other type in the crate derives `JsonSchema`. The implementation should include `JsonSchema` to maintain consistency; omitting it would make this the only type in the crate without schema generation support.
- **Serde representation**: By default, serde serializes Rust enums with associated data as externally tagged JSON (e.g., `{"Degraded": "reason"}`). The unit variant `Healthy` serializes as `"Healthy"`. This default is reasonable and should not be overridden unless a different wire format is later required.
- **String payload semantics**: The `String` in `Degraded` and `Unhealthy` is a free-form human-readable reason. There is no structured error code. This is intentional for simplicity but means consumers cannot programmatically branch on specific degradation causes. This is acceptable for an initial implementation.

## Verification
- `cargo check -p agent-sdk` compiles without errors (the module file compiles even before it is wired into `lib.rs`, but full verification requires the `lib.rs` update in Group 3)
- After `lib.rs` is updated (Group 3), `cargo test -p agent-sdk` passes
- After `lib.rs` is updated, confirm that `HealthStatus` is accessible as `agent_sdk::HealthStatus` from integration tests
- Round-trip serialization: a value like `HealthStatus::Degraded("cache miss".into())` serializes to JSON and deserializes back to an equal value
- All three variants can be constructed and compared with `PartialEq`
- `cargo clippy -p agent-sdk` produces no warnings
