# Spec: Add structured tracing to escalation path

> From: .claude/tasks/issue-17.md

## Objective

Add structured `tracing` instrumentation to the escalation logic in `orchestrator.rs` so that escalation events, warnings, and errors are observable at runtime. Currently, escalation decisions happen silently -- a request can chain through multiple agents, hit cycle detection, exceed depth limits, or silently return when `escalate_to` is `None`, all without emitting a single log line. This task makes escalation fully observable by following the structured tracing pattern already established in `semantic_router.rs`.

## Current State

### Escalation functions in `crates/orchestrator/src/orchestrator.rs`

- **`handle_escalation()`** (line 180): Loops through escalation hops. When `escalated` is `true` and `escalate_to` is `Some(name)`, it validates depth/cycle, looks up the target, builds a new request, and invokes. When `escalated` is `false`, it returns silently. When `escalated` is `true` but `escalate_to` is `None`, it also returns silently -- this is a suspicious state that should be warned about.

- **`validate_escalation_depth()`** (line 215): Returns `OrchestratorError::EscalationFailed` when `chain.len() >= MAX_ESCALATION_DEPTH` (5). No log is emitted before the error.

- **`validate_no_cycle()`** (line 231): Returns `OrchestratorError::EscalationFailed` when the target agent is already present in the chain. No log is emitted before the error.

- **`lookup_escalation_target()`** (line 245): Returns `OrchestratorError::EscalationFailed` when the target is not in the registry. No log is emitted.

### Existing tracing pattern in `crates/orchestrator/src/semantic_router.rs`

```rust
tracing::debug!(agent = %agent_name, "routed via intent match");
tracing::debug!(agent = %agent_name, "routed via semantic similarity");
```

The crate already depends on `tracing = "0.1"` in `crates/orchestrator/Cargo.toml` (line 15).

### `AgentResponse` fields available for structured logging

- `confidence: f32`
- `escalated: bool`
- `escalate_to: Option<String>`

## Requirements

1. **`tracing::info!` on each escalation hop** -- Inside the `handle_escalation()` loop, when an escalation is about to be dispatched, emit a structured `info` event with fields: `source_agent` (last agent in chain), `target_agent` (the escalation target), `confidence` (from the current response), `depth` (current chain length), and `chain` (the full escalation chain formatted as a debug string, e.g. `"[a -> b -> c]"`).

2. **`tracing::warn!` on escalated-with-no-target** -- When `response.escalated == true` but `response.escalate_to` is `None`, emit a structured `warn` event with fields: `source_agent` (last agent in chain), `confidence`, and `chain`. The message should indicate the agent signaled escalation but provided no target.

3. **`tracing::error!` on depth exceeded** -- In `validate_escalation_depth()`, before returning the error, emit a structured `error` event with fields: `depth` (current chain length), `max_depth` (the `MAX_ESCALATION_DEPTH` constant), and `chain`.

4. **`tracing::error!` on cycle detection** -- In `validate_no_cycle()`, before returning the error, emit a structured `error` event with fields: `target_agent`, `chain`.

5. **No new dependencies** -- The `tracing` crate is already a dependency. No additional crates are needed.

6. **Follow the existing pattern** -- Use `%` for Display formatting of string fields (e.g., `agent = %agent_name`), `?` for Debug formatting of collections (e.g., `chain = ?chain`). Use bare string literals for the event message.

## Implementation Details

### File to modify

**`crates/orchestrator/src/orchestrator.rs`**

#### Changes to `handle_escalation()`

- After the `if !current_response.escalated` early return (line 190-192), no change needed (this is the normal non-escalation path).

- In the `None` arm of `current_response.escalate_to` match (line 195-197), before `return Ok(current_response)`, add:

  ```rust
  tracing::warn!(
      source_agent = %current_chain.last().unwrap_or(&"unknown".to_string()),
      confidence = current_response.confidence,
      chain = ?current_chain,
      "agent signaled escalation but provided no target"
  );
  ```

- After `build_escalation_request()` and before `try_invoke()` (between lines 208 and 210), add:

  ```rust
  tracing::info!(
      source_agent = %current_chain.last().unwrap_or(&"unknown".to_string()),
      target_agent = %target_name,
      confidence = current_response.confidence,
      depth = current_chain.len(),
      chain = ?current_chain,
      "escalating request to next agent"
  );
  ```

#### Changes to `validate_escalation_depth()`

- Before the `return Err(...)` on line 220, add:

  ```rust
  tracing::error!(
      depth = chain.len(),
      max_depth = MAX_ESCALATION_DEPTH,
      chain = ?chain,
      "escalation depth exceeded"
  );
  ```

#### Changes to `validate_no_cycle()`

- Before the `return Err(...)` on line 237, add:

  ```rust
  tracing::error!(
      target_agent = %target_name,
      chain = ?chain,
      "escalation cycle detected"
  );
  ```

### No changes to other files

- `Cargo.toml` already has `tracing = "0.1"`.
- `semantic_router.rs` is untouched; it serves only as the reference pattern.
- No new modules, structs, or traits are introduced.

## Dependencies

- Blocked by: none
- Blocking: "Add cycle detection test", "Add missing escalation target test", "Add escalated-with-no-target test", "Add successful multi-hop escalation chain test", "Add escalation-via-semantic-routing test"

## Risks & Edge Cases

- **`unwrap_or` on empty chain**: `current_chain.last()` could theoretically be `None` if the chain is empty. The implementation uses `unwrap_or` with a fallback `"unknown"` string to avoid panics. In practice, the chain always has at least one entry by the time `handle_escalation` is called (populated in `dispatch()` and `dispatch_with_model()`), but the defensive fallback is still warranted.

- **Performance**: `tracing` macros are zero-cost when no subscriber is attached. Structured field formatting only runs when a subscriber is active and the level is enabled. There is no measurable overhead in production unless a subscriber collects these events.

- **Chain formatting**: Using `?chain` (Debug format) for `Vec<String>` produces `["a", "b", "c"]` output, which is adequate for log consumers. If a more opinionated format like `"a -> b -> c"` is preferred, a small formatting helper would be needed, but Debug format is consistent with how the error messages already format chains and avoids adding code.

## Verification

1. `cargo check -p orchestrator` passes with no errors or warnings.
2. `cargo clippy -p orchestrator` passes with no warnings.
3. `cargo test -p orchestrator` passes (existing tests remain green).
4. Manual review confirms:
   - Each `tracing` call uses structured fields, not string interpolation in the message.
   - Log levels match the semantics: `info` for normal escalation flow, `warn` for the suspicious no-target case, `error` for validation failures.
   - The field names (`source_agent`, `target_agent`, `confidence`, `depth`, `chain`, `max_depth`) are consistent and machine-parseable.
5. No new dependencies were added.
