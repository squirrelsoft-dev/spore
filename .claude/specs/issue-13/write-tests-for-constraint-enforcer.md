# Spec: Write tests for ConstraintEnforcer

> From: .claude/tasks/issue-13.md

## Objective

Create an integration test file that verifies the `ConstraintEnforcer` decorator correctly enforces confidence-threshold escalation, delegates `manifest()` and `health()` to the inner agent, and propagates errors without modification. These tests validate that the runtime constraint enforcement layer (Group 3) works as specified before the full verification suite runs.

## Current State

### ConstraintEnforcer (not yet implemented, specified in task breakdown)

`ConstraintEnforcer` will be a struct in `crates/agent-runtime/src/constraint_enforcer.rs` that:
- Wraps an `Arc<dyn MicroAgent>` as its inner agent.
- Implements `MicroAgent` itself (decorator pattern).
- Delegates `manifest()` and `health()` directly to the inner agent.
- In `invoke()`, calls the inner agent's `invoke()`, then performs a post-invocation check: if `(response.confidence as f64) < manifest.constraints.confidence_threshold`, it sets `response.escalated = true` and `response.escalate_to = manifest.constraints.escalate_to.clone()`.
- Low confidence produces a successful response with escalation metadata, not an error.
- The module will be registered in `crates/agent-runtime/src/lib.rs` as `pub mod constraint_enforcer;`.

### Key types involved

- **`AgentResponse`** (`crates/agent-sdk/src/agent_response.rs`): Has fields `id: Uuid`, `output: Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. After the "Add `escalate_to` field" task completes, it will also have `escalate_to: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- **`Constraints`** (`crates/agent-sdk/src/constraints.rs`): Has `max_turns: u32`, `confidence_threshold: f64`, `escalate_to: Option<String>`, `allowed_actions: Vec<String>`.
- **`MicroAgent` trait** (`crates/agent-sdk/src/micro_agent.rs`): `fn manifest(&self) -> &SkillManifest`, `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>`, `async fn health(&self) -> HealthStatus`.
- **`AgentError`** (`crates/agent-sdk/src/agent_error.rs`): Enum with `ToolCallFailed`, `ConfidenceTooLow`, `MaxTurnsExceeded`, `Internal` variants.

### Existing test patterns

The project uses a `MockAgent` pattern in integration tests:
- `crates/agent-sdk/tests/micro_agent_test.rs`: Defines a `MockAgent` struct with `manifest`, `should_fail`, `health_status` fields. Uses `make_manifest()` and `make_mock()` helpers. Implements `MicroAgent` via `#[async_trait]`. Returns a fixed `AgentResponse` with `confidence: 0.95` or an `AgentError::Internal` when `should_fail` is true.
- `crates/agent-runtime/tests/http_test.rs`: Similar `MockAgent` but uses an `ErrorMode` enum for more granular error control. Wraps the mock in `Arc<dyn MicroAgent>` for passing to the HTTP router.

Both test files use `#[tokio::test]` and import from `agent_sdk` via its public API.

### Crate dependencies

`crates/agent-runtime/Cargo.toml` already has `agent-sdk` as a dependency and `tokio` with `features = ["full"]`. No additional dependencies are needed for these tests.

## Requirements

1. **Confidence above threshold passes through unchanged**: When the inner agent returns a response with `confidence` >= `confidence_threshold`, the `ConstraintEnforcer` must return the response with `escalated: false` and `escalate_to: None`, and the `output`, `id`, `confidence`, and `tool_calls` fields must be identical to the inner agent's response.

2. **Confidence below threshold with `escalate_to` configured triggers escalation**: When the inner agent returns a response with `confidence` < `confidence_threshold` and `constraints.escalate_to` is `Some("fallback-agent")`, the response must have `escalated: true` and `escalate_to: Some("fallback-agent")`. The `output`, `id`, `confidence`, and `tool_calls` fields must remain unchanged (only escalation metadata is stamped).

