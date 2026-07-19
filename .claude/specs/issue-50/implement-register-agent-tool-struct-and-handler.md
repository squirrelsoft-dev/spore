# Spec: Implement `RegisterAgentTool` struct and handler in `src/register_agent.rs`

> From: .claude/tasks/issue-50.md

## Objective
Create `tools/register-agent/src/register_agent.rs` containing the core MCP tool implementation for registering an agent with the orchestrator via HTTP POST. Unlike the `docker_push` and `cargo_build` tools which spawn subprocesses, this tool uses `reqwest::blocking::Client` to make an HTTP request to the orchestrator's `/register` endpoint. The file defines the request struct, input validation, orchestrator URL resolution, HTTP POST invocation, and structured JSON response, following the same structural pattern as `docker_push.rs`.

## Current State
- `tools/docker-push/src/docker_push.rs` exists as the reference pattern. It defines `DockerPushRequest`, `DockerPushTool`, validates input via a character allowlist, invokes `docker push` via `std::process::Command`, and returns structured JSON. It uses `#[tool_router]`, `#[tool_handler]`, and `ServerHandler` from `rmcp`.
- `tools/docker-push/src/main.rs` is 7 lines: declares `mod docker_push;`, imports the tool struct, and calls `mcp_tool_harness::serve_stdio_tool`.
- `crates/orchestrator/src/orchestrator.rs` has `Orchestrator::register(&mut self, endpoint: AgentEndpoint)` as an in-process method. There is no HTTP `/register` route exposed yet. The tool should POST to `/register` regardless; the endpoint will be added to the orchestrator in a separate issue.
- `crates/orchestrator/src/agent_endpoint.rs` defines `AgentEndpoint` with fields `name: String`, `description: String`, `url: String`, and a private `client: reqwest::Client`. The `AgentConfig` struct in `config.rs` mirrors this with `name`, `description`, and `url` fields.
- The `tools/register-agent/` directory does not yet exist. This task is blocked by "Create `tools/register-agent/Cargo.toml`" which will provide dependencies including `reqwest` with `blocking` and `json` features.
- No `register_agent.rs` file exists anywhere in the workspace.
- The `#[tool_router]` handler methods throughout this codebase are synchronous (return `String`), so `reqwest::blocking::Client` is required rather than async reqwest.

## Requirements

### Request struct: `RegisterAgentRequest`
- Derive `Debug`, `serde::Deserialize`, `schemars::JsonSchema`.
- Field `name: String` with doc comment `/// Agent name to register`.
- Field `url: String` with doc comment `/// Agent HTTP endpoint (e.g., http://deploy-agent:8080)`.
- Field `description: String` with doc comment `/// Agent capabilities description`.

### Tool struct: `RegisterAgentTool`
- Field: `tool_router: ToolRouter<Self>`.
- Derive `Debug, Clone` (matching the `DockerPushTool` pattern).
- Constructor `new()` that calls `Self::tool_router()`.

### Input validation
- All three fields (`name`, `url`, `description`) must be non-empty. Return error JSON if any field is empty, with a message identifying which field is missing.
- URL must start with `http://` or `https://`. Reject URLs with any other scheme (e.g., `ftp://`) with an error message indicating invalid URL scheme.
- URL must not contain shell metacharacters. Use a character allowlist approach similar to `docker_push`'s `validate_image_ref`, but with an expanded allowed set appropriate for URLs: alphanumeric characters plus `.`, `-`, `_`, `/`, `:`, `@`, `?`, `=`, `#`, `%`, `+`, `~`.
- On validation failure, return structured error JSON via `build_error_json`.

### Orchestrator URL resolution
- Read `ORCHESTRATOR_URL` from `std::env::var("ORCHESTRATOR_URL")`.
- If unset or empty, default to `http://orchestrator:8080`.
- The registration endpoint is `{orchestrator_url}/register`.
- Trim trailing slashes from the orchestrator URL before appending `/register`.
- Extract this logic into a standalone helper function (`resolve_orchestrator_url` or similar) so it can be unit-tested without making HTTP requests.

### HTTP POST to orchestrator
- Use `reqwest::blocking::Client::new()` to create a client (no need to store it on the struct since registrations are infrequent).
- Build a JSON payload: `{"name": name, "url": url, "description": description}`.
- POST to `{orchestrator_url}/register`.
- Handle three outcomes:
  1. **Success (2xx response):** return `{"success": true, "agent_name": name, "registered_url": url}`.
  2. **Non-2xx response:** return `{"success": false, "agent_name": name, "registered_url": "", "error": "Registration failed: HTTP <status>"}`.
  3. **Connection/network error:** return `{"success": false, "agent_name": name, "registered_url": "", "error": "Orchestrator unreachable: <error>"}`.

### Error JSON helper: `build_error_json`
- Standalone function `fn build_error_json(name: &str, reason: &str) -> String` that returns `serde_json::json!({"success": false, "agent_name": name, "registered_url": "", "error": reason}).to_string()`.
- Used by validation failures, non-2xx responses, and connection errors to avoid duplication.

