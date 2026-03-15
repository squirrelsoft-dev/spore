# Spec: Write object-safety and mock-implementation tests

> From: .claude/tasks/issue-3.md

## Objective

Create integration tests that validate the `MicroAgent` trait is correctly defined, implementable by downstream types, and dyn-compatible (object-safe). These tests serve as the primary proof that the trait contract works end-to-end: a concrete struct can implement it, the implementation can be boxed into `Box<dyn MicroAgent>`, and all three trait methods (`manifest`, `invoke`, `health`) behave correctly through the trait object. This ensures the orchestrator's core abstraction -- dispatching work to agents via dynamic dispatch -- is sound.

## Current State

The `agent-sdk` crate currently defines only synchronous config types (`SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`) with serde derives. There are no async traits, no agent abstractions, and no async test infrastructure.

By the time this task executes, the following will exist (defined by earlier tasks in Groups 1-3):

- **Dependencies**: `async-trait` (runtime dep), `tokio` (dev dep), `uuid`, `serde_json`
- **Types**: `AgentRequest`, `AgentResponse`, `AgentError`, `HealthStatus`, `ToolCallRecord`
- **Trait**: `MicroAgent` with `#[async_trait]`, requiring `Send + Sync`, exposing:
  - `fn manifest(&self) -> &SkillManifest` (sync)
  - `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`
  - `async fn health(&self) -> HealthStatus`
- **Re-exports**: All of the above available via `use agent_sdk::*` from `lib.rs`

The existing test file `crates/agent-sdk/tests/skill_manifest_test.rs` establishes the integration test pattern: import types from `agent_sdk`, construct values inline, and assert with standard `assert_eq!`/`assert!` macros.

## Requirements

- Create the file `crates/agent-sdk/tests/micro_agent_test.rs`.
- Define a `MockAgent` struct inside the test file that implements `MicroAgent`. The mock must:
  - Hold a `SkillManifest` field and return a reference to it from `manifest()`.
  - Accept an `AgentRequest` in `invoke()` and return a valid `AgentResponse` (with a deterministic `id`, `output`, `confidence`, `escalated`, and empty `tool_calls`).
  - Return `HealthStatus::Healthy` from `health()` by default.
  - Be configurable (or have a second mock variant) to return `Err` variants from `invoke` and non-`Healthy` statuses from `health`.
- Write the following test functions:
  1. **`mock_agent_implements_trait`** (`#[tokio::test]`): Construct a `MockAgent`, call `manifest()`, `invoke()`, and `health()` directly, and assert the returned values are correct. This proves the trait is implementable.
  2. **`trait_object_is_dyn_compatible`** (`#[tokio::test]`): Box a `MockAgent` as `Box<dyn MicroAgent>`, call all three trait methods through the trait object, and assert correctness. This proves object safety.
  3. **`invoke_returns_ok`** (`#[tokio::test]`): Call `invoke()` on a mock agent (or trait object) and assert the `Ok` variant contains the expected `AgentResponse` fields (`output`, `confidence`, `escalated`, `tool_calls`).
  4. **`invoke_returns_err`** (`#[tokio::test]`): Configure the mock to return an error, call `invoke()`, and assert the `Err` variant matches the expected `AgentError` variant (test at least one variant, e.g., `AgentError::Internal`).
  5. **`health_status_healthy`** (`#[tokio::test]`): Assert `health()` returns `HealthStatus::Healthy`.
  6. **`health_status_degraded`** (`#[tokio::test]`): Configure the mock to return `HealthStatus::Degraded(reason)`, assert the variant and the contained reason string.
  7. **`health_status_unhealthy`** (`#[tokio::test]`): Configure the mock to return `HealthStatus::Unhealthy(reason)`, assert the variant and the contained reason string.
- All tests must use `#[tokio::test]` since `invoke` and `health` are async.
- Follow the import style from `skill_manifest_test.rs`: import types from `agent_sdk` (the crate name), not from internal modules.
- No external test helper crates (e.g., `mockall`) should be used. The `MockAgent` is hand-written.
- The test file must compile and pass with `cargo test -p agent-sdk`.

## Implementation Details

**File to create:** `crates/agent-sdk/tests/micro_agent_test.rs`

### MockAgent struct

```rust
use std::collections::HashMap;
use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, HealthStatus,
    MicroAgent, SkillManifest, ModelConfig, Constraints, OutputSchema,
};

struct MockAgent {
    manifest: SkillManifest,
    should_fail: bool,
    health_status: HealthStatus,
}
```

