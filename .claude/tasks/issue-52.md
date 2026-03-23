# Task Breakdown: Implement route_to_agent MCP tool

> Implement `route_to_agent` as a standalone Rust MCP server binary that forwards a request to a named agent via HTTP and returns its `AgentResponse`, following the echo-tool/register-agent reference pattern.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/route-to-agent/Cargo.toml`** `[S]`
      Copy `tools/register-agent/Cargo.toml` and change `name = "route-to-agent"`. Dependencies: `mcp-tool-harness` (path), `rmcp` with `transport-io`/`server`/`macros`, `tokio` with `macros`/`rt`/`io-std`, `serde` with `derive`, `serde_json`, `reqwest` with `json` feature (needed for HTTP POST to agent `/invoke` endpoint), `uuid` with `v4`/`serde` features (needed for `AgentRequest`). Dev-dependencies: `mcp-test-utils` (path), `tokio` with `macros`/`rt`/`rt-multi-thread`/`net`, `rmcp` with `client`/`transport-child-process`, `serde_json`.
      Files: `tools/route-to-agent/Cargo.toml`
      Blocking: "Implement `RouteToAgentTool` struct and handler", "Write `main.rs`", "Write integration tests"

- [x] **Add `"tools/route-to-agent"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/route-to-agent"` to the `members` list in the root `Cargo.toml`, after the existing `"tools/docker-build"` entry.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `RouteToAgentTool` struct and handler in `src/route_to_agent.rs`** `[M]`
      Create `tools/route-to-agent/src/route_to_agent.rs`. This is the core logic file.

      **Request struct:** Define `RouteToAgentRequest` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` containing:
      - `agent_name: String` — target agent name (required)
      - `input: String` — the request to forward (required)

      **Tool struct:** Define `RouteToAgentTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`.

      **Agent endpoint resolution:** Parse `AGENT_ENDPOINTS` env var (comma-separated `name=url` pairs, same format as list-agents and orchestrator config). Look up the target agent's URL by matching `agent_name` against the parsed endpoint names. Reuse the same `parse_endpoints` pattern from `tools/list-agents/src/list_agents.rs` lines 38-54.

      **HTTP forwarding logic (async):**
      1. Parse `AGENT_ENDPOINTS` and look up `agent_name`. If not found, return error JSON: `{"success": false, "agent_name": "<name>", "error": "Agent '<name>' not found in AGENT_ENDPOINTS"}`.
      2. Construct an `AgentRequest` (from `agent-sdk` crate): `AgentRequest::new(input)`. Note: this requires depending on `agent-sdk` or manually constructing the JSON with `uuid::Uuid::new_v4()`, `input`, `context: null`, `caller: null`. Preferring to depend on `agent-sdk` directly is cleaner.
      3. Build a `reqwest::Client` with `connect_timeout(5s)` and `timeout(30s)` (matching register-agent pattern).
      4. POST the `AgentRequest` as JSON to `{agent_url}/invoke`.
      5. On success (2xx): deserialize the response body as `AgentResponse` and return it serialized as the success JSON: `{"success": true, "agent_name": "<name>", "response": <AgentResponse>}`.
      6. On HTTP error (non-2xx): return error JSON with the status code and body.
      7. On connection/network error: return error JSON with the error message.

      **Error JSON structure:** `{"success": false, "agent_name": "<name>", "response": null, "error": "<message>"}`.

      **Success JSON structure:** `{"success": true, "agent_name": "<name>", "response": {"id": "...", "output": ..., "confidence": ..., "escalated": ..., "tool_calls": [...]}, "error": ""}`.

      Implement `ServerHandler` with `#[tool_handler]` returning tools-enabled capabilities (same as all other tools).

      Extract pure helper functions: `parse_endpoints` (can be copied from list-agents), `resolve_agent_url`, `build_error_json`, `build_success_json`.

      Files: `tools/route-to-agent/src/route_to_agent.rs`
      Blocked by: "Create `tools/route-to-agent/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write unit tests", "Write integration tests"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/route-to-agent/src/main.rs`. Mirror `tools/echo-tool/src/main.rs`: declare `mod route_to_agent;`, use `RouteToAgentTool`, call `mcp_tool_harness::serve_stdio_tool(RouteToAgentTool::new(), "route-to-agent").await`. Under 10 lines.
      Files: `tools/route-to-agent/src/main.rs`
      Blocked by: "Implement `RouteToAgentTool` struct and handler"
      Blocking: "Write integration tests"

## Group 3 — Tests and documentation

_Depends on: Group 2_

- [x] **Write unit tests in `route_to_agent.rs`** `[M]`
      Add `#[cfg(test)] mod tests` to `route_to_agent.rs`. Follow the register-agent test pattern with mock TCP servers.

      **Validation tests (no HTTP):**
      1. `rejects_empty_agent_name` — call with empty `agent_name`, assert `success: false` and error mentions "name"
      2. `rejects_empty_input` — call with empty `input`, assert `success: false` and error mentions "input"
      3. `returns_error_when_agent_not_found` — set `AGENT_ENDPOINTS` to `foo=http://localhost:8080`, call with `agent_name: "bar"`, assert `success: false` and error mentions "not found"
      4. `returns_error_when_no_endpoints_env` — unset `AGENT_ENDPOINTS`, call route, assert `success: false` error

      **Mock HTTP server tests (reuse register-agent's `start_mock_server` and `find_unused_port` patterns):**
      5. `successful_route_returns_agent_response` — start mock server returning a valid `AgentResponse` JSON with 200, set up tool with that URL, assert `success: true` and response fields match
      6. `agent_returns_4xx_produces_error` — mock returns 400, assert `success: false`
      7. `agent_returns_5xx_produces_error` — mock returns 500, assert `success: false`
      8. `agent_unreachable_produces_error` — use `find_unused_port`, assert `success: false` and error mentions connection failure

      Use `with_agent_endpoints` or similar constructor (like register-agent's `with_orchestrator_url`) to inject endpoints without env var mutation.

      Files: `tools/route-to-agent/src/route_to_agent.rs`
      Blocked by: "Implement `RouteToAgentTool` struct and handler"
      Blocking: None

- [x] **Write integration tests in `tests/route_to_agent_server_test.rs`** `[M]`
      Create `tools/route-to-agent/tests/route_to_agent_server_test.rs`. Use `spawn_client_with_env` pattern from list-agents integration tests (not the simple `spawn_mcp_client!` macro, since env vars need to be set on the child process).

      Tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):
      1. `tools_list_returns_route_to_agent_tool` — use `mcp_test_utils::assert_single_tool` to verify tool name is `"route_to_agent"`, description contains `"route"` or `"forward"`, and parameters include `["agent_name", "input"]`.
      2. `tools_call_returns_error_when_agent_not_found` — set `AGENT_ENDPOINTS` to some value, call `route_to_agent` with a non-existent agent name, assert error response.

      Files: `tools/route-to-agent/tests/route_to_agent_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: None

- [x] **Write `README.md`** `[S]`
      Create `tools/route-to-agent/README.md` following the pattern from `tools/register-agent/README.md`. Include: description, build/run/test commands, MCP Inspector command, input parameters (`agent_name` required, `input` required), output format (JSON with `success`, `agent_name`, `response`, `error`), environment variable configuration (`AGENT_ENDPOINTS`), error cases (agent not found, agent unreachable, agent error), and usage examples.
      Files: `tools/route-to-agent/README.md`
      Blocking: None

## Group 4 — Verification

_Depends on: Groups 1-3_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p route-to-agent`, `cargo test -p route-to-agent`, `cargo clippy -p route-to-agent`, and `cargo check` (workspace-wide). Verify all acceptance criteria: build succeeds, tests pass, tool is named `route_to_agent` in MCP tools/list, successfully routes a request to a target agent and returns its response, returns structured error when agent name is not found, returns structured error when target agent is unreachable.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

## Implementation Notes

1. **Depends on `agent-sdk` crate**: Unlike list-agents and register-agent, route-to-agent needs to construct `AgentRequest` and deserialize `AgentResponse`. Add `agent-sdk = { path = "../../crates/agent-sdk" }` to dependencies. This brings in `uuid`, `serde`, `schemars` transitively. The `AgentRequest::new(input)` constructor generates a UUID automatically. The `AgentResponse` struct has fields: `id` (Uuid), `output` (Value), `confidence` (f32), `escalated` (bool), `escalate_to` (Option<String>), `tool_calls` (Vec<ToolCallRecord>).

2. **Requires `reqwest`**: Like register-agent, this tool makes HTTP calls. Use `reqwest = { version = "0.13", features = ["json"] }`.

3. **`AGENT_ENDPOINTS` parsing**: Reuse the same `parse_endpoints` function pattern from `tools/list-agents/src/list_agents.rs`. The format is `name=url,name2=url2`. Do not depend on the orchestrator config module directly; duplicate the small parsing function locally (same as list-agents does).

4. **Agent `/invoke` endpoint**: The target agent exposes `POST /invoke` accepting `AgentRequest` JSON and returning `AgentResponse` JSON. This is defined in `crates/agent-runtime/src/http.rs` lines 64-70.

5. **Test isolation**: Use `with_endpoints` constructor pattern (like register-agent's `with_orchestrator_url`) to inject parsed endpoints in tests without mutating global env vars. For integration tests, use the `spawn_client_with_env` pattern from list-agents to set env vars on the child process only.

6. **Tool registry registration**: The acceptance criteria mention "Registered in tool registry as `route_to_agent`". The MCP tool name is determined by the `#[tool(description = "...")]` attribute on the method named `route_to_agent`. The tool registry (`crates/tool-registry`) is separate infrastructure for runtime registration; for this task, the MCP tool name is sufficient.

### Critical Files for Implementation
- `tools/register-agent/src/register_agent.rs` — Primary pattern for HTTP-calling MCP tools with reqwest, mock server tests, error JSON structure
- `tools/list-agents/src/list_agents.rs` — `parse_endpoints` function to reuse and `AGENT_ENDPOINTS` env var parsing pattern
- `crates/agent-sdk/src/agent_request.rs` — `AgentRequest` struct and `::new()` constructor for building the forwarded request
- `crates/agent-sdk/src/agent_response.rs` — `AgentResponse` struct to deserialize the target agent's reply
- `crates/agent-runtime/src/http.rs` — `/invoke` endpoint handler showing the expected request/response contract