3. **Confidence below threshold without `escalate_to` configured**: When the inner agent returns a response with `confidence` < `confidence_threshold` and `constraints.escalate_to` is `None`, the response must have `escalated: true` and `escalate_to: None`. This verifies escalation flagging works even when no target is specified.

4. **Manifest and health delegate correctly**: Calling `manifest()` on the `ConstraintEnforcer` must return the same `SkillManifest` as the inner agent. Calling `health()` must return the same `HealthStatus` as the inner agent (test with at least `Healthy` and `Degraded` variants).

5. **Error propagation**: When the inner agent returns an `Err(AgentError::Internal(...))`, the `ConstraintEnforcer` must propagate that exact error without modification. The confidence check must not run on error paths.

## Implementation Details

### File to create

**`crates/agent-runtime/tests/constraint_enforcer_test.rs`**

### MockAgent design

Define a `MockAgent` struct tailored for constraint enforcer testing. Unlike the SDK's mock (which uses a boolean `should_fail`) or the HTTP test mock (which uses `ErrorMode`), this mock needs a configurable `confidence` value on its responses so tests can set it above or below the threshold:

```rust
struct MockAgent {
    manifest: SkillManifest,
    response_confidence: f32,
    error_mode: Option<AgentError>,
    health_status: HealthStatus,
}
```

The `invoke()` implementation should:
- If `error_mode` is `Some(err)`, return `Err(err.clone())`.
- Otherwise, return `Ok(AgentResponse { id: request.id, output: json!({"result": "ok"}), confidence: self.response_confidence, escalated: false, escalate_to: None, tool_calls: vec![] })`.

### Helper functions

- `make_manifest_with_threshold(threshold: f64, escalate_to: Option<String>) -> SkillManifest`: Creates a `SkillManifest` with the given `confidence_threshold` and `escalate_to` in its constraints. Other fields use sensible defaults (name: `"test-agent"`, version: `"1.0.0"`, etc.), following the pattern from existing test helpers.
- `make_mock(confidence: f32, threshold: f64, escalate_to: Option<String>, error_mode: Option<AgentError>, health_status: HealthStatus) -> MockAgent`: Convenience constructor combining manifest creation and mock setup.

### Test cases

1. **`confidence_above_threshold_passes_through_unchanged`**
   - Create a mock with `response_confidence: 0.95`, `confidence_threshold: 0.85`, `escalate_to: Some("fallback-agent")`.
   - Wrap in `ConstraintEnforcer::new(Arc::new(mock))`.
   - Call `enforcer.invoke(AgentRequest::new("hello"))`.
   - Assert: `response.escalated == false`, `response.escalate_to == None`, `response.confidence == 0.95`, `response.output == json!({"result": "ok"})`, `response.id == request.id`.

2. **`confidence_below_threshold_triggers_escalation`**
   - Create a mock with `response_confidence: 0.50`, `confidence_threshold: 0.85`, `escalate_to: Some("fallback-agent")`.
   - Wrap in `ConstraintEnforcer`.
   - Call `enforcer.invoke(...)`.
   - Assert: `response.escalated == true`, `response.escalate_to == Some("fallback-agent")`, `response.confidence == 0.50` (unchanged), `response.output == json!({"result": "ok"})` (unchanged).

3. **`confidence_below_threshold_without_escalate_to`**
   - Create a mock with `response_confidence: 0.50`, `confidence_threshold: 0.85`, `escalate_to: None`.
   - Wrap in `ConstraintEnforcer`.
   - Call `enforcer.invoke(...)`.
   - Assert: `response.escalated == true`, `response.escalate_to == None`.

4. **`manifest_delegates_to_inner_agent`**
   - Create a mock and wrap in `ConstraintEnforcer`.
   - Call `enforcer.manifest()`.
   - Assert: `manifest.name == "test-agent"`, `manifest.version == "1.0.0"`, `manifest.constraints.confidence_threshold == 0.85`.

