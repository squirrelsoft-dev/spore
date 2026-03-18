# Spec: Map rig-core MaxTurnsError to AgentError::MaxTurnsExceeded

> From: .claude/tasks/issue-13.md

## Objective

Translate rig-core's `PromptError::MaxTurnsError` into the domain-specific `AgentError::MaxTurnsExceeded { turns }` so that HTTP consumers receive a semantically meaningful 422 response (via the existing `AppError` mapping) instead of a generic 500 Internal Server Error. This requires improving the error pipeline in `BuiltAgent::prompt()` and `RuntimeAgent::invoke()` to distinguish max-turns exhaustion from other prompt failures.

## Current State

### `crates/agent-runtime/src/provider.rs` -- `ProviderError` and `BuiltAgent::prompt()`

`ProviderError` is an enum with four variants:

```rust
pub enum ProviderError {
    UnsupportedProvider { provider: String },
    MissingApiKey { provider: String, env_var: String },
    ClientBuild(String),
    Prompt(String),
}
```

`BuiltAgent::prompt()` calls rig-core's `Agent::prompt()` (which returns `Result<String, PromptError>`) and maps all errors uniformly to `ProviderError::Prompt(e.to_string())`:

```rust
pub async fn prompt(&self, input: &str) -> Result<String, ProviderError> {
    match self {
        Self::OpenAi(agent) => agent
            .prompt(input)
            .await
            .map_err(|e| ProviderError::Prompt(e.to_string())),
        Self::Anthropic(agent) => agent
            .prompt(input)
            .await
            .map_err(|e| ProviderError::Prompt(e.to_string())),
    }
}
```

This erases the typed `PromptError` enum, making it impossible for callers to distinguish `MaxTurnsError` from other prompt failures without inspecting the stringified message.

### Rig-core `PromptError` (v0.32)

In `rig-core-0.32.0/src/completion/request.rs`:

```rust
#[derive(Debug, Error)]
pub enum PromptError {
    #[error("CompletionError: {0}")]
    CompletionError(#[from] CompletionError),
    #[error("ToolCallError: {0}")]
    ToolError(#[from] ToolSetError),
    #[error("ToolServerError: {0}")]
    ToolServerError(#[from] ToolServerError),
    #[error("MaxTurnError: (reached max turn limit: {max_turns})")]
    MaxTurnsError {
        max_turns: usize,
        chat_history: Box<Vec<Message>>,
        prompt: Box<Message>,
    },
    #[error("PromptCancelled: {reason}")]
    PromptCancelled {
        chat_history: Box<Vec<Message>>,
        reason: String,
    },
}
```

The `#[error]` attribute produces the display string `"MaxTurnError: (reached max turn limit: N)"`. Note: the display text uses `MaxTurnError` (singular), not `MaxTurnsError`.

### `crates/agent-runtime/src/runtime_agent.rs` -- `RuntimeAgent::invoke()`

```rust
async fn invoke(&self, request: AgentRequest) -> Result<AgentResponse, AgentError> {
    let output = self
        .agent
        .prompt(&request.input)
        .await
        .map_err(|e| AgentError::Internal(e.to_string()))?;
    Ok(AgentResponse::success(request.id, Value::String(output)))
}
```

All `ProviderError` variants are collapsed into `AgentError::Internal(String)`, losing the ability to surface `MaxTurnsExceeded` as a distinct error to the HTTP layer.

### `crates/agent-sdk/src/agent_error.rs` -- `AgentError`

```rust
pub enum AgentError {
    ToolCallFailed { tool: String, reason: String },
    ConfidenceTooLow { confidence: f32, threshold: f32 },
    MaxTurnsExceeded { turns: u32 },
    Internal(String),
}
```

The `MaxTurnsExceeded` variant already exists and is mapped to HTTP 422 in `crates/agent-runtime/src/http.rs`.

### `crates/agent-sdk/src/constraints.rs` -- `Constraints`

```rust
pub struct Constraints {
    pub max_turns: u32,
    // ...
}
```

