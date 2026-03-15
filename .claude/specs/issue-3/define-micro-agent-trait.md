# Spec: Define `MicroAgent` trait

> From: .claude/tasks/issue-3.md

## Objective

Define the `MicroAgent` async trait in the `agent-sdk` crate. This is the core abstraction every agent implements, enabling the runtime and orchestrator to work with agents via `Box<dyn MicroAgent>`. The trait bridges the `SkillManifest` (which declares what an agent can do) with runtime behavior (invocation and health checking).

## Current State

The `agent-sdk` crate (`crates/agent-sdk/`) currently contains four modules, all of which are data-only structs with serde derives:

- `skill_manifest.rs` -- `SkillManifest` struct (name, version, description, model, preamble, tools, constraints, output)
- `model_config.rs` -- `ModelConfig` struct (provider, name, temperature)
- `constraints.rs` -- `Constraints` struct (max_turns, confidence_threshold, escalate_to, allowed_actions)
- `output_schema.rs` -- `OutputSchema` struct (format, schema)

All types are re-exported from `lib.rs`. The crate currently depends on `serde` (with `derive` feature) and `schemars` (with `derive` feature). There are no async dependencies, no trait definitions, and no runtime behavior yet.

The `Cargo.toml` uses `edition = "2024"` (Rust 1.85+). While Rust 1.75+ supports native `async fn` in traits, native async trait methods are **not** dyn-compatible (cannot be used with `Box<dyn Trait>`). Since the orchestrator requires `Box<dyn MicroAgent>`, the `async-trait` crate is necessary.

## Requirements

1. **File**: Create `crates/agent-sdk/src/micro_agent.rs` containing the `MicroAgent` trait definition.

2. **Trait supertraits**: The trait must require `Send + Sync` to guarantee object safety and allow trait objects to be shared across threads.

3. **`#[async_trait]` macro**: The trait must be annotated with `#[async_trait::async_trait]` (or `#[async_trait]` with the appropriate `use` statement) to make async methods dyn-compatible.

4. **Three methods** on the trait:
   - `fn manifest(&self) -> &SkillManifest` -- Synchronous. Returns a reference to the agent's manifest. This allows the runtime to introspect an agent's declared capabilities without any async overhead.
   - `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>` -- The primary entry point for executing an agent. Takes ownership of an `AgentRequest` and returns either a successful `AgentResponse` or an `AgentError`.
   - `async fn health(&self) -> HealthStatus` -- Returns the agent's current health. Used by the runtime for readiness/liveness checks.

5. **Imports**: All referenced types (`SkillManifest`, `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`) must be imported from sibling modules via `crate::` paths (e.g., `use crate::skill_manifest::SkillManifest;`).

6. **No derive macros on the trait itself**: Traits do not get `#[derive(...)]` annotations. The file should only contain the `use` imports, the `#[async_trait]` annotation, and the trait definition.

7. **Visibility**: The trait must be `pub` so it can be re-exported from `lib.rs`.

## Implementation Details

### File to create

**`crates/agent-sdk/src/micro_agent.rs`**

This file should contain:

- `use async_trait::async_trait;` import
- `use crate::skill_manifest::SkillManifest;` import
- `use crate::agent_request::AgentRequest;` import
- `use crate::agent_response::AgentResponse;` import
- `use crate::agent_error::AgentError;` import
- `use crate::health_status::HealthStatus;` import
- The trait definition with `#[async_trait]` annotation

### Trait signature

```rust
#[async_trait]
pub trait MicroAgent: Send + Sync {
    fn manifest(&self) -> &SkillManifest;
    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>;
    async fn health(&self) -> HealthStatus;
}
```

### Key design decisions

- **`&self` for all methods (not `&mut self`)**: Agents should be shareable. Interior mutability (e.g., `Arc<Mutex<...>>`) can be used by implementations that need mutable state. This is consistent with the `Send + Sync` requirement and allows multiple concurrent invocations.
- **`manifest` returns `&SkillManifest` (borrowed reference)**: The manifest is expected to be stored as a field in the implementing struct. Returning a reference avoids cloning on every call. This is a synchronous method because the manifest is always available in memory.
- **`invoke` takes `AgentRequest` by value**: The request is consumed by the invocation. Callers should not need the request after invoking. This avoids lifetime complexity.
- **`health` takes no parameters besides `&self`**: Health checking is a self-contained diagnostic. Implementations may check internal state, downstream service connectivity, or simply return `HealthStatus::Healthy`.
- **No default method implementations**: All three methods are required. There is no sensible default for `manifest` or `invoke`, and requiring `health` ensures all agents have explicit health reporting from the start.

