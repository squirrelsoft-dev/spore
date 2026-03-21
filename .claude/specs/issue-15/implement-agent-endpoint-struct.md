# Spec: Implement AgentEndpoint struct

> From: .claude/tasks/issue-15.md

## Objective

Create the `AgentEndpoint` struct that represents a single downstream micro-agent reachable over HTTP. This is the orchestrator's primary mechanism for communicating with child agents -- it wraps `reqwest::Client` and provides typed methods for invoking an agent and checking its health. The struct must avoid importing anything from `agent-runtime` to prevent circular dependencies; it depends only on `agent-sdk` types (`AgentRequest`, `AgentResponse`, `HealthStatus`) and the orchestrator's own `OrchestratorError`.

## Current State

- **Orchestrator crate** (`crates/orchestrator/`) is a stub binary with a `main.rs` containing `println!("Hello, world!")` and an empty `Cargo.toml` dependencies section. Prerequisite tasks will convert it to a library crate and add dependencies before this task begins.
- **`agent-sdk` types** (defined in `crates/agent-sdk/src/`):
  - `AgentRequest { id: Uuid, input: String, context: Option<Value>, caller: Option<String> }` -- Serialize + Deserialize
  - `AgentResponse { id: Uuid, output: Value, confidence: f32, escalated: bool, escalate_to: Option<String>, tool_calls: Vec<ToolCallRecord> }` -- Serialize + Deserialize
  - `HealthStatus` -- enum with variants `Healthy`, `Degraded(String)`, `Unhealthy(String)` -- Serialize + Deserialize
- **Health endpoint contract** (defined in `crates/agent-runtime/src/http.rs`):
  - `GET /health` returns JSON: `{ "name": String, "version": String, "status": HealthStatus }`
  - The `HealthResponse` struct is defined in `agent-runtime::http` and must NOT be imported (to avoid circular dependency). A local minimal DTO will be used instead.
- **Invoke endpoint contract** (defined in `crates/agent-runtime/src/http.rs`):
  - `POST /invoke` accepts `AgentRequest` as JSON body, returns `AgentResponse` as JSON on success
  - On error, returns an HTTP error status with `AgentError` as the JSON body
- **`OrchestratorError`** (to be defined by a prerequisite task in `crates/orchestrator/src/error.rs`):
  - `HttpError { url: String, reason: String }` -- for reqwest/network failures
  - `AgentUnavailable { name: String, reason: String }` -- for unreachable/unhealthy agents

## Requirements

1. Define `AgentEndpoint` as a public struct in `crates/orchestrator/src/agent_endpoint.rs` with fields:
   - `pub name: String` -- the registered name of the downstream agent
   - `pub description: String` -- human-readable description of what the agent does (used later for routing)
   - `pub url: String` -- base URL of the agent (e.g., `http://localhost:3001`), no trailing slash
   - `client: reqwest::Client` -- private, shared HTTP client instance

2. Implement `AgentEndpoint::new(name: impl Into<String>, description: impl Into<String>, url: impl Into<String>) -> Self`:
   - Construct a single `reqwest::Client` instance (reused across all calls for connection pooling)
   - Store all fields, converting arguments to owned `String`s

