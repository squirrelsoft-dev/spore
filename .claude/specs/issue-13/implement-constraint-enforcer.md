# Spec: Implement ConstraintEnforcer struct with confidence and escalation checks

> From: .claude/tasks/issue-13.md

## Objective

Create a decorator (wrapper) struct `ConstraintEnforcer` that sits between the HTTP layer and the inner `RuntimeAgent`, intercepting every `invoke()` response to enforce the confidence-threshold constraint declared in the agent's skill manifest. When the agent's self-reported confidence falls below the threshold, the enforcer marks the response as escalated (with optional target) rather than returning an error. This keeps low-confidence results visible to the caller while signaling that a more capable agent should handle the request.

## Current State

### MicroAgent trait (`crates/agent-sdk/src/micro_agent.rs`)

The `MicroAgent` trait is async-trait-based and dyn-compatible. It has three methods:

- `fn manifest(&self) -> &SkillManifest` -- returns the agent's manifest by reference
- `async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError>` -- processes a request
- `async fn health(&self) -> HealthStatus` -- returns current health

### AgentResponse (`crates/agent-sdk/src/agent_response.rs`)

Currently has fields: `id: Uuid`, `output: Value`, `confidence: f32`, `escalated: bool`, `tool_calls: Vec<ToolCallRecord>`. The `success()` constructor sets `confidence: 1.0` and `escalated: false`.

**Note:** This task is blocked by the addition of `escalate_to: Option<String>` to `AgentResponse`. The spec assumes that field will exist when implementation begins.

### Constraints (`crates/agent-sdk/src/constraints.rs`)

```rust
pub struct Constraints {
    pub max_turns: u32,
    pub confidence_threshold: f64,     // f64, not f32
    pub escalate_to: Option<String>,   // target agent name
    pub allowed_actions: Vec<String>,
}
```

Key detail: `confidence_threshold` is `f64` while `AgentResponse.confidence` is `f32`. The comparison requires casting `response.confidence as f64`.

### SkillManifest (`crates/agent-sdk/src/skill_manifest.rs`)

Contains a `constraints: Constraints` field, accessible via `manifest().constraints`.

### RuntimeAgent (`crates/agent-runtime/src/runtime_agent.rs`)

The existing `MicroAgent` implementor. It holds a `SkillManifest`, a `BuiltAgent` (LLM backend), and a `ToolRegistry`. Its `manifest()` returns `&self.manifest` by reference.

### Current module registration (`crates/agent-runtime/src/lib.rs`)

```rust
pub mod config;
pub mod http;
pub mod provider;
pub mod runtime_agent;
pub mod tool_bridge;
```

### Wiring in main.rs (`crates/agent-runtime/src/main.rs`)

Currently wraps `RuntimeAgent` directly into `Arc<dyn MicroAgent>`:
```rust
let runtime_agent = RuntimeAgent::new(manifest, agent, registry.clone());
let micro_agent: Arc<dyn MicroAgent> = Arc::new(runtime_agent);
```
The future "Wire ConstraintEnforcer into main.rs" task will insert the enforcer between these two lines.

## Requirements

1. **Create `ConstraintEnforcer` struct** in a new file `crates/agent-runtime/src/constraint_enforcer.rs` that wraps an `Arc<dyn MicroAgent>`.

2. **Implement `MicroAgent` for `ConstraintEnforcer`** using `#[async_trait]`:
   - `manifest()` delegates to `self.inner.manifest()`.
   - `health()` delegates to `self.inner.health().await`.
   - `invoke()` delegates to `self.inner.invoke(request).await`, then performs a post-invocation confidence check on the `Ok` response.

3. **Confidence check logic in `invoke()`**: After receiving a successful response from the inner agent:
   - Read `self.inner.manifest().constraints.confidence_threshold` (f64).
   - Cast `response.confidence` (f32) to f64 for comparison.
   - If `(response.confidence as f64) < confidence_threshold`:
     - Set `response.escalated = true`.
     - Set `response.escalate_to = manifest.constraints.escalate_to.clone()`.
   - Return the (possibly mutated) response as `Ok(response)`. This is not an error path.

4. **Error passthrough**: If the inner agent returns `Err(...)`, the enforcer must propagate it unchanged without any confidence check.

5. **Constructor**: Provide `ConstraintEnforcer::new(inner: Arc<dyn MicroAgent>) -> Self`.

6. **Module registration**: Add `pub mod constraint_enforcer;` to `crates/agent-runtime/src/lib.rs`.

7. **No new dependencies**: The implementation uses only `std::sync::Arc`, `agent_sdk` types, and `async_trait` -- all already available in `agent-runtime`.

## Implementation Details

### Files to create

**`crates/agent-runtime/src/constraint_enforcer.rs`**

