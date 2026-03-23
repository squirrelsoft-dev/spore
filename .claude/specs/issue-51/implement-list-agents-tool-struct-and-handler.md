# Spec: Implement `ListAgentsTool` struct and handler
> From: .claude/tasks/issue-51.md

## Objective
Create `tools/list-agents/src/list_agents.rs` containing the `ListAgentsTool` MCP tool struct and handler. The tool reads agent configuration from environment variables (mirroring `crates/orchestrator/src/config.rs`), supports optional filtering, and returns a JSON array of agents.

## Current State
- No `list_agents.rs` file exists yet.
- `crates/orchestrator/src/config.rs` already implements `AGENT_ENDPOINTS` / `AGENT_DESCRIPTIONS` parsing via `parse_comma_pairs`, but that logic lives behind `OrchestratorError` and is not reusable as a library. The list-agents tool must reimplement equivalent parsing locally.
- Existing tool patterns (`echo.rs`, `register_agent.rs`) establish the `ToolRouter<Self>` / `#[tool_router]` / `#[tool_handler]` boilerplate.

## Requirements

### Structs
1. `ListAgentsRequest` -- derive `Debug, serde::Deserialize, schemars::JsonSchema`.
   - One field: `filter: Option<String>` with doc comment `/// Optional substring to filter agents by name or description`.
2. `ListAgentsTool` -- derive `Debug, Clone`.
   - Field: `tool_router: ToolRouter<Self>`.
   - Constructor `pub fn new() -> Self` that calls `Self::tool_router()`.

### Agent resolution (pure functions)
Extract env-var reading and parsing into pure helper functions so tests can call them without mutating process env vars.

1. `fn parse_endpoints(raw: &str) -> Result<Vec<(String, String)>, String>` -- split on `,`, then on `=`, trim whitespace, reject empty keys/values. Return descriptive error on malformed input.
2. `fn parse_descriptions(raw: &str) -> HashMap<String, String>` -- same comma/equals split but lenient: skip malformed pairs silently (descriptions are optional metadata).
3. `fn build_agent_list(endpoints: &[(String, String)], descriptions: &HashMap<String, String>) -> Vec<AgentInfo>` -- join on agent name; missing description defaults to `""`.
4. `fn filter_agents(agents: &[AgentInfo], filter: &str) -> Vec<AgentInfo>` -- case-insensitive substring match on both `name` and `description`. Empty filter returns all agents.

### AgentInfo helper struct
```rust
#[derive(Debug, Clone, serde::Serialize)]
struct AgentInfo {
    name: String,
    url: String,
    description: String,
}
```
Not public -- internal to the module.

### Tool handler (`#[tool_router]` impl)
- Method: `fn list_agents(&self, Parameters(request): Parameters<ListAgentsRequest>) -> String`.
- Read `AGENT_ENDPOINTS` via `std::env::var`. If missing or empty, return `{"agents": []}`.
- Parse endpoints with `parse_endpoints`. On error, return `{"agents": [], "error": "<message>"}`.
- Read `AGENT_DESCRIPTIONS` via `std::env::var`. If missing or empty, use empty `HashMap`.
- Parse descriptions with `parse_descriptions`.
- Build agent list with `build_agent_list`.
- If `filter` is `Some(f)` and `f` is non-empty, apply `filter_agents`.
- Serialize result as `{"agents": [...]}` via `serde_json::json!`.

### ServerHandler impl
Standard boilerplate matching `echo.rs`:
```rust
#[tool_handler]
impl ServerHandler for ListAgentsTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
```

## Dependencies
- `rmcp` (already in workspace) -- `ToolRouter`, `Parameters`, `ServerCapabilities`, `ServerInfo`, `schemars`, `tool`, `tool_handler`, `tool_router`, `ServerHandler`.
- `serde`, `serde_json`, `schemars` -- already in workspace.
- No new crate dependencies required.

Blocked by: "Create `tools/list-agents/Cargo.toml`" (the crate must exist before this file compiles).

## Risks & Edge Cases

1. **Env var mutation in tests** -- Process-wide env vars cause test flakiness under parallel execution. All parsing logic must be in pure functions that accept `&str` inputs so unit tests never call `std::env::set_var`.
2. **Malformed `AGENT_ENDPOINTS`** -- entries like `foo` (no `=`), `=bar` (empty key), `foo=` (empty value). `parse_endpoints` must return an error; the handler must surface it in the `error` field without panicking.
3. **Malformed `AGENT_DESCRIPTIONS`** -- same edge cases but handled leniently (skip bad pairs) because descriptions are optional.
4. **Whitespace** -- leading/trailing whitespace around commas, keys, and values must be trimmed (matching `config.rs` behavior).
5. **Empty filter string** -- `Some("")` should behave the same as `None` (return all agents).
6. **Unicode in names/descriptions** -- substring matching must use `.to_lowercase()` for case-insensitive comparison, which handles ASCII correctly. Non-ASCII case folding is out of scope.

## Verification

### Unit tests (6 required, in `#[cfg(test)] mod tests`)
All tests call pure functions directly -- no env var mutation.

1. **`test_empty_endpoints`** -- `parse_endpoints("")` returns `Ok(vec![])`. `build_agent_list(&[], &HashMap::new())` returns empty vec. Full flow returns `{"agents": []}`.
2. **`test_single_agent`** -- `parse_endpoints("alpha=http://a:8080")` returns one pair. `build_agent_list` with matching description produces one `AgentInfo`. JSON output has one entry with all three fields.
3. **`test_multiple_agents`** -- `parse_endpoints("a=http://a:80,b=http://b:80")` returns two pairs. Descriptions map has entry for `a` only. Result: agent `a` has description, agent `b` has `""`.
4. **`test_filter_matches_name`** -- Three agents, filter `"alp"`. Only agents whose name contains `"alp"` are returned.
5. **`test_filter_case_insensitive`** -- Agent named `"Alpha"`, filter `"ALPHA"`. Must match.
6. **`test_missing_descriptions`** -- Endpoints present, descriptions map empty. All agents returned with `description: ""`.

### Manual verification
After implementation, run:
```bash
cargo test -p list-agents
cargo clippy -p list-agents
```
