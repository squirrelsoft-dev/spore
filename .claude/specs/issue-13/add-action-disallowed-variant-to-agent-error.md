# Spec: Add `ActionDisallowed` variant to `AgentError`

> From: .claude/tasks/issue-13.md

## Objective

Introduce an `ActionDisallowed` error variant so that the constraint enforcement system can report when a tool's action type is not in the agent's allowed actions list. This gives downstream consumers (HTTP callers, orchestrators) a structured, typed signal that a permission boundary was hit, distinct from general failures. The HTTP layer must map this to `403 Forbidden` so clients can distinguish authorization-style rejections from other errors.

## Current State

**`crates/agent-sdk/src/agent_error.rs`** defines `AgentError` as a four-variant enum:

- `ToolCallFailed { tool: String, reason: String }`
- `ConfidenceTooLow { confidence: f32, threshold: f32 }`
- `MaxTurnsExceeded { turns: u32 }`
- `Internal(String)`

The enum derives `Debug, Clone, PartialEq, Serialize, Deserialize` and has a manual `Display` impl with a match arm per variant, plus a blanket `impl std::error::Error`.

**`crates/agent-runtime/src/http.rs`** defines `AppError(pub AgentError)` which implements `IntoResponse`. The `into_response()` method maps each `AgentError` variant to an HTTP status code:

- `ToolCallFailed` -> `502 Bad Gateway`
- `ConfidenceTooLow` -> `200 OK`
- `MaxTurnsExceeded` -> `422 Unprocessable Entity`
- `Internal` -> `500 Internal Server Error`

The response body is the JSON-serialized `AgentError`. The test suite in the same file has one test per variant verifying both the status code and the round-tripped JSON body.

## Requirements

1. Add a new variant `AgentError::ActionDisallowed { action: String, allowed: Vec<String> }` to the `AgentError` enum.
2. The new variant must participate in all existing derives (`Debug, Clone, PartialEq, Serialize, Deserialize`) without additional manual impls — `Vec<String>` supports all of them.
3. Implement `Display` for the new variant with the format: `"Action '<action>' is not in allowed actions: [<comma-separated allowed>]"`. For example, `"Action 'write' is not in allowed actions: [read, query]"`.
4. In `AppError::into_response()`, map `AgentError::ActionDisallowed { .. }` to `StatusCode::FORBIDDEN` (403).
5. Add a test in `crates/agent-runtime/src/http.rs` (in the existing `mod tests` block) that constructs an `ActionDisallowed` error, converts it through `AppError::into_response()`, asserts the status is `403 Forbidden`, and round-trips the JSON body back to `AgentError` for equality.
6. All existing tests must continue to pass unchanged.

## Implementation Details

### File: `crates/agent-sdk/src/agent_error.rs`

- Add variant to the `AgentError` enum:
  ```
  ActionDisallowed { action: String, allowed: Vec<String> }
  ```
- Add a match arm in the `Display` impl:
  ```
  AgentError::ActionDisallowed { action, allowed } => {
      write!(f, "Action '{}' is not in allowed actions: [{}]", action, allowed.join(", "))
  }
  ```

### File: `crates/agent-runtime/src/http.rs`

- Add a match arm in `IntoResponse for AppError`:
  ```
  AgentError::ActionDisallowed { .. } => StatusCode::FORBIDDEN,
  ```
- Add a test following the exact pattern of existing tests (e.g., `internal_returns_500`):
  - Construct `AgentError::ActionDisallowed { action: "write".into(), allowed: vec!["read".into(), "query".into()] }`
  - Assert status is `StatusCode::FORBIDDEN`
  - Deserialize body and assert equality with the original error

### No new files are created.

### No new dependencies are required.

## Dependencies

- Blocked by: none
- Blocking: "Filter tools by `allowed_actions` in tool resolution"

## Risks & Edge Cases

- **Exhaustive match breakage**: Adding a variant to `AgentError` will cause compile errors in any `match` on `AgentError` that lacks a wildcard arm. The only known match sites are `Display` in `agent_error.rs` and `into_response` in `http.rs` — both are modified in this task. Verify with `cargo check` that no other crates have exhaustive matches on `AgentError`.
- **Serde backwards compatibility**: The new variant uses serde's default externally-tagged enum encoding, producing `{"ActionDisallowed":{"action":"...","allowed":[...]}}`. Existing JSON with only the four original variants will continue to deserialize correctly. Older consumers that receive the new variant will fail to deserialize it — this is acceptable for a pre-1.0 project.
- **Empty `allowed` list**: The `Display` format handles an empty `allowed` list gracefully, producing `"Action 'write' is not in allowed actions: []"`. No special-casing needed.

## Verification

1. `cargo check --workspace` compiles with no errors.
2. `cargo clippy --workspace` produces no warnings related to the changed files.
3. `cargo test --workspace` passes all tests, including:
   - The new `action_disallowed_returns_403` test (or similar name) in `crates/agent-runtime/src/http.rs`.
   - All four existing `AppError` status code tests remain green.
4. Manually confirm the `Display` output matches the specified format by inspecting the test or adding a unit test in `agent_error.rs` that calls `.to_string()` on an `ActionDisallowed` value and asserts the string.