- Define `ConstraintEnforcer` with a single field `inner: Arc<dyn MicroAgent>`.
- Implement `ConstraintEnforcer::new(inner: Arc<dyn MicroAgent>) -> Self`.
- Implement `MicroAgent` for `ConstraintEnforcer` via `#[async_trait]`:
  - `manifest()` returns `self.inner.manifest()` (direct delegation, same lifetime).
  - `health()` returns `self.inner.health().await` (direct delegation).
  - `invoke()`:
    1. Call `let mut response = self.inner.invoke(request).await?;` -- the `?` propagates errors immediately.
    2. Read `let manifest = self.inner.manifest();`
    3. Read `let threshold = manifest.constraints.confidence_threshold;`
    4. Compare: `if (response.confidence as f64) < threshold`
    5. If true: set `response.escalated = true;` and `response.escalate_to = manifest.constraints.escalate_to.clone();`
    6. Return `Ok(response)`.

### Files to modify

**`crates/agent-runtime/src/lib.rs`**

- Add `pub mod constraint_enforcer;` to the module list.

### Key types and interfaces

| Type | Role |
|------|------|
| `ConstraintEnforcer` | Decorator struct wrapping `Arc<dyn MicroAgent>` |
| `ConstraintEnforcer::new()` | Constructor taking the inner agent |
| `MicroAgent` impl | Delegates manifest/health, intercepts invoke for confidence check |

### Integration points

- **Downstream (blocked by)**: Requires `AgentResponse.escalate_to: Option<String>` to exist.
- **Upstream (blocking)**: The "Wire ConstraintEnforcer into main.rs" task will wrap `RuntimeAgent` with this enforcer before passing to the HTTP layer.
- **The decorator pattern** preserves the `Arc<dyn MicroAgent>` contract so no changes are needed to `http.rs`, `AppState`, or any handler code.

## Dependencies

- **Blocked by**: "Add `escalate_to` field to `AgentResponse`" -- the `response.escalate_to` field must exist before this code can compile.
- **Blocking**: "Wire ConstraintEnforcer into main.rs" -- that task inserts `ConstraintEnforcer::new(Arc::new(runtime_agent))` between agent construction and HTTP server startup.

## Risks & Edge Cases

1. **Type mismatch (f32 vs f64)**: `confidence` is `f32`, `confidence_threshold` is `f64`. The cast `response.confidence as f64` is lossless (f32 fits in f64) but could introduce subtle floating-point comparison issues. For example, `0.85_f32 as f64` becomes `0.8500000238418579_f64`, which is strictly greater than `0.85_f64`. This means a response with `confidence: 0.85_f32` would NOT be escalated against a `confidence_threshold: 0.85_f64`. This is arguably correct (threshold is "below", not "at or below"), but the implementer should be aware of it. The task description specifies strict `<` comparison, so follow that.

2. **`manifest()` lifetime**: `ConstraintEnforcer::manifest()` returns `&SkillManifest`. Since it delegates to `self.inner.manifest()`, the returned reference borrows from `self.inner` (which is behind `Arc`), so the lifetime is tied to `&self`. This works correctly with the trait signature `fn manifest(&self) -> &SkillManifest`.

3. **Confidence at exactly the threshold**: Per the `<` comparison, a response with `confidence` exactly equal to the threshold (after casting) is NOT escalated. This matches the task description.

4. **`escalate_to` is `None` in constraints**: If the manifest has `escalate_to: None`, the enforcer will still set `response.escalated = true` but `response.escalate_to` will be `None`. This is valid -- the caller sees "escalation needed" but must decide the target. The downstream test suite explicitly covers this case.

5. **Thread safety**: `ConstraintEnforcer` holds `Arc<dyn MicroAgent>` where `MicroAgent: Send + Sync`. The struct itself derives neither, but `Arc<dyn MicroAgent>` is `Send + Sync`, so `ConstraintEnforcer` is automatically `Send + Sync`. The `#[async_trait]` macro requires this for `MicroAgent`.

6. **No mutation of inner agent**: The enforcer only mutates the response, never the inner agent's state. This is a pure post-processing decorator.

## Verification

1. **Compiles**: `cargo check -p agent-runtime` succeeds with no errors.
2. **No warnings**: `cargo clippy -p agent-runtime` produces no warnings.
3. **Unit tests pass**: `cargo test -p agent-runtime` passes (existing tests unaffected).
4. **Full workspace**: `cargo test` passes across all crates.
5. **Module is public**: `constraint_enforcer` appears in `crates/agent-runtime/src/lib.rs` and `ConstraintEnforcer` is importable as `agent_runtime::constraint_enforcer::ConstraintEnforcer`.
6. **Manual review**: Confirm that `invoke()` returns `Ok(response)` with `escalated: true` when confidence is below threshold (not `Err`). Confirm that errors from the inner agent pass through without modification.
7. **Function size**: All functions remain under 50 lines per project rules.
