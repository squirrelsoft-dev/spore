# Task Breakdown: implement register_agent MCP tool

> Add a `register-agent` MCP tool binary that POSTs agent registration payloads to the orchestrator, following the established tool pattern (echo-tool / docker-push).

## Group 1 — Scaffolding

_Tasks in this group can be done in parallel._

- [x] **Create register-agent Cargo.toml** `[S]`
      Create `tools/register-agent/Cargo.toml` following the docker-push pattern. Package name `register-agent`, edition 2024. Dependencies: `mcp-tool-harness` (path), `rmcp` (with transport-io, server, macros features), `tokio` (with macros, rt, io-std), `serde` (with derive), `serde_json`, and `reqwest` (with json feature) for HTTP POST to the orchestrator. Dev-dependencies: `mcp-test-utils` (path), `tokio` (with macros, rt, rt-multi-thread), `rmcp` (with client, transport-child-process), `serde_json`, and a lightweight mock server (prefer `axum` or bare `tokio::net::TcpListener` over adding `mockito`/`wiremock`).
      Files: `tools/register-agent/Cargo.toml`
      Blocking: "Implement register_agent tool logic", "Create main.rs entry point", "Write integration tests"

- [x] **Add register-agent to workspace members** `[S]`
      Add `"tools/register-agent"` to the `[workspace] members` array in the root `Cargo.toml`.
      Files: `Cargo.toml`
      Blocking: "Implement register_agent tool logic", "Create main.rs entry point", "Write integration tests"

## Group 2 — Core Implementation

_Depends on: Group 1_

- [x] **Implement register_agent tool logic** `[M]`
      Create `tools/register-agent/src/register_agent.rs` following the docker-push pattern. Define `RegisterAgentRequest` struct with fields: `name` (String), `url` (String), `description` (String). Define `RegisterAgentTool` struct with `tool_router: ToolRouter<Self>`. Implement input validation: reject empty `name`, `url`, and `description`; validate `url` format; validate `name` contains only safe characters. Read `ORCHESTRATOR_URL` from env var with default `http://orchestrator:8080`. Use `reqwest::Client` to POST JSON payload `{"name": ..., "url": ..., "description": ...}` to `{orchestrator_url}/register`. Return JSON `{"success": true, "agent_name": ..., "registered_url": ...}` on success, or `{"success": false, "agent_name": ..., "registered_url": "", "error": "..."}` on failure. Use `#[tool_router]` and `#[tool_handler]` macros. The tool method must be async since it makes HTTP calls — verify that `rmcp` `#[tool]` macro supports async fn.
      Files: `tools/register-agent/src/register_agent.rs`
      Blocked by: "Create register-agent Cargo.toml", "Add register-agent to workspace members"
      Blocking: "Create main.rs entry point", "Write unit tests", "Write integration tests"

- [x] **Create main.rs entry point** `[S]`
      Create `tools/register-agent/src/main.rs` following the docker-push pattern: `mod register_agent; use register_agent::RegisterAgentTool;` then `#[tokio::main(flavor = "current_thread")] async fn main()` calling `mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await`.
      Files: `tools/register-agent/src/main.rs`
      Blocked by: "Implement register_agent tool logic"
      Blocking: "Write integration tests"

## Group 3 — Tests and Documentation

_Depends on: Group 2_

- [x] **Write unit tests** `[M]`
      Add `#[cfg(test)] mod tests` in `register_agent.rs`. Test input validation: empty name rejected, empty url rejected, empty description rejected, name with shell metacharacters rejected. Test error JSON structure on validation failure. For HTTP POST logic, stand up a lightweight mock HTTP server (use `axum` or `tokio::net::TcpListener`). Test: successful registration returns correct JSON, orchestrator returning 4xx/5xx produces error JSON, orchestrator unreachable produces error JSON with meaningful message.
      Files: `tools/register-agent/src/register_agent.rs`
      Blocked by: "Implement register_agent tool logic"
      Blocking: None

- [x] **Write integration tests (MCP server tests)** `[M]`
      Create `tools/register-agent/tests/register_agent_server_test.rs` following the docker-push server test pattern. Tests: (1) `tools_list_returns_register_agent_tool` — spawn MCP client, assert single tool named `register_agent` with params `["name", "url", "description"]`; (2) `tools_call_with_empty_name_returns_error` — call tool with empty name, assert `success: false`; (3) `tools_call_with_valid_inputs_returns_structured_json` — call tool with valid inputs (orchestrator unreachable in CI, so assert response has expected JSON shape with `success` field present).
      Files: `tools/register-agent/tests/register_agent_server_test.rs`
      Blocked by: "Create main.rs entry point"
      Blocking: None

- [x] **Write README.md** `[S]`
      Create `tools/register-agent/README.md` following the docker-build README pattern. Document: purpose, build/run/test commands, MCP Inspector command, parameters table (name, url, description), output JSON table (success, agent_name, registered_url, error), success/failure examples, environment variables (ORCHESTRATOR_URL with default), security considerations.
      Files: `tools/register-agent/README.md`
      Blocked by: "Implement register_agent tool logic"
      Blocking: None

## Key Design Decisions

1. **reqwest dependency**: The issue calls for `reqwest` to POST to the orchestrator. Already used in the `orchestrator` crate, so not a new workspace-level dependency.
2. **Async tool method**: Unlike docker-build/docker-push which use synchronous `std::process::Command`, this tool needs async HTTP via `reqwest`. Verify `rmcp` `#[tool]` macro supports async methods.
3. **No orchestrator changes needed**: The tool POSTs to a registration endpoint assumed to exist. The orchestrator already has `register(&mut self, endpoint: AgentEndpoint)`. Wiring the HTTP endpoint is outside this issue's scope.
4. **Mock strategy**: Use `axum` or bare `tokio::net::TcpListener` for test mocks to avoid adding new dependencies.

### Critical Files for Implementation
- `tools/docker-push/src/docker_push.rs` — Primary pattern to follow
- `tools/docker-push/tests/docker_push_server_test.rs` — Integration test pattern
- `tools/docker-push/Cargo.toml` — Dependency template; add `reqwest`
- `Cargo.toml` — Root workspace; add `"tools/register-agent"` to members
- `crates/orchestrator/src/agent_endpoint.rs` — `AgentEndpoint` struct the POST payload should align with
