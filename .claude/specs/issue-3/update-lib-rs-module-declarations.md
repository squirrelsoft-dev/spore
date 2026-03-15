# Spec: Update `lib.rs` with new module declarations and re-exports

> From: .claude/tasks/issue-3.md

## Objective

Extend `crates/agent-sdk/src/lib.rs` to declare and re-export all six new modules introduced by issues #3 and #4 (`tool_call_record`, `agent_request`, `agent_response`, `agent_error`, `health_status`, `micro_agent`), plus a convenience re-export of the `async_trait` proc macro. After this change, downstream crates can import every public type from the crate root (e.g., `use agent_sdk::MicroAgent;`) and can apply `#[agent_sdk::async_trait]` when implementing the trait without declaring `async-trait` as a direct dependency.

## Current State

`crates/agent-sdk/src/lib.rs` currently declares four private modules and re-exports one type from each:

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

This was established by the issue #2 work. The file uses the "facade" pattern: modules are private (`mod`, not `pub mod`) and only the primary types are re-exported via `pub use`. There are no doc comments, feature flags, or inline tests.

By the time this task executes, the following new source files will exist under `crates/agent-sdk/src/`:

| File | Primary export(s) | Added by |
|---|---|---|
| `tool_call_record.rs` | `ToolCallRecord` (struct) | Issue #4 / Group 2 |
| `agent_request.rs` | `AgentRequest` (struct) | Issue #4 / Group 2 |
| `agent_response.rs` | `AgentResponse` (struct) | Issue #4 / Group 2 |
| `agent_error.rs` | `AgentError` (enum) | Issue #4 / Group 2 |
| `health_status.rs` | `HealthStatus` (enum) | Issue #4 / Group 2 |
| `micro_agent.rs` | `MicroAgent` (trait) | Issue #3 / Group 3 |

Additionally, `async-trait` will already be listed as a dependency in `Cargo.toml` (added by the "Add `async-trait` dependency" task).

## Requirements

1. **Add six `mod` declarations.** Declare the following private modules, appended after the existing four declarations:
   - `mod tool_call_record;`
   - `mod agent_request;`
   - `mod agent_response;`
   - `mod agent_error;`
   - `mod health_status;`
   - `mod micro_agent;`

   The modules must remain private (`mod`, not `pub mod`), consistent with the existing pattern.

2. **Add six `pub use` re-exports.** Re-export the primary type from each new module:
   - `pub use tool_call_record::ToolCallRecord;`
   - `pub use agent_request::AgentRequest;`
   - `pub use agent_response::AgentResponse;`
   - `pub use agent_error::AgentError;`
   - `pub use health_status::HealthStatus;`
   - `pub use micro_agent::MicroAgent;`

3. **Re-export `async_trait`.** Add a pub use for the `async_trait` attribute macro so downstream implementors of `MicroAgent` do not need to add `async-trait` as a direct dependency:
   ```rust
   pub use async_trait::async_trait;
   ```

4. **Preserve existing declarations.** All four existing `mod` declarations and their `pub use` re-exports must remain unchanged and in their current position.

5. **Ordering convention.** Group the declarations logically:
   - Existing config/manifest modules first (already present, unchanged).
   - New envelope types next (`tool_call_record`, `agent_request`, `agent_response`, `agent_error`, `health_status`), ordered so that types with no intra-crate dependencies come before types that depend on others (i.e., `tool_call_record` before `agent_response`, since `AgentResponse` contains `Vec<ToolCallRecord>`).
   - The `micro_agent` module last, since the trait depends on all the types above.
   - The `async_trait` re-export last, separated by a blank line, since it is a third-party re-export rather than a module declaration.

6. **No `pub mod`.** No module should be made public. The internal module structure remains an implementation detail.

7. **No new dependencies.** This task modifies only `lib.rs`. All dependency changes (`async-trait`, `uuid`, `serde_json`) belong to their respective prerequisite tasks.

8. **No tests in `lib.rs`.** Integration tests are handled by separate tasks in Group 4.

9. **No doc comments required at this stage.** Doc comments on the crate root or re-exports can be added in a later documentation pass.

## Implementation Details

**File to modify:** `crates/agent-sdk/src/lib.rs`

The final file contents should be:

```rust
mod constraints;
mod model_config;
mod output_schema;
mod skill_manifest;

mod tool_call_record;
mod agent_request;
mod agent_response;
mod agent_error;
mod health_status;
mod micro_agent;

pub use constraints::Constraints;
pub use model_config::ModelConfig;
pub use output_schema::OutputSchema;
pub use skill_manifest::SkillManifest;

pub use tool_call_record::ToolCallRecord;
pub use agent_request::AgentRequest;
pub use agent_response::AgentResponse;
pub use agent_error::AgentError;
pub use health_status::HealthStatus;
pub use micro_agent::MicroAgent;

pub use async_trait::async_trait;
```

