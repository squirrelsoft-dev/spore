# Spec: Define OrchestratorError enum

> From: .claude/tasks/issue-15.md

## Objective

Create the `OrchestratorError` enum in `crates/orchestrator/src/error.rs` to serve as the primary error type for all orchestrator operations (routing, dispatch, health checks, escalation). This enum follows the manual `Display + Error` pattern established by `AgentError` in `agent-sdk`, and provides a `From` conversion so orchestrator errors can flow through the `MicroAgent::invoke()` boundary as `AgentError::Internal(String)`.

## Current State

**`crates/agent-sdk/src/agent_error.rs`** defines the SDK-level error type using a manual `Display` implementation (no `thiserror` derive). It has five variants including `Internal(String)`, which is the catch-all for errors originating outside the agent's own logic. The type derives `Debug, Clone, PartialEq, Serialize, Deserialize` and implements `std::error::Error` as an empty impl.

**`crates/orchestrator/src/main.rs`** is a placeholder 3-line `println!` stub. The orchestrator crate currently has no dependencies in `Cargo.toml` and no real logic. A sibling task will convert this from a binary to a library crate (`lib.rs` with `pub mod error;`), and another sibling task will add `agent-sdk` as a dependency.

**`crates/agent-sdk/src/micro_agent.rs`** defines the `MicroAgent` trait whose `invoke()` method returns `Result<AgentResponse, AgentError>`. The orchestrator's `MicroAgent` implementation will call `dispatch()` which returns `Result<AgentResponse, OrchestratorError>`, so the `From<OrchestratorError> for AgentError` conversion is needed at that boundary.

## Requirements

1. Define `OrchestratorError` as a `pub enum` in `crates/orchestrator/src/error.rs` with exactly four variants:
   - `NoRoute { input: String }` -- no registered agent matches the request input
   - `AgentUnavailable { name: String, reason: String }` -- the target agent is unhealthy or unreachable
   - `EscalationFailed { chain: Vec<String>, reason: String }` -- the escalation chain was exhausted without resolution
   - `HttpError { url: String, reason: String }` -- network or HTTP failure when calling a downstream agent

2. Derive `Debug, Clone` on `OrchestratorError`. Do NOT derive `Serialize`/`Deserialize` (this is an internal error type, not serialized over the wire). Do NOT derive `PartialEq` because `Vec<String>` comparisons in `EscalationFailed` are fragile for tests; instead, tests should match on variant and check fields individually.

3. Implement `fmt::Display for OrchestratorError` manually (no `thiserror`), following the same match-arm pattern as `AgentError::Display`. Each variant should produce a distinct, human-readable message:
   - `NoRoute` -> `"No route found for input: {input}"`
   - `AgentUnavailable` -> `"Agent '{name}' unavailable: {reason}"`
   - `EscalationFailed` -> `"Escalation failed through chain [{chain joined by " -> "}]: {reason}"`
   - `HttpError` -> `"HTTP error calling {url}: {reason}"`

4. Implement `std::error::Error for OrchestratorError` as an empty impl (matching the `AgentError` pattern).

5. Implement `From<OrchestratorError> for AgentError` that converts any `OrchestratorError` into `AgentError::Internal(err.to_string())`. This uses the `Display` output as the internal error message.

6. Do NOT use `thiserror` or any new dependency. The manual `Display + Error` pattern is deliberate and consistent with `agent-sdk`.

7. The file should import `AgentError` from `agent_sdk` (via `agent_sdk::AgentError` -- the crate name uses an underscore in Rust code since the package name is `agent-sdk`).

## Implementation Details

### File to create

**`crates/orchestrator/src/error.rs`**

- `use std::fmt;`
- `use agent_sdk::AgentError;`
- Define `#[derive(Debug, Clone)] pub enum OrchestratorError` with the four variants listed above.
- `impl fmt::Display for OrchestratorError` -- one match arm per variant, each calling `write!()` with the format strings specified in Requirements.
- `impl std::error::Error for OrchestratorError {}` -- empty impl body.
- `impl From<OrchestratorError> for AgentError` -- single match arm: `AgentError::Internal(err.to_string())` (no need to match per-variant, just use the `Display` output).

### Integration points

- **Module declaration**: The sibling task "Convert orchestrator from binary to library crate" will create `lib.rs` with `pub mod error;`. This spec only covers the `error.rs` file itself.
- **Cargo.toml dependency**: The sibling task "Update orchestrator Cargo.toml with dependencies" will add `agent-sdk = { path = "../agent-sdk" }`. This file requires that dependency to import `AgentError`.
- **Consumers**: `AgentEndpoint` methods will return `Result<_, OrchestratorError>` for HTTP and availability errors. `Orchestrator::route()` will return `NoRoute`. `Orchestrator::dispatch()` will return `EscalationFailed`. The `MicroAgent::invoke()` impl will use the `From` conversion at the `?` operator boundary.

### Key design decisions

- The `From` impl converts TO `AgentError` (not from). The direction is `From<OrchestratorError> for AgentError`, which lets orchestrator code write `Ok(self.dispatch(request)?)` in the `MicroAgent::invoke()` body and have the `?` operator auto-convert.
- `EscalationFailed.chain` is a `Vec<String>` of agent names in the order they were tried, providing debuggability for escalation failures.
- `HttpError` is intentionally generic (url + reason string) rather than wrapping `reqwest::Error` directly, to avoid coupling the error type to a specific HTTP client library and to keep the type `Clone`-compatible.

## Dependencies

- **Blocked by**: "Convert orchestrator from binary to library crate" (needs `lib.rs` with `pub mod error;`), "Update orchestrator Cargo.toml with dependencies" (needs `agent-sdk` dependency)
- **Blocking**: "Implement AgentEndpoint struct" (returns `OrchestratorError` from its methods), "Implement Orchestrator struct with dispatch logic" (returns `OrchestratorError` from `route()` and `dispatch()`)

## Risks & Edge Cases

- **Dependency ordering**: This file will not compile until the Cargo.toml has `agent-sdk` as a dependency. The three Group 1 tasks should be applied together or in the order: Cargo.toml update, then lib.rs creation, then error.rs creation.
- **Crate name resolution**: The Cargo package is `agent-sdk` but Rust normalizes hyphens to underscores, so the import is `agent_sdk::AgentError`. If the dependency is aliased differently in Cargo.toml, the import path must match.
- **Future extensibility**: Additional variants may be needed later (e.g., `ConfigError`, `TimeoutError`). The enum is not `#[non_exhaustive]` since it is crate-internal and downstream consumers interact through the `AgentError::Internal` conversion. If this changes, `#[non_exhaustive]` should be reconsidered.
- **Display format stability**: The `From<OrchestratorError> for AgentError` impl relies on `Display` output becoming the `Internal(String)` message. If the display format changes, error messages in logs and responses will change accordingly. This is acceptable since these are not parsed programmatically.

## Verification

1. `cargo check -p orchestrator` passes (requires sibling tasks to be complete first)
2. `cargo clippy -p orchestrator` produces no warnings on the new file
3. All four `Display` outputs match the specified format strings (verified by the sibling test task "Write unit tests for OrchestratorError" in `crates/orchestrator/tests/error_test.rs`)
4. `From<OrchestratorError> for AgentError` converts each variant to `AgentError::Internal(...)` where the inner string matches the `Display` output
5. `cargo test -p orchestrator` passes (once test file from the test task exists)
6. No new dependencies are introduced beyond what the sibling Cargo.toml task specifies