### Integration points

- **`lib.rs`** (separate task): Will need `mod micro_agent;` and `pub use micro_agent::MicroAgent;` added. Also should re-export `async_trait::async_trait` for downstream convenience.
- **`Cargo.toml`** (separate task, Group 1): Must have `async-trait = "0.1"` in `[dependencies]` before this file can compile.
- **Group 2 types** (separate tasks): `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, and `ToolCallRecord` must exist in their respective modules before this file can compile.

## Dependencies

- **Blocked by**:
  - "Add `async-trait` dependency to `agent-sdk/Cargo.toml`" (Group 1) -- the `async_trait` macro must be available
  - "Add `uuid` and `serde_json` dependencies to `agent-sdk/Cargo.toml`" (Group 1) -- transitive, via Group 2 types
  - "Define `AgentRequest` struct" (Group 2) -- used in `invoke` signature
  - "Define `AgentResponse` struct" (Group 2) -- used in `invoke` return type
  - "Define `AgentError` enum" (Group 2) -- used in `invoke` return type
  - "Define `HealthStatus` enum" (Group 2) -- used in `health` return type
  - "Define `ToolCallRecord` struct" (Group 2) -- transitive, via `AgentResponse`

- **Blocking**:
  - "Update `lib.rs` with new module declarations and re-exports" (Group 3)
  - "Write object-safety and mock-implementation tests" (Group 4)

## Risks & Edge Cases

1. **`async-trait` vs native async traits**: Rust edition 2024 supports `async fn` in traits natively, but native async trait methods produce opaque return types that are **not** object-safe. If someone removes the `#[async_trait]` annotation thinking native support is sufficient, `Box<dyn MicroAgent>` will fail to compile. The rationale for `async-trait` should be documented in a code comment.

2. **`Send` bound on futures**: The `#[async_trait]` macro by default adds `Send` bounds to the returned futures (i.e., the generated return type is `Pin<Box<dyn Future<Output = T> + Send + '_>>`). This is the correct behavior for a multi-threaded runtime (tokio default). Do **not** use `#[async_trait(?Send)]`, which would remove the `Send` bound and prevent usage across thread boundaries.

3. **Object safety**: The `manifest` method returns `&SkillManifest`, which is object-safe. If this were changed to return a generic or `Self`-associated type, it would break `Box<dyn MicroAgent>`. All methods use `&self`, not `Self` by value, preserving object safety.

4. **Module naming**: The file is `micro_agent.rs` (snake_case), matching the Rust convention. The trait is `MicroAgent` (PascalCase). This is consistent with the existing pattern (`skill_manifest.rs` contains `SkillManifest`).

5. **No `#[cfg(test)]` module**: This file defines only a trait with no logic to unit-test in isolation. Integration tests (Group 4) will verify object safety and implementability via a mock.

## Verification

1. **Compiles**: After all blockers are resolved, `cargo check` passes with no errors on `crates/agent-sdk`.
2. **No clippy warnings**: `cargo clippy -p agent-sdk` produces no warnings.
3. **Object safety**: A test (Group 4) creates `Box<dyn MicroAgent>` from a mock implementation and calls all three methods through the trait object. This confirms dyn-compatibility.
4. **Implementability**: A test (Group 4) defines a `MockAgent` struct that implements `MicroAgent`, proving the trait is straightforward to implement.
5. **Method signatures match spec**: `manifest` returns `&SkillManifest`, `invoke` takes `AgentRequest` by value and returns `Result<AgentResponse, AgentError>`, `health` returns `HealthStatus`. These can be verified by inspection.
6. **Supertraits enforced**: A test can confirm that a type implementing `MicroAgent` satisfies `Send + Sync` bounds.
