# Spec: Add `EmbeddingError` variant to `OrchestratorError`

> From: .claude/tasks/issue-16.md

## Objective

Add an `EmbeddingError { reason: String }` variant to the `OrchestratorError` enum so that the upcoming `SemanticRouter` can propagate embedding-related failures (network timeouts, provider API errors, malformed responses, etc.) through the orchestrator's existing error hierarchy. Without this variant, the semantic router would have no type-safe way to surface embedding failures to callers.

## Current State

`crates/orchestrator/src/error.rs` defines `OrchestratorError` as a `#[derive(Debug, Clone)]` enum with five variants:

- `NoRoute { input: String }` -- no agent matched the request
- `AgentUnavailable { name: String, reason: String }` -- agent exists but is unhealthy
- `EscalationFailed { chain: Vec<String>, reason: String }` -- escalation chain broke
- `HttpError { url: String, reason: String }` -- downstream HTTP call failed
- `Config { reason: String }` -- configuration problem

Each variant uses owned `String` fields (no lifetimes, no generic parameters). The enum implements:
- `fmt::Display` with a `match` arm per variant
- `std::error::Error` (empty impl, relying on `Display`)
- `From<OrchestratorError> for AgentError` converting to `AgentError::Internal(err.to_string())`

The `Clone` derive is significant: any new variant must also be `Clone`-compatible, which rules out storing rig-core's `EmbeddingError` directly (it contains `reqwest::Error` which is not `Clone`). The task description specifies `reason: String` which satisfies this constraint by stringifying the upstream error.

## Requirements

1. Add variant `EmbeddingError { reason: String }` to the `OrchestratorError` enum.
2. Add a corresponding arm to the `Display` impl that formats as `"Embedding error: {reason}"`.
3. The enum must continue to derive `Debug` and `Clone` without issue (the `String` field guarantees this).
4. The existing `impl std::error::Error for OrchestratorError` and `impl From<OrchestratorError> for AgentError` require no changes -- they work automatically with the new variant via `Display`.
5. No new dependencies or imports are needed in `error.rs` for this change.
6. The crate must compile, pass clippy, and pass all existing tests after the change.

## Implementation Details

**File to modify:** `crates/orchestrator/src/error.rs`

**Change 1 -- Add the variant to the enum:**

Insert `EmbeddingError { reason: String }` as the last variant in the `OrchestratorError` enum (after `Config`). Placement after the existing variants preserves logical grouping: the first three variants are routing/agent lifecycle errors, `HttpError` and `Config` are infrastructure errors, and `EmbeddingError` is also infrastructure-level.

**Change 2 -- Add the Display arm:**

Add a match arm in the `Display` impl:

```rust
OrchestratorError::EmbeddingError { reason } => {
    write!(f, "Embedding error: {}", reason)
}
```

Place it after the `Config` arm to mirror the variant declaration order.

**No other files need modification** for this task. The `SemanticRouter` (a later task) will create `OrchestratorError::EmbeddingError` instances by converting rig-core's embedding errors into `reason: String` at the call site, e.g.:

```rust
model.embed_text(text).await.map_err(|e| OrchestratorError::EmbeddingError {
    reason: e.to_string(),
})?;
```

## Dependencies

- **Blocked by:** Nothing (Group 1 task, can be done in parallel with other Group 1 tasks)
- **Blocking:** "Implement `SemanticRouter` struct with two-phase routing" (Group 2) -- the SemanticRouter will return `OrchestratorError::EmbeddingError` when embedding calls fail

## Risks & Edge Cases

- **Non-exhaustive match in downstream code:** Any existing code that matches on `OrchestratorError` with explicit variant arms (without a wildcard `_`) will fail to compile after adding this variant. A search of the codebase confirms there are no such external matches -- all pattern matches on `OrchestratorError` are in `error.rs` itself (the `Display` impl), and the `From<OrchestratorError> for AgentError` impl uses `.to_string()` which delegates to `Display`. No breakage expected.
- **Clone constraint:** rig-core's `EmbeddingError` wraps `reqwest::Error` which is not `Clone`. By using `reason: String` instead of storing the upstream error directly, we preserve the `Clone` derive. The tradeoff is loss of the original error chain for programmatic inspection, but this is acceptable since the orchestrator's error-handling pattern already stringifies errors at the boundary (see the `From<OrchestratorError> for AgentError` impl).
- **Display format consistency:** The format `"Embedding error: {reason}"` follows the same pattern as `"Config error: {reason}"` and `"HTTP error calling {url}: {reason}"`, keeping user-facing messages consistent.

## Verification

1. Run `cargo check -p orchestrator` -- must succeed with no errors.
2. Run `cargo clippy -p orchestrator` -- must produce no new warnings.
3. Run `cargo test -p orchestrator` -- all existing tests must continue to pass.
4. Manually inspect `error.rs` and confirm:
   - The new variant is present in the enum with field `reason: String`.
   - The `Display` impl has a matching arm.
   - The `#[derive(Debug, Clone)]` still compiles (trivially true with `String` field).
   - The `impl std::error::Error` and `impl From<OrchestratorError> for AgentError` require no changes.
