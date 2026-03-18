# Spec: Add `escalate_to` field to `AgentResponse`

> From: .claude/tasks/issue-13.md

## Objective

Add an `escalate_to: Option<String>` field to `AgentResponse` so that when a ConstraintEnforcer (Group 3) detects low confidence, it can stamp the response with the name of the agent or entity to escalate to. This is the first prerequisite for runtime constraint enforcement: the response struct must be able to carry escalation metadata before the enforcer can populate it.

## Current State

`AgentResponse` in `crates/agent-sdk/src/agent_response.rs` has five fields:

```rust
pub struct AgentResponse {
    pub id: Uuid,
    pub output: Value,
    pub confidence: f32,
    pub escalated: bool,
    pub tool_calls: Vec<ToolCallRecord>,
}
```

The `success()` constructor initializes `escalated: false` and does not reference any escalation target.

The `Constraints` struct already carries `escalate_to: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`, which is the pattern to follow.

Two test files construct `AgentResponse` using struct literals (not the `success()` constructor):

- `crates/agent-sdk/tests/micro_agent_test.rs` (line 58-64): builds `AgentResponse` inside `MockAgent::invoke()`.
- `crates/agent-runtime/tests/http_test.rs` (line 69-75): builds `AgentResponse` inside `MockAgent::invoke()`.

Both will fail to compile after adding the new field unless they are updated.

## Requirements

1. Add `pub escalate_to: Option<String>` to the `AgentResponse` struct.
2. Annotate it with `#[serde(default, skip_serializing_if = "Option::is_none")]` so that:
   - Deserializing JSON that omits `escalate_to` produces `None` (backward-compatible reads).
   - Serializing a response where `escalate_to` is `None` omits the key entirely (backward-compatible writes).
3. The `AgentResponse::success()` constructor must initialize `escalate_to: None`.
4. The `AgentResponse` struct must continue to derive `JsonSchema` (the `Option<String>` field is natively supported by schemars).
5. All struct-literal constructions of `AgentResponse` in test files must include `escalate_to: None` to compile.
6. Existing tests must continue to pass with no behavioral change.
7. `cargo check`, `cargo clippy`, and `cargo test` must all succeed after the change.

## Implementation Details

### Files to modify

1. **`crates/agent-sdk/src/agent_response.rs`**
   - Add the field after `escalated`:
     ```rust
     #[serde(default, skip_serializing_if = "Option::is_none")]
     pub escalate_to: Option<String>,
     ```
   - Update `success()` to include `escalate_to: None`.

2. **`crates/agent-sdk/tests/micro_agent_test.rs`**
   - In the `MockAgent::invoke()` impl (around line 58-64), add `escalate_to: None` to the `AgentResponse` struct literal.

3. **`crates/agent-runtime/tests/http_test.rs`**
   - In the `MockAgent::invoke()` impl (around line 69-75), add `escalate_to: None` to the `AgentResponse` struct literal.

### No new types or functions

This task adds a single field and updates existing sites. No new modules, traits, or functions are introduced.

### Integration points

- The `ConstraintEnforcer` (Group 3, next task) will read `manifest.constraints.escalate_to` and write it into `response.escalate_to` when confidence is below threshold.
- The HTTP layer serializes `AgentResponse` via serde; the `skip_serializing_if` annotation ensures the new field is invisible to existing consumers unless populated.

## Dependencies

- Blocked by: none
- Blocking: "Implement ConstraintEnforcer struct with confidence and escalation checks"

## Risks & Edge Cases

- **Forgotten construction sites**: If any other file constructs `AgentResponse` via struct literal (not `success()`), it will fail to compile. A workspace-wide `cargo check` will catch this. The two known sites are listed above.
- **Schema generation**: Adding `Option<String>` to a `JsonSchema`-derived struct produces a nullable string in the generated schema. This is the correct representation and should not break any schema consumers.
- **Field ordering in JSON**: serde serializes fields in declaration order. Placing `escalate_to` immediately after `escalated` keeps related fields adjacent and produces a natural JSON layout. Existing consumers that parse by key name (not position) are unaffected.
- **f32 vs f64 mismatch**: Not directly relevant to this task, but worth noting that `AgentResponse.confidence` is `f32` while `Constraints.confidence_threshold` is `f64`. The ConstraintEnforcer (downstream) must cast appropriately. No action needed here.

## Verification

1. `cargo check --workspace` compiles with zero errors.
2. `cargo clippy --workspace` produces no warnings.
3. `cargo test --workspace` passes all existing tests with no regressions.
4. Manually confirm that serializing an `AgentResponse` with `escalate_to: None` omits the key from JSON output (can be verified with an assertion in an existing or ad-hoc test).
5. Manually confirm that deserializing JSON without an `escalate_to` key produces `AgentResponse { escalate_to: None, .. }`.
