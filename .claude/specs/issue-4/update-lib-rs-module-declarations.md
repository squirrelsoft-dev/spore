# Spec: Update `lib.rs` module declarations and re-exports

> From: .claude/tasks/issue-4.md

## Objective

Wire all five new modules (`agent_request`, `agent_response`, `agent_error`, `health_status`, `tool_call_record`) into the crate's public API by adding `mod` declarations and `pub use` re-exports in `crates/agent-sdk/src/lib.rs`. After this change, downstream consumers can import the new types directly from the crate root: `use agent_sdk::{AgentRequest, AgentResponse, AgentError, HealthStatus, ToolCallRecord}`.

## Current State

`crates/agent-sdk/src/lib.rs` currently declares four modules and re-exports one type from each:

```rust
mod constraints;
mod model_config;
mod output_schema;
mod skill_manifest;

pub use constraints::Constraints;
pub use model_config::ModelConfig;
pub use output_schema::OutputSchema;
pub use skill_manifest::SkillManifest;
```

The pattern is consistent: one `mod` line per module (private), followed by a `pub use` that re-exports the module's primary type. No grouping comments, no blank lines between `mod` declarations, one blank line separating the `mod` block from the `pub use` block.

## Requirements

- Add `mod` declarations for: `agent_request`, `agent_response`, `agent_error`, `health_status`, `tool_call_record`.
- Add `pub use` re-exports for: `agent_request::AgentRequest`, `agent_response::AgentResponse`, `agent_error::AgentError`, `health_status::HealthStatus`, `tool_call_record::ToolCallRecord`.
- Preserve all existing `mod` declarations and `pub use` re-exports unchanged.
- Follow the existing code style: private `mod` declarations in an alphabetically sorted block, `pub use` re-exports in an alphabetically sorted block, one blank line between the two blocks.
- Each new module file must exist (created by prior tasks) before this change can compile: `agent_error.rs`, `agent_request.rs`, `agent_response.rs`, `health_status.rs`, `tool_call_record.rs`.
- After this change, `cargo check` must pass (assuming all dependency modules are in place).

## Implementation Details

**File to modify:** `crates/agent-sdk/src/lib.rs`

The updated file should contain exactly:

```rust
mod agent_error;
mod agent_request;
mod agent_response;
mod constraints;
mod health_status;
mod model_config;
mod output_schema;
mod skill_manifest;
mod tool_call_record;

pub use agent_error::AgentError;
pub use agent_request::AgentRequest;
pub use agent_response::AgentResponse;
pub use constraints::Constraints;
pub use health_status::HealthStatus;
pub use model_config::ModelConfig;
pub use output_schema::OutputSchema;
pub use skill_manifest::SkillManifest;
pub use tool_call_record::ToolCallRecord;
```

Key decisions:
- All `mod` declarations remain private (no `pub mod`). Public access is via re-exports only, which keeps the module files as implementation details and the crate root as the sole public API surface.
- Alphabetical ordering of both `mod` and `pub use` blocks for consistency and to minimize merge conflicts.
- Each re-export targets exactly one primary type per module, matching the established one-type-per-file convention.

## Dependencies

- **Blocked by:** All Group 2 and Group 3 tasks:
  - "Define `ToolCallRecord` struct" (`tool_call_record.rs`)
  - "Define `AgentError` enum" (`agent_error.rs`)
  - "Define `HealthStatus` enum" (`health_status.rs`)
  - "Define `AgentRequest` struct" (`agent_request.rs`)
  - "Define `AgentResponse` struct" (`agent_response.rs`)
- **Blocking:** "Write serialization and construction tests" (Group 5)

## Risks & Edge Cases

- **Missing module files:** If any of the five `.rs` files do not yet exist when this change is applied, `cargo check` will fail with `file not found` errors. Mitigation: this task is explicitly sequenced after all module-creation tasks.
- **Name collisions:** None expected. The five new type names (`AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `ToolCallRecord`) do not collide with the four existing re-exports (`Constraints`, `ModelConfig`, `OutputSchema`, `SkillManifest`).
- **Re-export visibility of sub-types:** `AgentError` variants and method signatures reference standard library types (`String`, `f32`, `u32`) and `serde_json::Value` / `uuid::Uuid`, which are already public dependencies. No additional re-exports are needed for consumers to construct these types.

## Verification

- `cargo check` passes with no errors (confirms module wiring is correct).
- `cargo clippy` reports no warnings related to unused imports or module declarations.
- The following `use` statements compile in downstream test code:
  - `use agent_sdk::AgentRequest;`
  - `use agent_sdk::AgentResponse;`
  - `use agent_sdk::AgentError;`
  - `use agent_sdk::HealthStatus;`
  - `use agent_sdk::ToolCallRecord;`
  - `use agent_sdk::{AgentRequest, AgentResponse, AgentError, HealthStatus, ToolCallRecord};`
- Existing re-exports still work: `use agent_sdk::{Constraints, ModelConfig, OutputSchema, SkillManifest};`