`max_turns` is a `u32`, matching the `turns` field on `AgentError::MaxTurnsExceeded`.

## Requirements

1. `ProviderError` must gain a new variant `MaxTurnsExceeded { max_turns: u32 }` to carry typed max-turns errors out of `BuiltAgent::prompt()` without relying on string matching.
2. `BuiltAgent::prompt()` must pattern-match on `PromptError::MaxTurnsError` and map it to `ProviderError::MaxTurnsExceeded { max_turns: e.max_turns as u32 }`. All other `PromptError` variants continue to map to `ProviderError::Prompt(e.to_string())`.
3. `RuntimeAgent::invoke()` must match `ProviderError::MaxTurnsExceeded` and produce `AgentError::MaxTurnsExceeded { turns: manifest.constraints.max_turns }` instead of `AgentError::Internal(...)`.
4. All other `ProviderError` variants must continue to map to `AgentError::Internal(e.to_string())` in `invoke()`.
5. `ProviderError::MaxTurnsExceeded` must implement `Display` with a message like `"max turns exceeded: {max_turns} turns"`.
6. No new crate dependencies may be added.
7. `cargo check --workspace`, `cargo clippy --workspace`, and `cargo test --workspace` must all pass.

## Implementation Details

### Files to modify

1. **`crates/agent-runtime/src/provider.rs`**

   - **Add `MaxTurnsExceeded` variant to `ProviderError`**:
     ```rust
     pub enum ProviderError {
         UnsupportedProvider { provider: String },
         MissingApiKey { provider: String, env_var: String },
         ClientBuild(String),
         Prompt(String),
         MaxTurnsExceeded { max_turns: u32 },
     }
     ```

   - **Add `Display` arm** for the new variant in the existing `impl fmt::Display for ProviderError`:
     ```rust
     Self::MaxTurnsExceeded { max_turns } => {
         write!(f, "max turns exceeded: {max_turns} turns")
     }
     ```

   - **Import `PromptError`** at the top of the file. Based on rig-core source, the correct import is `rig::completion::PromptError`.

   - **Extract a private helper** to avoid duplicating the error-mapping closure across both `BuiltAgent` arms:
     ```rust
     fn map_prompt_error(e: PromptError) -> ProviderError {
         match e {
             PromptError::MaxTurnsError { max_turns, .. } => {
                 ProviderError::MaxTurnsExceeded { max_turns: max_turns as u32 }
             }
             other => ProviderError::Prompt(other.to_string()),
         }
     }
     ```

   - **Update `BuiltAgent::prompt()`** to use the helper in both `OpenAi` and `Anthropic` arms:
     ```rust
     Self::OpenAi(agent) => agent.prompt(input).await.map_err(map_prompt_error),
     Self::Anthropic(agent) => agent.prompt(input).await.map_err(map_prompt_error),
     ```

   - **Add a unit test** in the existing `#[cfg(test)] mod tests` block:
     ```rust
     #[test]
     fn max_turns_exceeded_displays_count() {
         let err = ProviderError::MaxTurnsExceeded { max_turns: 5 };
         assert!(err.to_string().contains("5"));
     }
     ```

2. **`crates/agent-runtime/src/runtime_agent.rs`**

   - **Import `ProviderError`** from `crate::provider::ProviderError`.

   - **Update `invoke()` error mapping** to distinguish `MaxTurnsExceeded` from other errors. Replace the single `.map_err(|e| AgentError::Internal(e.to_string()))` with a match:
     ```rust
     .map_err(|e| match e {
         ProviderError::MaxTurnsExceeded { .. } => AgentError::MaxTurnsExceeded {
             turns: self.manifest.constraints.max_turns,
         },
         other => AgentError::Internal(other.to_string()),
     })
     ```

### Key design decision: typed variant vs. string matching

The task breakdown mentions two approaches: (a) check for the `"MaxTurnError"` substring in the stringified error, or (b) improve `ProviderError` to carry a typed variant. This spec chooses approach (b) -- adding `ProviderError::MaxTurnsExceeded` -- for the following reasons:

