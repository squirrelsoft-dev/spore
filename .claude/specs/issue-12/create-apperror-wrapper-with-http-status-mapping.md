# Spec: Create AppError wrapper with HTTP status mapping

> From: .claude/tasks/issue-12.md

## Objective

Create an `AppError` newtype that wraps `AgentError` and implements `axum::response::IntoResponse`, so that HTTP handlers can use `?` to propagate agent errors and have them automatically converted into appropriate HTTP responses with JSON bodies. This is a prerequisite for the HTTP handler module (Group 2) and ensures consistent, well-typed error responses across all endpoints.

## Current State

- `AgentError` is defined in `crates/agent-sdk/src/agent_error.rs` with four variants:
  - `ToolCallFailed { tool: String, reason: String }`
  - `ConfidenceTooLow { confidence: f32, threshold: f32 }`
  - `MaxTurnsExceeded { turns: u32 }`
  - `Internal(String)`
- `AgentError` already derives `Debug, Clone, PartialEq, Serialize, Deserialize` and manually implements `fmt::Display` and `std::error::Error` (no `thiserror`).
- The project follows a manual error implementation pattern as seen in `crates/agent-runtime/src/provider.rs` where `ProviderError` manually implements `Display` and `Error` without `thiserror`.
- `crates/agent-runtime/src/http.rs` does not exist yet. It will be created by this task and then extended by the "Create HTTP handler module" task.
- `axum` is not yet a dependency of `agent-runtime` (the "Add axum dependency" task is a sibling in Group 1). The `AppError` code depends on `axum::response::IntoResponse` and `axum::http::StatusCode`.
- `serde_json` is already a dependency of `agent-runtime`.

## Requirements

- Define a public struct `AppError` in `crates/agent-runtime/src/http.rs` that wraps `AgentError`.
- Implement `axum::response::IntoResponse` for `AppError` with the following status code mapping:
  - `ToolCallFailed` -> `StatusCode::BAD_GATEWAY` (502)
  - `ConfidenceTooLow` -> `StatusCode::OK` (200) -- this is not a server error; it represents a successful escalation. The response body must include the confidence and threshold values.
  - `MaxTurnsExceeded` -> `StatusCode::UNPROCESSABLE_ENTITY` (422)
  - `Internal` -> `StatusCode::INTERNAL_SERVER_ERROR` (500)
- The response body for all variants must be the JSON serialization of the inner `AgentError`. The `Content-Type` header must be `application/json`.
- Implement `From<AgentError> for AppError` so handlers can use `?` on `Result<T, AgentError>` when the function returns `Result<T, AppError>`.
- Implement `fmt::Display` for `AppError` by delegating to the inner `AgentError::fmt`.
- Implement `fmt::Debug` for `AppError` by delegating to the inner `AgentError`'s `Debug`.
- Do NOT use `thiserror`. Follow the manual `impl` pattern from `provider.rs`.
- Register the `http` module in `crates/agent-runtime/src/lib.rs` (`pub mod http;`).
- Keep the `IntoResponse` implementation under 50 lines per project rules.

## Implementation Details

### Files to create

**`crates/agent-runtime/src/http.rs`**

This file will contain the `AppError` type. Later, the "Create HTTP handler module" task will add handler functions, `AppState`, and `build_router` to this same file.

Types and impls to add:

1. `pub struct AppError(pub AgentError);` -- a newtype wrapper.

2. `impl fmt::Display for AppError` -- delegate to `self.0.fmt(f)`.

3. `impl fmt::Debug for AppError` -- delegate to `self.0.fmt()` via `Debug`.

4. `impl From<AgentError> for AppError` -- wrap the error: `AppError(err)`.

5. `impl axum::response::IntoResponse for AppError`:
   - Match on `self.0` to determine the `StatusCode`.
   - Serialize `self.0` to JSON using `serde_json::to_string(&self.0)`. If serialization somehow fails (it should not since `AgentError` derives `Serialize`), fall back to a plain-text 500 response.
   - Construct the response as a tuple of `(StatusCode, [(header_name, header_value)], body_string)` which axum converts via its blanket `IntoResponse` impl. Alternatively, use `axum::Json` to set the content type automatically: return `(status, axum::Json(self.0)).into_response()`.