- `should_fail`: when `true`, `invoke()` returns `Err(AgentError::Internal(...))`.
- `health_status`: the value returned by `health()`.
- A helper function (e.g., `fn make_manifest() -> SkillManifest`) constructs a reusable test manifest to avoid duplication across tests, following the inline-construction pattern from `skill_manifest_test.rs`.
- A helper function (e.g., `fn make_mock(should_fail: bool, health: HealthStatus) -> MockAgent`) constructs a `MockAgent` with the given configuration.

### MicroAgent implementation

```rust
#[async_trait]
impl MicroAgent for MockAgent {
    fn manifest(&self) -> &SkillManifest { &self.manifest }

    async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
        if self.should_fail {
            return Err(AgentError::Internal("mock failure".to_string()));
        }
        Ok(AgentResponse { ... })
    }

    async fn health(&self) -> HealthStatus {
        self.health_status.clone()
    }
}
```

### Key design decisions

- The `MockAgent` uses a `should_fail` bool rather than separate mock structs to keep the test file concise while still covering both `Ok` and `Err` paths.
- The `health_status` field is cloned on return, which is valid because `HealthStatus` derives `Clone`.
- `AgentResponse` fields in the `Ok` path should use deterministic values (e.g., `confidence: 0.95`, `escalated: false`, `output: serde_json::json!({"result": "ok"})`, `tool_calls: vec![]`) so assertions are straightforward.
- The `AgentResponse.id` should match the `AgentRequest.id` to demonstrate request-response correlation.

### Test functions (7 total)

Each test function follows this pattern:
1. Construct a `MockAgent` with the appropriate configuration.
2. Call the trait method (directly or via `Box<dyn MicroAgent>`).
3. Assert the result with `assert_eq!`, `assert!`, or pattern matching.

The **`trait_object_is_dyn_compatible`** test is the most critical: it must construct `let agent: Box<dyn MicroAgent> = Box::new(mock);` and then call `agent.manifest()`, `agent.invoke(req).await`, and `agent.health().await` through the trait object. If this compiles, the trait is object-safe.

### Integration points

- Imports all public types from `agent_sdk` crate root (relies on the `lib.rs` re-exports task completing first).
- Uses `#[async_trait]` re-exported from `agent_sdk` (so the test does not need its own `async-trait` dependency).
- Uses `#[tokio::test]` from the `tokio` dev-dependency added in Group 1.
- Uses `serde_json::json!()` macro for constructing `AgentResponse.output` values (available via the `serde_json` dependency added in Group 1).

## Dependencies

- **Blocked by**:
  - "Update `lib.rs` with new module declarations and re-exports" (Group 3) -- all types and the trait must be importable from `agent_sdk`.
  - Transitively blocked by all Group 1 and Group 2 tasks (dependencies, type definitions, trait definition).
- **Blocking**:
  - "Run verification suite" (Group 4) -- `cargo test` must pass, which includes these tests.

## Risks & Edge Cases

- **`async_trait` re-export missing**: If `lib.rs` does not re-export `async_trait::async_trait`, the test file will need to add `async-trait` as a dev-dependency or use a different import path. The spec for `lib.rs` re-exports explicitly includes this re-export, but if it is missed, the test will fail to compile. Mitigation: the test file should import `async_trait` from `agent_sdk` and the compilation failure will immediately surface the missing re-export.
- **`Send + Sync` bounds on `MockAgent`**: The `MicroAgent` trait requires `Send + Sync`. `MockAgent` must be `Send + Sync`-compatible, meaning its fields must all be `Send + Sync`. `SkillManifest` (all `String`/`f64`/`Vec` fields), `bool`, and `HealthStatus` (enum of `String` variants) are all `Send + Sync`. No risk here.
- **`HealthStatus::Clone`**: The `health()` method returns `HealthStatus` by value, so the mock needs to clone its stored `health_status`. The spec for `HealthStatus` includes `Clone` in the derives. If `Clone` is missing, the mock will not compile. Mitigation: this will surface immediately as a compile error.
- **`serde_json` availability in tests**: The test uses `serde_json::json!()` to construct `Value` instances. Since `serde_json` is a regular dependency (not dev-only), it is available in integration tests. No risk.
- **Test isolation**: Each test constructs its own `MockAgent` instance. There is no shared mutable state. No concurrency or ordering concerns.

## Verification

1. `cargo check -p agent-sdk --tests` -- the test file compiles without errors, confirming the trait is implementable and object-safe.
2. `cargo test -p agent-sdk --test micro_agent_test` -- all 7 tests pass.
3. `cargo clippy -p agent-sdk --tests` -- no warnings in the test file.
4. Manual review: confirm the `trait_object_is_dyn_compatible` test constructs a `Box<dyn MicroAgent>` and calls all three methods through it. If the file compiles, object safety is proven.
5. Manual review: confirm the test file does not import from internal module paths (e.g., `agent_sdk::micro_agent::MicroAgent`) -- all imports must go through the crate root.
