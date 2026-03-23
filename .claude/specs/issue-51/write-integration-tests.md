# Spec: Write integration tests
> From: .claude/tasks/issue-51.md

## Objective
Create `tools/list-agents/tests/list_agents_server_test.rs` with three integration tests that verify the `list-agents` MCP server binary works correctly end-to-end: tool discovery, empty-state behavior, and env-var-driven agent listing.

## Current State
- `mcp-test-utils` crate provides `spawn_mcp_client!` macro and `assert_single_tool` helper, both used by existing integration tests (e.g., `tools/register-agent/tests/register_agent_server_test.rs`).
- The `list-agents` binary and `ListAgentsTool` handler do not exist yet; this spec is blocked by the `main.rs` and handler implementation tasks.
- The established integration test pattern uses `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`, spawns the server via `spawn_mcp_client!`, exercises tools through the `rmcp` client API, and cancels the client at the end.

## Requirements
1. **`tools_list_returns_list_agents_tool`** — Call `list_tools` on the spawned server and verify:
   - Exactly one tool is returned.
   - Tool name is `"list_agents"`.
   - Description contains the substring `"agent"`.
   - Input schema properties include `["filter"]`.
   - Use `mcp_test_utils::assert_single_tool` for all assertions.

2. **`tools_call_returns_empty_when_no_agents`** — Call `list_agents` with no arguments (or empty filter), without setting `AGENT_ENDPOINTS` / `AGENT_DESCRIPTIONS`:
   - Parse the response text as JSON.
   - Assert `json["agents"]` is an empty array (`[]`).

3. **`tools_call_returns_agents_from_env`** — Before spawning the child process, set env vars:
   - `AGENT_ENDPOINTS=foo=http://localhost:8080,bar=http://localhost:9090`
   - `AGENT_DESCRIPTIONS=foo=A foo agent,bar=A bar agent`
   - Call `list_agents` with no filter.
   - Parse the response text as JSON.
   - Assert `json["agents"]` contains two entries with correct `name`, `url`, and `description` fields.

## Implementation Details

### File location
`tools/list-agents/tests/list_agents_server_test.rs`

### Imports
```rust
use rmcp::model::CallToolRequestParams;
```
The `mcp_test_utils` crate is used via its macro path (`mcp_test_utils::spawn_mcp_client!` and `mcp_test_utils::assert_single_tool`).

### Test 1: `tools_list_returns_list_agents_tool`
- Spawn client with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_list-agents")).await`.
- Call `mcp_test_utils::assert_single_tool(&client, "list_agents", "agent", &["filter"]).await`.
- Cancel the client.

### Test 2: `tools_call_returns_empty_when_no_agents`
- Spawn client (no env vars set).
- Build `CallToolRequestParams::new("list_agents")` with an empty arguments object `{}`.
- Call `client.peer().call_tool(params).await`.
- Extract first content item as text, parse as JSON.
- Assert `json["agents"]` is an array and is empty.
- Cancel the client.

### Test 3: `tools_call_returns_agents_from_env`
- Use `unsafe { std::env::set_var(...) }` to set `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` before spawning. The `unsafe` block is required because `set_var` is unsafe in recent Rust editions (the child process inherits the parent's environment at spawn time).
- Spawn client.
- Build `CallToolRequestParams::new("list_agents")` with empty arguments.
- Call tool, extract text, parse JSON.
- Assert `json["agents"]` is an array of length 2.
- Assert each entry has `name`, `url`, and `description` fields with expected values. Since array order is not guaranteed, either sort before comparing or check that both expected agents exist somewhere in the array.
- Cancel the client.
- Clean up env vars with `unsafe { std::env::remove_var(...) }` to avoid polluting other tests (though since each test spawns a separate child process, cross-test contamination of the *child* is not a concern; this is for parent-process hygiene).

### Pattern notes
- Every test function uses `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- Every test ends with `client.cancel().await.expect("failed to cancel client")`.
- Response parsing follows the same chain: `.content.first().as_text().text` then `serde_json::from_str`.

## Dependencies
- **Blocked by:** `main.rs` implementation (Group 2) and `ListAgentsTool` handler (Group 2).
- **Crate dependencies (dev):** `mcp-test-utils` (path), `tokio` (rt-multi-thread), `rmcp` (client, transport-child-process), `serde_json`. These must be declared in `tools/list-agents/Cargo.toml` (handled by the Cargo.toml task).

## Risks & Edge Cases
1. **Env var race conditions:** `std::env::set_var` mutates global process state and is not thread-safe. Since `cargo test` runs tests in parallel threads by default, setting env vars in one test can leak into another. Mitigation: test 3 is the only test that sets env vars, and the child process captures env at spawn time, so the main risk is if two tests run simultaneously and one reads stale env. Using unique var names or `--test-threads=1` would eliminate the risk, but the established codebase pattern (see `register_agent_server_test.rs` line 55) accepts this trade-off.
2. **Array ordering:** The order of agents in the response JSON array may not match insertion order. The test should not assume a specific order; check for membership rather than index equality.
3. **Binary not found:** `env!("CARGO_BIN_EXE_list-agents")` is resolved at compile time by Cargo. If the binary target name in `Cargo.toml` does not match `list-agents`, compilation will fail with a clear error.

## Verification
- `cargo test -p list-agents --test list_agents_server_test` — all three tests pass.
- `cargo clippy -p list-agents` — no warnings on the test file.