### Key design decisions

- **`pub use async_trait::async_trait`**: This is the idiomatic Rust approach for convenience re-exports of proc macros that are part of a crate's public API contract. Without this, every crate implementing `MicroAgent` would need `async-trait = "0.1"` in its own `Cargo.toml` and a `use async_trait::async_trait;` import. The re-export avoids version skew (all implementors use the same `async-trait` version) and reduces boilerplate. Downstream usage becomes:
  ```rust
  use agent_sdk::{async_trait, MicroAgent, AgentRequest, AgentResponse, AgentError, HealthStatus, SkillManifest};

  #[async_trait]
  impl MicroAgent for MyAgent { ... }
  ```

- **Module ordering**: Placing `tool_call_record` before `agent_response` mirrors the dependency relationship (`AgentResponse` uses `ToolCallRecord`). While Rust does not require any specific `mod` declaration order, maintaining dependency order improves readability and makes the module graph easier to understand at a glance.

- **Blank-line grouping**: Three logical groups separated by blank lines -- (1) existing config modules, (2) new type modules, (3) third-party re-export -- make the file scannable without being verbose.

### No new types, functions, or interfaces

This task only wires existing modules into the crate's public API. It introduces no new logic.

## Dependencies

- **Blocked by:**
  - "Define `MicroAgent` trait" (creates `crates/agent-sdk/src/micro_agent.rs`) -- the last module that must exist before this task can compile.
  - Transitively blocked by all Group 2 type definitions (`ToolCallRecord`, `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`) and Group 1 dependency additions (`async-trait`, `uuid`, `serde_json`).
- **Blocking:**
  - "Write object-safety and mock-implementation tests" (Group 4) -- tests import types through these re-exports.
  - "Write serialization tests for envelope types" (Group 4) -- tests import types through these re-exports.
  - "Run verification suite" (Group 4) -- depends on a compilable crate.
  - All future consumers of the `agent-sdk` crate that use the `MicroAgent` trait or envelope types.

## Risks & Edge Cases

1. **Missing module files at compile time.** If this task runs before all six `.rs` files exist, `rustc` will emit "file not found" errors. The dependency chain in the task breakdown prevents this, but if tasks are executed out of order, this will surface immediately as a compile error. Mitigation: strictly follow the task ordering.

2. **Struct/enum/trait visibility.** Each module's primary type must be declared `pub`. If any prerequisite task declares a type as `pub(crate)` or private, the `pub use` re-export will fail with a visibility error. The fix belongs in the module file, not in `lib.rs`.

3. **`async_trait` re-export path.** The re-export `pub use async_trait::async_trait;` requires that `async-trait` is listed in `Cargo.toml` dependencies. If the "Add `async-trait` dependency" task has not completed, this line will produce an "unresolved import" error.

4. **Name collisions.** None of the new type names (`ToolCallRecord`, `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `MicroAgent`) collide with standard library types, the existing four types, or common crate names. The re-exported `async_trait` name is a well-known macro name with no collision risk.

5. **Duplicate re-exports.** If a future task accidentally adds a second `pub use` for the same type, the compiler will warn about unused imports. This is not a risk for this task specifically, but should be kept in mind for future modifications.

6. **Edition 2024 behavior.** Rust edition 2024 does not change the semantics of `mod`, `pub use`, or proc-macro re-exports. No edition-specific risk.

## Verification

1. **`cargo check -p agent-sdk`** must succeed with no errors.
2. **`cargo clippy -p agent-sdk`** must produce no warnings.
3. **`cargo test -p agent-sdk`** must pass (no new tests are introduced by this task; existing tests must remain green).
4. **Manual inspection of `lib.rs`:** Confirm all ten `mod` declarations and ten `pub use` re-exports are present, plus the `async_trait` re-export. Confirm no `pub mod` is used. Confirm no commented-out code or debug statements remain.
5. **Import validation:** A downstream crate or integration test should be able to write all of the following successfully:
   ```rust
   use agent_sdk::MicroAgent;
   use agent_sdk::AgentRequest;
   use agent_sdk::AgentResponse;
   use agent_sdk::AgentError;
   use agent_sdk::HealthStatus;
   use agent_sdk::ToolCallRecord;
   use agent_sdk::async_trait;
   use agent_sdk::SkillManifest;  // existing, still works
   ```
6. **Object safety check:** The integration tests (Group 4) will verify that `Box<dyn MicroAgent>` compiles, but the re-exports themselves are a prerequisite for that test to even reference the types.