### ServerHandler implementation
- `#[tool_handler] impl ServerHandler for RegisterAgentTool` with `get_info()` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.

### Unit tests (`#[cfg(test)] mod tests`)
Eight tests, none of which require a running orchestrator:

1. **`rejects_empty_name`** -- Call with `name: ""`, `url: "http://test:8080"`, `description: "test"`. Assert `success: false` and `error` contains a message about empty name.

2. **`rejects_empty_url`** -- Call with `name: "test"`, `url: ""`, `description: "test"`. Assert `success: false` and `error` contains a message about empty URL.

3. **`rejects_empty_description`** -- Call with `name: "test"`, `url: "http://test:8080"`, `description: ""`. Assert `success: false` and `error` contains a message about empty description.

4. **`rejects_invalid_url_scheme`** -- Call with `url: "ftp://bad"`. Assert `success: false` and `error` contains a message about invalid URL scheme.

5. **`rejects_url_with_shell_metachar`** -- Call with `url: "http://evil;host"`. Assert `success: false` and `error` contains a message about invalid character.

6. **`default_orchestrator_url`** -- Call `resolve_orchestrator_url` helper when `ORCHESTRATOR_URL` is not set. Assert it returns `http://orchestrator:8080`.

7. **`custom_orchestrator_url`** -- Set `ORCHESTRATOR_URL` env var, call `resolve_orchestrator_url`, assert the custom URL is returned. Clean up the env var after the test.

8. **`orchestrator_unreachable_returns_error`** -- Call with valid inputs (`name: "test-agent"`, `url: "http://test:8080"`, `description: "test agent"`). Since no orchestrator is running, assert `success: false` and `error` contains "unreachable" or connection error text.

### Test helper pattern
Follow the `docker_push.rs` test pattern: define a helper function `call_register_agent(tool, name, url, description)` that constructs a `RegisterAgentRequest` and invokes the tool method via `Parameters(...)`. Parse the returned string as `serde_json::Value` for assertions.

## Implementation Details

### Imports
```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

### Validation function: `validate_request`
A standalone `fn validate_request(request: &RegisterAgentRequest) -> Result<(), String>` that:
1. Returns `Err("name must not be empty")` if `name` is empty.
2. Returns `Err("url must not be empty")` if `url` is empty.
3. Returns `Err("description must not be empty")` if `description` is empty.
4. Returns `Err("url must start with http:// or https://")` if `url` does not start with either prefix.
5. Calls `validate_url_chars` on the URL. Returns the error if validation fails.
6. Returns `Ok(())` on success.

### URL character validation function: `validate_url_chars`
A standalone `fn validate_url_chars(url: &str) -> Result<(), String>` that:
1. Iterates over each character.
2. Accepts: alphanumeric, `.`, `-`, `_`, `/`, `:`, `@`, `?`, `=`, `#`, `%`, `+`, `~`.
3. Rejects any character outside the allowlist with `Err(format!("invalid character '{}' in URL", ch))`.
4. Returns `Ok(())` on success.

Shell metacharacters (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `{`, `}`, `<`, `>`, `!`) are implicitly rejected by the allowlist.

### Orchestrator URL resolution function: `resolve_orchestrator_url`
A standalone `fn resolve_orchestrator_url() -> String` that:
1. Reads `std::env::var("ORCHESTRATOR_URL")`.
2. If `Ok(url)` and non-empty, trims trailing slashes and returns the result.
3. Otherwise returns `"http://orchestrator:8080"`.

### Error JSON helper: `build_error_json`
```rust
fn build_error_json(name: &str, reason: &str) -> String {
    serde_json::json!({
        "success": false,
        "agent_name": name,
        "registered_url": "",
        "error": reason,
    })
    .to_string()
}
```

### Tool method: `register_agent`
```rust
#[tool(description = "Register a deployed agent endpoint with the orchestrator")]
fn register_agent(&self, Parameters(request): Parameters<RegisterAgentRequest>) -> String
```

Follows this flow:
1. Validate the request via `validate_request`. On failure, return error JSON via `build_error_json`.
2. Resolve the orchestrator URL via `resolve_orchestrator_url`.
3. Build the registration endpoint: `format!("{}/register", orchestrator_url)`.
4. Create `reqwest::blocking::Client::new()` and POST the JSON payload.
5. On connection error, return `build_error_json(&request.name, &format!("Orchestrator unreachable: {}", e))`.
6. On non-2xx status, return `build_error_json(&request.name, &format!("Registration failed: HTTP {}", status))`.
7. On 2xx success, return `serde_json::json!({"success": true, "agent_name": request.name, "registered_url": request.url}).to_string()`.

### Line budget
- Imports: ~6 lines
- `RegisterAgentRequest` struct: ~9 lines
- `RegisterAgentTool` struct + derives: ~5 lines
- `validate_url_chars`: ~12 lines
- `validate_request`: ~18 lines
- `resolve_orchestrator_url`: ~8 lines
- `build_error_json`: ~9 lines
- `#[tool_router] impl` with `new()` + `register_agent()`: ~30 lines
- `#[tool_handler] impl ServerHandler`: ~8 lines
- Tests module: ~100 lines
- Total: ~205 lines