- **Reliability**: String matching is fragile; if rig-core changes the error message format, the detection breaks silently. A typed match produces a compile-time error if the upstream enum changes.
- **Consistency**: The codebase already uses typed error enums (`AgentError`, `ProviderError`). Adding a variant follows the established pattern.
- **Minimal cost**: Adding one enum variant, one `Display` arm, and one helper function is low effort and does not introduce new dependencies.

### Why `manifest.constraints.max_turns` instead of `PromptError::MaxTurnsError::max_turns`

`PromptError::MaxTurnsError::max_turns` is `usize` and reflects the rig-core turn limit (which may differ from the manifest value due to the `as usize` cast, though in practice they are identical on 64-bit platforms). Using `manifest.constraints.max_turns` is preferred because:

- It is the authoritative source of the constraint as declared in the skill file.
- It avoids a `usize` to `u32` downcast (which would need a `try_from` or silent narrowing).
- The `AgentError::MaxTurnsExceeded { turns }` field semantically means "the configured limit", not "the internal loop counter".

## Dependencies

- Blocked by: "Set `default_max_turns` from constraints at agent build time" -- without `default_max_turns` being wired into the agent builder, rig-core will never produce `PromptError::MaxTurnsError`, making this mapping dead code.
- Blocking: "Wire ConstraintEnforcer into main.rs" -- the constraint enforcer needs the max-turns error to be properly surfaced so that the full enforcement pipeline is functional.

## Risks & Edge Cases

1. **`PromptError` may not be re-exported from `rig::completion`**: The import path must be verified. Based on the rig-core source (`src/completion/request.rs` defines `PromptError`, and `src/completion/mod.rs` re-exports it), the correct import is `rig::completion::PromptError`. If this is not publicly accessible, the implementation must use the full path or fall back to string matching.

2. **`usize` to `u32` narrowing in `map_prompt_error`**: `PromptError::MaxTurnsError::max_turns` is `usize`. Casting to `u32` could narrow on 64-bit if the value exceeds `u32::MAX` (~4 billion turns). This is unrealistic in practice since skill manifests are validated with `max_turns` being greater than 0 and practical limits are single digits. A `u32::try_from(...).unwrap_or(u32::MAX)` guard is acceptable but not strictly necessary.

3. **Exhaustive match on `ProviderError` in `invoke()`**: Other code that matches on `ProviderError` (currently only `invoke()`) must handle the new variant. Since the match uses an `other` wildcard for the fallthrough, this is automatically covered, but the implementer should verify no other call sites exist.

4. **`PromptError` variants may grow**: Future rig-core versions may add new `PromptError` variants. The `other` catch-all arm mapping to `ProviderError::Prompt(other.to_string())` in `map_prompt_error` handles this gracefully.

5. **Thread safety**: `RuntimeAgent::invoke()` accesses `self.manifest.constraints.max_turns` which is immutable after construction. No synchronization concerns.

## Verification

1. `cargo check --workspace` compiles without errors, confirming that `PromptError::MaxTurnsError` can be pattern-matched in `BuiltAgent::prompt()`.
2. `cargo clippy --workspace` produces no new warnings.
3. `cargo test --workspace` passes all existing tests with no regressions, including:
   - `ProviderError` display tests in `crates/agent-runtime/src/provider.rs`.
   - `RuntimeAgent` tests in `crates/agent-runtime/tests/runtime_agent_test.rs`.
   - HTTP handler tests in `crates/agent-runtime/tests/http_test.rs`.
4. The new `max_turns_exceeded_displays_count` test in `provider.rs` passes.
5. Manual code review confirms:
   - `BuiltAgent::prompt()` matches on `PromptError::MaxTurnsError` and produces `ProviderError::MaxTurnsExceeded`.
   - `RuntimeAgent::invoke()` matches on `ProviderError::MaxTurnsExceeded` and produces `AgentError::MaxTurnsExceeded { turns: self.manifest.constraints.max_turns }`.
   - All other error variants fall through to their previous behavior.
