# Task Breakdown: Implement list_agents MCP tool

> Implement `list_agents` as a standalone Rust MCP server binary that reads agent registrations from `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` environment variables and returns them as a JSON array, following the established tool pattern.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/list-agents/Cargo.toml`** `[S]`
      Copy `tools/register-agent/Cargo.toml` and change `name = "list-agents"`. Keep the same dependency set: `mcp-tool-harness` (path), `rmcp` with `transport-io`/`server`/`macros`, `tokio` with `macros`/`rt`/`io-std`, `serde` with `derive`, `serde_json`. No `reqwest` needed since this tool only reads env vars (no HTTP calls). Dev-dependencies: `mcp-test-utils` (path), `tokio` with `rt-multi-thread`, `rmcp` with `client`/`transport-child-process`, `serde_json`.
      Files: `tools/list-agents/Cargo.toml`
      Blocking: "Implement `ListAgentsTool` struct and handler", "Write `main.rs`", "Write integration tests"

- [x] **Add `"tools/list-agents"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/list-agents"` to the `members` list in the root `Cargo.toml`, after the existing `"tools/docker-build"` entry.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `ListAgentsTool` struct and handler in `src/list_agents.rs`** `[M]`
      Create `tools/list-agents/src/list_agents.rs`. Define `ListAgentsRequest` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` containing one optional field:
      - `filter: Option<String>` — optional substring to match against agent name or description

      Define `ListAgentsTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`.

      **Agent resolution from env vars:** Reuse the same parsing logic as `crates/orchestrator/src/config.rs`:
      - Read `AGENT_ENDPOINTS` env var: comma-separated `name=url` pairs
      - Read `AGENT_DESCRIPTIONS` env var (optional): comma-separated `name=description` pairs
      - Build a list of `{name, url, description}` objects by joining on agent name
      - If `AGENT_ENDPOINTS` is missing or empty, return an empty JSON array (not an error, per the spec)

      **Filtering:** If `filter` is provided and non-empty, case-insensitively match it against both `name` and `description`, returning only agents where either field contains the filter substring.

      **Output:** Return a JSON string: `{"agents": [{name, url, description}, ...]}`. On parse errors in the env vars, return `{"agents": [], "error": "<message>"}`.

      Implement `ServerHandler` with `#[tool_handler]` returning tools-enabled capabilities.

      **Unit tests** in `#[cfg(test)] mod tests`:
      1. `returns_empty_array_when_no_env_vars` — with no env vars set, assert response contains `"agents": []`
      2. `parses_single_agent_from_env` — set `AGENT_ENDPOINTS=foo=http://localhost:8080` and `AGENT_DESCRIPTIONS=foo=A test agent`, assert one agent returned with correct fields
      3. `parses_multiple_agents_from_env` — set env vars with two comma-separated entries, assert both are returned
      4. `filter_narrows_results` — set env vars with two agents, call with filter matching only one, assert only the matching agent is returned
      5. `filter_is_case_insensitive` — set env var with agent name "MyAgent", filter with "myagent", assert it matches
      6. `missing_description_returns_empty_string` — set only `AGENT_ENDPOINTS`, assert agent is returned with empty description

      Note: Extract core logic into pure functions that accept parsed data, testing those independently. Only one or two tests should exercise the env var reading path to avoid race conditions.

      Files: `tools/list-agents/src/list_agents.rs`
      Blocked by: "Create `tools/list-agents/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write integration tests"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/list-agents/src/main.rs`. Mirror `tools/echo-tool/src/main.rs`: declare `mod list_agents;`, use `ListAgentsTool`, call `mcp_tool_harness::serve_stdio_tool(ListAgentsTool::new(), "list-agents").await`. Under 10 lines.
      Files: `tools/list-agents/src/main.rs`
      Blocked by: "Implement `ListAgentsTool` struct and handler"
      Blocking: "Write integration tests"

## Group 3 — Integration tests and documentation

_Depends on: Group 2_

- [x] **Write integration tests in `tests/list_agents_server_test.rs`** `[M]`
      Create `tools/list-agents/tests/list_agents_server_test.rs`. Use `spawn_mcp_client!(env!("CARGO_BIN_EXE_list-agents"))` pattern from `mcp-test-utils`.

      Tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):
      1. `tools_list_returns_list_agents_tool` — use `mcp_test_utils::assert_single_tool` to verify tool name is `"list_agents"`, description contains `"agent"`, and parameters include `["filter"]`.
      2. `tools_call_returns_empty_when_no_agents` — call `list_agents` with no filter and no env vars set, parse response, assert `agents` is an empty array.
      3. `tools_call_returns_agents_from_env` — set `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` env vars before spawning the binary, call `list_agents`, assert agents are returned correctly.

      Note: Since integration tests spawn a child process, env vars must be set before the process starts. Use `std::env::set_var` before `spawn_mcp_client!` to pass env vars to the child process.

      Files: `tools/list-agents/tests/list_agents_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: None

- [x] **Write `README.md`** `[S]`
      Create `tools/list-agents/README.md` following the pattern from `tools/register-agent/README.md`. Include: description, build/run/test commands, MCP Inspector command, input parameters (optional `filter`), output format (JSON array of `{name, url, description}`), environment variable configuration (`AGENT_ENDPOINTS`, `AGENT_DESCRIPTIONS`), and usage examples.
      Files: `tools/list-agents/README.md`
      Non-blocking

## Group 4 — Verification

_Depends on: Groups 1-3_

- [ ] **Run verification suite** `[S]`
      Run `cargo build -p list-agents`, `cargo test -p list-agents`, `cargo clippy -p list-agents`, and `cargo check` (workspace-wide). Verify all acceptance criteria: build succeeds, tests pass, tool is named `list_agents` in MCP tools/list, returns structured JSON array, works with `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` env vars, returns empty array when no agents are registered.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

## Implementation Notes

1. **No new dependencies needed**: All dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `mcp-tool-harness`) are already used by existing tools. Unlike `register-agent`, this tool does NOT need `reqwest` since it reads from environment variables rather than making HTTP calls.

2. **Env var parsing mirrors orchestrator config**: The `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` parsing logic in `crates/orchestrator/src/config.rs` (`parse_comma_pairs`) is the reference for how these env vars are structured (`name=value,name2=value2`). The list-agents tool should parse them identically.

3. **Thread safety for env var tests**: Extract core logic into pure functions that accept parsed data and test those independently. Only one or two tests should exercise the env var reading path.

4. **Filter semantics**: Case-insensitive substring matching on both `name` and `description`.

### Critical Files for Implementation
- `tools/echo-tool/src/echo.rs` — Simplest reference pattern for tool struct, macros, and test layout
- `crates/orchestrator/src/config.rs` — `AGENT_ENDPOINTS`/`AGENT_DESCRIPTIONS` env var parsing logic to replicate
- `tools/register-agent/src/register_agent.rs` — Agent-related tool with validation and async handler pattern
- `Cargo.toml` — Workspace members list to update
- `crates/mcp-test-utils/src/lib.rs` — Test utilities for integration tests