3. Implement `AgentEndpoint::invoke(&self, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>`:
   - Send `POST {self.url}/invoke` with `request` serialized as JSON body
   - Set `Content-Type: application/json` header (handled by `reqwest`'s `.json()` method)
   - On success (2xx), deserialize the response body as `AgentResponse` and return it
   - On HTTP error status (non-2xx), attempt to deserialize body as error context; map to `OrchestratorError::HttpError { url, reason }` where `url` is the full invoke URL and `reason` includes the HTTP status code and any body text
   - On network/connection errors, map to `OrchestratorError::HttpError { url, reason }` with the reqwest error's display string as reason

4. Implement `AgentEndpoint::health(&self) -> Result<HealthStatus, OrchestratorError>`:
   - Send `GET {self.url}/health`
   - Define a local private DTO struct `HealthResponseDto` with `#[derive(Deserialize)]` containing only `status: HealthStatus` (ignoring `name` and `version` fields via `#[serde(default)]` or by simply omitting them -- serde's default behavior with `Deserialize` ignores unknown fields)
   - On success, deserialize as `HealthResponseDto` and return the `status` field
   - On any error (network, non-2xx, deserialization), map to `OrchestratorError::AgentUnavailable { name: self.name.clone(), reason: <error description> }`

5. The struct must NOT depend on `agent-runtime` crate. Only `agent-sdk` and standard/third-party crates (`reqwest`, `serde`, `serde_json`) are allowed.

6. All methods must be `async` (since they perform HTTP I/O).

7. Each method must be 50 lines or fewer per project rules.

## Implementation Details

### File to create

**`crates/orchestrator/src/agent_endpoint.rs`**

```
use agent_sdk::{AgentRequest, AgentResponse, HealthStatus};
use serde::Deserialize;

use crate::error::OrchestratorError;
```

### Types to define

- `AgentEndpoint` -- public struct as described above
- `HealthResponseDto` -- private (non-`pub`) struct used only for deserializing the `/health` response:
  ```rust
  #[derive(Deserialize)]
  struct HealthResponseDto {
      status: HealthStatus,
  }
  ```
  Serde will ignore the `name` and `version` fields in the JSON since they are not declared in the struct (serde's default behavior for structs is to ignore unknown fields). No `#[serde(deny_unknown_fields)]` should be used.

### Methods

- `new(name, description, url) -> Self`:
  - Use `impl Into<String>` for ergonomic construction
  - Call `reqwest::Client::new()` to create the shared client

- `invoke(&self, request: &AgentRequest) -> Result<AgentResponse, OrchestratorError>`:
  - Build URL: `format!("{}/invoke", self.url)`
  - Use `self.client.post(url).json(request).send().await`
  - Call `.error_for_status()` to convert non-2xx responses to reqwest errors
  - On success, call `.json::<AgentResponse>().await`
  - Map all `reqwest::Error` to `OrchestratorError::HttpError`

- `health(&self) -> Result<HealthStatus, OrchestratorError>`:
  - Build URL: `format!("{}/health", self.url)`
  - Use `self.client.get(url).send().await`
  - Call `.error_for_status()` then `.json::<HealthResponseDto>().await`
  - Map all errors to `OrchestratorError::AgentUnavailable`
  - Return `dto.status`

### Integration points

- Imported by `crates/orchestrator/src/orchestrator.rs` (future task) to build the agent registry
- `Orchestrator::dispatch()` will call `endpoint.invoke()` and `endpoint.health()`
- `Orchestrator::from_config()` will construct `AgentEndpoint` instances from `AgentConfig` entries
- The `lib.rs` module declaration `pub mod agent_endpoint;` makes this available as `orchestrator::agent_endpoint::AgentEndpoint`

## Dependencies

- **Blocked by:**
  - "Convert orchestrator from binary to library crate" -- `lib.rs` must declare `pub mod agent_endpoint;`
  - "Update orchestrator Cargo.toml with dependencies" -- `reqwest`, `serde`, `agent-sdk` must be available
  - "Define OrchestratorError enum" -- `OrchestratorError::HttpError` and `OrchestratorError::AgentUnavailable` must exist in `crate::error`
- **Blocking:**
  - "Implement Orchestrator struct with dispatch logic" -- uses `AgentEndpoint` in the registry
  - "Implement MicroAgent for Orchestrator" -- indirectly, through the Orchestrator struct

## Risks & Edge Cases

1. **Trailing slash in URL**: If `url` is provided as `http://host:3000/`, the constructed path becomes `http://host:3000//invoke`. Mitigation: strip trailing slashes in `new()` using `url.trim_end_matches('/')`.

2. **Non-2xx responses with valid AgentError body**: The invoke handler in `agent-runtime` returns structured `AgentError` JSON even on error status codes (e.g., 502 for `ToolCallFailed`). The current design maps all non-2xx to `OrchestratorError::HttpError`, which loses the structured error. This is acceptable for the initial implementation -- the Orchestrator's `dispatch()` method only needs to know the call failed. Future enhancement could preserve the `AgentError` if needed.

3. **Deserialization failures**: If the downstream agent returns malformed JSON (not matching `AgentResponse` or `HealthResponseDto`), the reqwest `.json()` call will return a deserialization error. This is correctly mapped to `HttpError` / `AgentUnavailable` respectively.

4. **Connection refused / timeout**: If the downstream agent is not running, `reqwest` will return a connection error. The `health()` method maps this to `AgentUnavailable`, which is the expected behavior for the orchestrator's health-check-before-dispatch pattern.

5. **reqwest::Client reuse**: `reqwest::Client` internally uses connection pooling. Creating one per `AgentEndpoint` is efficient. If multiple endpoints share the same host, they could benefit from a shared client, but the per-endpoint approach is simpler and sufficient for the initial implementation.

6. **HealthStatus enum variant compatibility**: The local `HealthResponseDto` deserializes `HealthStatus` from `agent-sdk`. Since both the downstream agent and the orchestrator use the same `agent-sdk` crate, the enum variants will match. If `HealthStatus` is extended in the future, both sides must be updated together (they share the same workspace dependency).

7. **Borrow vs. owned in invoke**: The `invoke` method takes `&AgentRequest` (borrowed) rather than owned, because the orchestrator may need to retry the request against a different agent (e.g., during escalation). `reqwest`'s `.json()` accepts `&T where T: Serialize`, so this works without cloning.

## Verification

1. **Compilation**: `cargo check -p orchestrator` succeeds with no errors or warnings.
2. **Linting**: `cargo clippy -p orchestrator` passes with no warnings.
3. **Unit tests** (defined in a separate task but should verify):
   - `new()` constructs an endpoint with correct field values
   - `invoke()` sends correct JSON payload to `{url}/invoke` and returns deserialized `AgentResponse`
   - `invoke()` maps HTTP error responses to `OrchestratorError::HttpError` with informative url and reason
   - `invoke()` maps connection failures to `OrchestratorError::HttpError`
   - `health()` returns the correct `HealthStatus` from a well-formed health response
   - `health()` correctly ignores the `name` and `version` fields in the health response JSON
   - `health()` maps connection failures to `OrchestratorError::AgentUnavailable` with the endpoint name
   - `health()` maps non-2xx responses to `OrchestratorError::AgentUnavailable`
4. **No dependency on agent-runtime**: `cargo tree -p orchestrator` does not list `agent-runtime` as a dependency.
5. **All methods are under 50 lines**: Manual inspection of each method body.
