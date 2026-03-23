# Spec: Write integration tests (MCP server tests)

> From: .claude/tasks/issue-50.md

## Objective

Create MCP server-level integration tests for the `register-agent` tool binary. These tests verify that the tool is correctly registered via the MCP protocol, validates inputs, and returns well-structured JSON responses. This ensures the full stdio MCP transport path works end-to-end, complementing the unit tests in the tool module.

## Current State

The project has an established integration test pattern used by `docker-push` and `docker-build`:

- Each tool binary has a `tests/<tool_name>_server_test.rs` file.
- Tests use `mcp_test_utils::spawn_mcp_client!` macro to spawn the binary as a child process and connect an MCP client over stdio transport.
- Tests use `mcp_test_utils::assert_single_tool` to verify tool name, description substring, and parameter names.
- Tests call `client.peer().call_tool(params)` and parse the text content as JSON to assert on fields.
- Every test ends with `client.cancel().await.expect("failed to cancel client")`.
- All tests use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- Parameters are constructed via `CallToolRequestParams::new("tool_name").with_arguments(serde_json::json!({...}).as_object().unwrap().clone())`.

## Requirements

- Create file `tools/register-agent/tests/register_agent_server_test.rs`.
- Import `rmcp::model::CallToolRequestParams` (the only direct rmcp import needed; `mcp_test_utils` re-exports the rest).
- Implement exactly three test functions:

1. **`tools_list_returns_register_agent_tool`** -- Spawns an MCP client connected to the `register-agent` binary. Calls `assert_single_tool` with name `"register_agent"`, a description substring related to registration (e.g., `"register"` or `"Register"`), and expected params `["name", "url", "description"]`.

2. **`tools_call_with_empty_name_returns_error`** -- Calls the tool with `name` set to `""`, `url` set to a valid placeholder (e.g., `"http://example.com"`), and `description` set to a non-empty string. Asserts the response parses as JSON with `success: false`.

3. **`tools_call_with_valid_inputs_returns_structured_json`** -- Calls the tool with valid inputs: a non-empty `name`, a valid `url`, and a non-empty `description`. Since the orchestrator is unreachable in CI, does NOT assert `success: true`. Instead asserts the response JSON has the expected shape: `success` field is present, `agent_name` field is present, and `registered_url` field is present.

## Implementation Details

### File structure

```rust
use rmcp::model::CallToolRequestParams;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_register_agent_tool() { ... }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_empty_name_returns_error() { ... }

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_call_with_valid_inputs_returns_structured_json() { ... }
```

### Test 1: `tools_list_returns_register_agent_tool`

- Spawn client: `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_register-agent")).await`
- Call `mcp_test_utils::assert_single_tool(&client, "register_agent", <description_substring>, &["name", "url", "description"]).await`
- The description substring should be a word like `"egister"` or `"Register"` that will appear in the tool description. Check the actual description set in `register_agent.rs` when implementing.
- Cancel client.

### Test 2: `tools_call_with_empty_name_returns_error`

- Spawn client.
- Build params: `CallToolRequestParams::new("register_agent").with_arguments(serde_json::json!({"name": "", "url": "http://example.com", "description": "A test agent"}).as_object().unwrap().clone())`
- Call `client.peer().call_tool(params).await.expect("call_tool")`
- Extract first content item as text, parse as `serde_json::Value`.
- Assert `json["success"] == false`.
- Cancel client.

### Test 3: `tools_call_with_valid_inputs_returns_structured_json`

- Spawn client.
- Build params with valid values: `{"name": "test-agent", "url": "http://localhost:9999", "description": "A test agent"}`
- Call tool, extract JSON.
- Assert `json.get("success").is_some()` -- field present.
- Assert `json.get("agent_name").is_some()` -- field present.
- Assert `json.get("registered_url").is_some()` -- field present.
- Do NOT assert the value of `success` since the orchestrator will be unreachable. The response will likely be `success: false` with an error message, but we only care about the JSON shape.
- Cancel client.

### Using mcp-test-utils

The `Cargo.toml` dev-dependencies should already include `mcp-test-utils` (set up by the scaffolding task). The macro `spawn_mcp_client!` and function `assert_single_tool` are the two main utilities. No other helpers are needed for these tests.

### JSON extraction pattern (reused across tests 2 and 3)

```rust
let text = result
    .content
    .first()
    .expect("should have content")
    .as_text()
    .expect("first content should be text");
let json: serde_json::Value = serde_json::from_str(&text.text).expect("should parse as JSON");
```

## Dependencies

- **Blocked by:** "Create main.rs entry point" -- the binary must exist for `env!("CARGO_BIN_EXE_register-agent")` to resolve.
- **Blocking:** None

## Risks & Edge Cases

- **Description substring mismatch:** The `assert_single_tool` call requires a substring that appears in the tool's description. The implementer must check what description string is set in `register_agent.rs` and use a matching substring. Using a short, common word like `"egister"` reduces fragility.
- **Orchestrator unreachable in CI:** Test 3 deliberately avoids asserting `success: true` because the orchestrator endpoint will not be running in CI. It only checks JSON shape. If the tool implementation hangs waiting for a connection (e.g., no timeout on the HTTP request), the test will time out. The tool implementation should use a reasonable timeout on the reqwest client (covered by the tool logic spec, not this spec).
- **Binary name:** The Cargo binary name is derived from the package name `register-agent`, so the env var is `CARGO_BIN_EXE_register-agent`. If the `Cargo.toml` uses a different `[[bin]]` name, this must be adjusted.
- **Flaky cancel:** All existing tests call `client.cancel().await` at the end. If a test panics before cancel, the child process may linger. This is consistent with existing patterns and acceptable.

## Verification

- `cargo test -p register-agent` passes with all three tests green.
- `cargo test -p register-agent tools_list_returns_register_agent_tool` confirms the tool is discoverable with correct name and params.
- `cargo test -p register-agent tools_call_with_empty_name_returns_error` confirms validation rejects empty name.
- `cargo test -p register-agent tools_call_with_valid_inputs_returns_structured_json` confirms response JSON structure is correct even when orchestrator is unreachable.