5. **`health_delegates_to_inner_agent`**
   - Create a mock with `health_status: HealthStatus::Healthy`, wrap in `ConstraintEnforcer`, assert `health().await == HealthStatus::Healthy`.
   - Create a mock with `health_status: HealthStatus::Degraded("slow")`, wrap in `ConstraintEnforcer`, assert `health().await == HealthStatus::Degraded("slow")`.

6. **`inner_agent_error_propagates_unchanged`**
   - Create a mock with `error_mode: Some(AgentError::Internal("mock failure"))`.
   - Wrap in `ConstraintEnforcer`.
   - Call `enforcer.invoke(...)`.
   - Assert: result is `Err(AgentError::Internal("mock failure"))`.

### Imports

```rust
use std::collections::HashMap;
use std::sync::Arc;

use agent_sdk::{
    async_trait, AgentError, AgentRequest, AgentResponse, Constraints, HealthStatus, MicroAgent,
    ModelConfig, OutputSchema, SkillManifest,
};
use serde_json::json;

use agent_runtime::constraint_enforcer::ConstraintEnforcer;
```

### Edge case: confidence exactly equal to threshold

The spec from the task breakdown says `if (response.confidence as f64) < manifest.constraints.confidence_threshold` (strict less-than). When confidence equals the threshold exactly, it should NOT trigger escalation. Consider adding a test for `confidence: 0.85` with `threshold: 0.85` asserting `escalated: false`. This can be included as part of test case 1 or as a separate test.

### Type casting note

`AgentResponse.confidence` is `f32` while `Constraints.confidence_threshold` is `f64`. The `ConstraintEnforcer` casts `response.confidence as f64` before comparing. Tests should exercise values where f32-to-f64 casting does not introduce false positives (use clean values like 0.5, 0.85, 0.95).

## Dependencies

- Blocked by: "Wire ConstraintEnforcer into main.rs" (the `ConstraintEnforcer` struct and its `pub mod` registration must exist for the test file to compile)
- Blocking: "Run verification suite"

## Risks & Edge Cases

- **`escalate_to` field not yet on `AgentResponse`**: This test file depends on the "Add `escalate_to` field to `AgentResponse`" task completing first. If that field is missing, the mock's `invoke()` and the test assertions will not compile. Since "Wire ConstraintEnforcer into main.rs" already depends on that task transitively, the blocking chain is correct.
- **`ConstraintEnforcer` API changes**: The tests assume `ConstraintEnforcer::new(Arc<dyn MicroAgent>)` as the constructor. If the implementation uses a different signature (e.g., taking `Box<dyn MicroAgent>` or separate constraint parameters), the test construction will need adjustment. The task breakdown specifies `Arc<dyn MicroAgent>`.
- **f32 precision in assertions**: Use `(value - expected).abs() < f32::EPSILON` for floating-point comparisons on confidence, following the pattern in `micro_agent_test.rs` (line 114).
- **Module visibility**: The test imports `agent_runtime::constraint_enforcer::ConstraintEnforcer`. This requires `constraint_enforcer` to be a `pub mod` in `crates/agent-runtime/src/lib.rs` and `ConstraintEnforcer` to be `pub`. The task breakdown specifies both.
- **`AgentError` clone**: The mock stores an `Option<AgentError>` and must clone it on each `invoke()` call. `AgentError` already derives `Clone`, so this is safe.

## Verification

1. `cargo test --package agent-runtime --test constraint_enforcer_test` runs all six test cases and they pass.
2. `cargo check --workspace` compiles with zero errors.
3. `cargo clippy --workspace` produces no warnings.
4. `cargo test --workspace` shows no regressions in existing tests.
5. Each test case maps 1:1 to the five requirements listed in the task breakdown (with the boundary case as a bonus).