### Status code mapping (exact values)

| AgentError variant  | HTTP Status                  | Rationale                                         |
|---------------------|------------------------------|---------------------------------------------------|
| `ToolCallFailed`    | 502 Bad Gateway              | Upstream tool dependency failed                   |
| `ConfidenceTooLow`  | 200 OK                       | Successful escalation, not an error               |
| `MaxTurnsExceeded`  | 422 Unprocessable Entity     | Request could not be completed within constraints |
| `Internal`          | 500 Internal Server Error    | Unexpected internal failure                       |

### File to modify

**`crates/agent-runtime/src/lib.rs`**

Add `pub mod http;` to the module declarations.

### Imports needed in `http.rs`

- `use std::fmt;`
- `use agent_sdk::AgentError;`
- `use axum::http::StatusCode;`
- `use axum::response::IntoResponse;`
- `use axum::Json;`

### Unit tests to include in `http.rs`

Add a `#[cfg(test)] mod tests` block with the following tests (following the pattern in `provider.rs`):

1. `app_error_from_agent_error` -- verify `AppError::from(AgentError::Internal("x".into()))` wraps correctly.
2. `app_error_display_delegates` -- verify `AppError(err).to_string()` matches `err.to_string()`.
3. `tool_call_failed_returns_502` -- create `AppError` from `ToolCallFailed`, call `.into_response()`, assert status is 502 and body is valid JSON containing `tool` and `reason` fields.
4. `confidence_too_low_returns_200` -- create `AppError` from `ConfidenceTooLow`, call `.into_response()`, assert status is 200 and body JSON includes `confidence` and `threshold`.
5. `max_turns_exceeded_returns_422` -- assert status 422 and body contains `turns`.
6. `internal_returns_500` -- assert status 500 and body contains the error message.

Note: Testing `IntoResponse` requires converting the response body to bytes. Use `axum::body::to_bytes` (available in axum 0.8) or `http_body_util::BodyExt::collect` to read the response body in tests. Since `http-body-util` may be added later as a dev-dependency for integration tests (Group 4), the unit tests here can use `axum::body::to_bytes` which is available without extra dependencies.

## Dependencies

- **Blocked by**: "Add axum dependency to agent-runtime" (must have `axum = "0.8"` in `Cargo.toml` before this code compiles)
- **Blocking**: "Create HTTP handler module" (needs `AppError` to define handler return types)

## Risks & Edge Cases

- **Serialization failure**: `serde_json::to_string` on `AgentError` should never fail since all variants contain `String`, `f32`, and `u32`. However, the implementation should handle this defensively by falling back to a 500 plain-text response if serialization fails unexpectedly.
- **`ConfidenceTooLow` as 200**: This is intentional and may surprise consumers expecting all error-path responses to be non-2xx. The response body clearly indicates escalation via the JSON-serialized `ConfidenceTooLow` variant with `confidence` and `threshold` fields, which distinguishes it from a successful `AgentResponse`.
- **Future variant additions**: If new `AgentError` variants are added, the `IntoResponse` match will fail to compile (non-exhaustive match), which is the desired behavior to force explicit status code decisions.
- **Content-Type header**: Using `axum::Json(self.0)` as part of the response tuple ensures the `Content-Type: application/json` header is set automatically. Do not manually set this header.

## Verification

- `cargo check -p agent-runtime` compiles without errors (requires the axum dependency task to be done first).
- `cargo clippy -p agent-runtime` produces no warnings.
- `cargo test -p agent-runtime` passes all unit tests in `http.rs`, specifically:
  - Each `AgentError` variant maps to the correct HTTP status code.
  - Response bodies are valid JSON containing the expected fields.
  - `From<AgentError>` conversion works, enabling `?` propagation.
  - `Display` delegation produces the same string as the inner `AgentError`.
- `cargo test` across the full workspace shows no regressions.