Each function stays well under the 50-line limit from the project rules.

## Dependencies
- **Blocked by:** "Create `tools/register-agent/Cargo.toml`" -- the crate must exist with `reqwest = { version = "0.13", features = ["blocking", "json"] }` before this file can be compiled.
- **Blocking:** "Write `main.rs`" (which declares `mod register_agent;` and uses `RegisterAgentTool`), "Write integration tests" (which spawn the binary and exercise the tool over MCP).
- **New crate dependency:** `reqwest` with `blocking` and `json` features. This is the same `reqwest` version (0.13) already in the workspace lockfile via `crates/orchestrator`, so it adds no new transitive dependencies. The `blocking` feature is additionally needed because the `#[tool_router]` handler is synchronous.

## Risks & Edge Cases

1. **No orchestrator `/register` endpoint yet.** The orchestrator currently only has an in-process `register()` method on the `Orchestrator` struct (in `crates/orchestrator/src/orchestrator.rs`). There is no HTTP route that accepts POST to `/register`. The tool should be implemented to POST to this endpoint regardless. Until the orchestrator exposes the route, all calls will return connection errors or 404s. The tool's "orchestrator unreachable" error handling covers this gracefully. The `orchestrator_unreachable_returns_error` unit test validates this path.

2. **`reqwest::blocking` in a tokio runtime.** The MCP tool harness runs inside a tokio runtime (`#[tokio::main]` in `main.rs`). Calling `reqwest::blocking::Client` methods from within an active tokio runtime can panic with "Cannot start a runtime from within a runtime". However, the `#[tool_router]` handler method is synchronous and `rmcp` runs tool handlers on a blocking thread pool, so this should work correctly. If issues arise during implementation, the alternative is to use `tokio::task::spawn_blocking` or use async reqwest with `Handle::current().block_on()`. Verify during implementation that the blocking client works in the tool harness context.

3. **Environment variable thread safety in tests.** Tests 6 and 7 (`default_orchestrator_url` and `custom_orchestrator_url`) read or manipulate the `ORCHESTRATOR_URL` environment variable. `std::env::set_var` and `std::env::remove_var` are not thread-safe. If `cargo test` runs these tests in parallel, they could interfere with each other or with test 8. Mitigation: refactor `resolve_orchestrator_url` to accept an `Option<&str>` override parameter for testability, testing the logic without touching env vars. Alternatively, use `#[serial]` from the `serial_test` crate, though adding a dependency is less desirable.

4. **URL validation edge cases.** The allowlist approach accepts characters like `%`, `+`, `~` for URL compatibility but does not validate full URL structure (e.g., it would accept `http://` with nothing after the scheme). This is acceptable since the tool's purpose is to pass the URL to the orchestrator, which will perform its own validation. The tool's role is to prevent shell injection and obvious malformation (wrong scheme).

5. **Payload shape alignment.** The JSON payload sent to `/register` uses `{"name", "url", "description"}` which mirrors the `AgentConfig` struct fields in `crates/orchestrator/src/config.rs`. When the orchestrator `/register` HTTP route is implemented, it should deserialize this same shape. If the orchestrator expects a different payload structure, the tool will need to be updated.

6. **MCP tool name.** The `#[tool_router]` macro derives the tool name from the method name. The method `register_agent` produces tool name `register_agent` (snake_case). The binary name is `register-agent` (kebab-case) and the env macro is `CARGO_BIN_EXE_register-agent`. This matches the naming convention used by `docker_push` / `docker-push`.

7. **`schemars` re-export availability.** The `RegisterAgentRequest` derives `schemars::JsonSchema`. The `rmcp` crate re-exports `schemars`, matching the `docker_push` pattern which uses `schemars` without a direct dependency. If the re-export is not available, `schemars` would need to be added to `Cargo.toml`.

## Verification

1. **Compilation:** `cargo check -p register-agent` succeeds with no errors.
2. **Lint:** `cargo clippy -p register-agent` produces no warnings.
3. **Unit tests:** `cargo test -p register-agent -- --lib` passes all eight unit tests:
   - `rejects_empty_name`
   - `rejects_empty_url`
   - `rejects_empty_description`
   - `rejects_invalid_url_scheme`
   - `rejects_url_with_shell_metachar`
   - `default_orchestrator_url`
   - `custom_orchestrator_url`
   - `orchestrator_unreachable_returns_error`
4. **Line count:** Each function in `register_agent.rs` is under 50 lines. The file overall is under 250 lines.
5. **No commented-out code or debug statements** in the final file.
6. **Pattern conformance:** The file structure (imports, request struct, tool struct, validation helpers, error helper, `#[tool_router]` impl, `#[tool_handler]` impl, `#[cfg(test)]` mod) mirrors `tools/docker-push/src/docker_push.rs`.
7. **Workspace check:** `cargo check` (full workspace) succeeds with no regressions.
