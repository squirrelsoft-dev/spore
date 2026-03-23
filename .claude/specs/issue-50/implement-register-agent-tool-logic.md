# Spec: Implement register_agent tool logic

> From: .claude/tasks/issue-50.md

## Objective
Create the core `RegisterAgentTool` implementation in `tools/register-agent/src/register_agent.rs`. This tool accepts an agent's name, URL, and description, validates the inputs, then POSTs a registration payload to the orchestrator's `/register` endpoint via `reqwest`. It follows the established MCP tool pattern (docker-push/docker-build) but differs in that the tool method must be async to perform HTTP calls.

## Current State
- `tools/docker-push/src/docker_push.rs` defines the primary pattern: a `*Request` struct with `serde::Deserialize` + `schemars::JsonSchema`, a `*Tool` struct holding a `ToolRouter<Self>`, validation helpers, a `#[tool_router]` impl block with a `#[tool]`-annotated method, and a `#[tool_handler] impl ServerHandler`.
- `tools/docker-build/src/docker_build.rs` follows the same pattern with additional validation helpers.
- Both existing tools use synchronous `std::process::Command`. This tool must use async `reqwest::Client` instead.
- The rmcp `#[tool]` macro (v1.2.0) explicitly supports async fn: it detects `sig.asyncness`, strips it, and wraps the body in `Box::pin(async move { ... })`. This is confirmed in `rmcp-macros-1.2.0/src/tool.rs` lines 333-361.
- `crates/orchestrator/src/agent_endpoint.rs` defines `AgentEndpoint` with fields `name: String`, `description: String`, `url: String`, and a `client: reqwest::Client`. The POST payload sent by this tool should include `name`, `url`, and `description` to align with `AgentEndpoint::new()`.
- `crates/orchestrator/src/orchestrator.rs` has `pub fn register(&mut self, endpoint: AgentEndpoint)` which stores the endpoint. The HTTP route that accepts the POST is assumed to exist or will be wired separately (outside this issue's scope).

## Requirements
- Define `RegisterAgentRequest` with three required `String` fields: `name`, `url`, `description`. All derive `serde::Deserialize` and `schemars::JsonSchema`.
- Define `RegisterAgentTool` struct with field `tool_router: ToolRouter<Self>` and a `new()` constructor following the docker-push pattern.
- Input validation must reject:
  - Empty `name` (return error JSON)
  - Empty `url` (return error JSON)
  - Empty `description` (return error JSON)
  - `name` containing unsafe characters (only allow alphanumeric, `-`, `_`, `.`; reject shell metacharacters, spaces, etc.)
  - `url` that does not start with `http://` or `https://` (basic URL format check)
- Read `ORCHESTRATOR_URL` from environment variable, defaulting to `http://orchestrator:8080` if unset.
- Use `reqwest::Client` to POST JSON `{"name": ..., "url": ..., "description": ...}` to `{orchestrator_url}/register`.
- The tool method must be `async fn` (confirmed supported by rmcp `#[tool]` macro).
- On success (HTTP 2xx), return: `{"success": true, "agent_name": "<name>", "registered_url": "<url>"}`.
- On failure (HTTP error, network error, or validation error), return: `{"success": false, "agent_name": "<name>", "registered_url": "", "error": "<reason>"}`.
- Use `#[tool_router]` on the impl block and `#[tool_handler]` on `impl ServerHandler`.
- The `#[tool]` annotation must include a description, e.g. `"Register an agent with the orchestrator"`.
- All validation helpers must be standalone functions (not methods), each under 50 lines, following the single-responsibility pattern from docker-push.
- No commented-out code or debug statements in the final file.

## Implementation Details

### File to create
`tools/register-agent/src/register_agent.rs`

### Imports
```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

### Types

**`RegisterAgentRequest`** -- input struct for the tool:
- `name: String` -- agent name, doc comment: `/// Agent name (alphanumeric, hyphens, underscores, dots only)`
- `url: String` -- doc comment: `/// Agent URL (must start with http:// or https://)`
- `description: String` -- doc comment: `/// Human-readable description of the agent`

**`RegisterAgentTool`** -- the MCP tool handler:
- `tool_router: ToolRouter<Self>` -- populated via `Self::tool_router()` in `new()`

### Validation functions

1. `fn validate_name(name: &str) -> Result<(), String>` -- rejects empty names and names containing characters outside `[a-zA-Z0-9._-]`.
2. `fn validate_url(url: &str) -> Result<(), String>` -- rejects empty URLs and URLs not starting with `http://` or `https://`.
3. `fn validate_description(description: &str) -> Result<(), String>` -- rejects empty descriptions.

### Helper functions

4. `fn build_error_json(name: &str, reason: &str) -> String` -- returns `{"success": false, "agent_name": name, "registered_url": "", "error": reason}`.
5. `fn resolve_orchestrator_url() -> String` -- reads `ORCHESTRATOR_URL` env var, defaults to `"http://orchestrator:8080"`, trims trailing slashes.

### Tool method (in `#[tool_router] impl RegisterAgentTool`)

```rust
#[tool(description = "Register an agent with the orchestrator")]
async fn register_agent(&self, Parameters(request): Parameters<RegisterAgentRequest>) -> String
```

Flow:
1. Validate `name`, `url`, `description` -- return `build_error_json` on any failure.
2. Call `resolve_orchestrator_url()` to get base URL.
3. Construct `reqwest::Client::new()` and POST JSON payload to `{base_url}/register`.
4. On success: return success JSON.
5. On HTTP error or network error: return error JSON with the error message.

### ServerHandler implementation

```rust
#[tool_handler]
impl ServerHandler for RegisterAgentTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
```

### Integration points
- `tools/register-agent/src/main.rs` (created by a separate task) will `mod register_agent;` and use `RegisterAgentTool::new()`.
- The orchestrator's `/register` HTTP endpoint is the target of the POST. The payload shape (`name`, `url`, `description`) aligns with the `AgentEndpoint::new()` constructor parameters.

## Dependencies
- Blocked by: "Create register-agent Cargo.toml", "Add register-agent to workspace members"
- Blocking: "Create main.rs entry point", "Write unit tests", "Write integration tests"

## Risks & Edge Cases
- **Orchestrator endpoint does not exist yet**: The orchestrator has `register()` on its struct but may not have an HTTP route wired for `/register`. The tool should still be implemented to POST to this path; wiring the route is outside this issue's scope. Tests should use a mock HTTP server.
- **reqwest::Client lifetime**: Creating a new `reqwest::Client` per call is acceptable for an MCP tool (low call frequency). If performance becomes a concern, the client could be stored on the struct, but that adds complexity (not Clone-friendly with ToolRouter). Keep it simple for now.
- **Async in rmcp #[tool]**: Confirmed supported in rmcp-macros 1.2.0. The macro detects `asyncness`, removes it from the signature, and wraps the body in `Box::pin(async move { ... })`. No special handling needed by the implementer.
- **URL validation depth**: Only checking for `http://` or `https://` prefix is intentionally minimal. Full URL parsing (e.g., via the `url` crate) would add a dependency. The basic check prevents obvious misuse while keeping dependencies minimal.
- **Name character set**: Allowing `[a-zA-Z0-9._-]` matches common DNS/container naming conventions and prevents shell injection. This is more restrictive than docker-push's `is_valid_ref_char` (which also allows `/` and `:`) because agent names should be simple identifiers.
- **Network timeouts**: `reqwest::Client::new()` uses default timeouts. For a first implementation this is acceptable. If orchestrator calls hang in production, a timeout can be added later via `reqwest::ClientBuilder::timeout()`.

## Verification
- `cargo check -p register-agent` compiles without errors (requires Cargo.toml and main.rs tasks to be complete first).
- `cargo clippy -p register-agent` produces no warnings.
- `cargo test -p register-agent` passes all unit tests (validation tests do not require a running orchestrator).
- The file contains no `todo!()`, `unimplemented!()`, commented-out code, or debug print statements.
- The `#[tool]` method is declared `async fn` and the file compiles, confirming rmcp macro support.
- Manual review confirms: all three input fields are validated, error JSON includes `"success": false` with an `"error"` field, success JSON includes `"success": true` with `"agent_name"` and `"registered_url"`.
