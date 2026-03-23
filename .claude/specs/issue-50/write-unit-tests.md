# Spec: Write unit tests

> From: .claude/tasks/issue-50.md

## Objective

Add a comprehensive `#[cfg(test)] mod tests` block inside `tools/register-agent/src/register_agent.rs` that covers input validation, error JSON structure, and HTTP POST logic against a lightweight mock server. This ensures the register_agent tool behaves correctly before integration tests run against the full MCP server.

## Current State

The codebase has two established unit-test patterns in sibling tools:

- **`tools/docker-push/src/docker_push.rs`** -- inline `#[cfg(test)] mod tests` with a `call_docker_push` helper that directly invokes the tool method via `Parameters(...)`. Tests cover: empty input rejected, shell metacharacters rejected, valid input produces correct JSON shape, helper function edge cases (digest extraction, registry URL resolution). All tests are synchronous `#[test]`.

- **`tools/docker-build/src/docker_build.rs`** -- same pattern with a `call_docker_build` helper. Tests cover: path traversal rejected, shell metacharacters in tag rejected, invalid build arg keys rejected, valid inputs produce correct JSON shape, helper function edge cases (image ID extraction, tag validation, metacharacter detection). All tests are synchronous `#[test]`.

Key differences for `register_agent`:
- The tool method will be **async** (it uses `reqwest` for HTTP POST), so tests must use `#[tokio::test]`.
- Tests need a **mock HTTP server** to verify POST behavior without a real orchestrator.
- The task file specifies `axum` or `tokio::net::TcpListener` for mocking (no `mockito`/`wiremock`).

## Requirements

- All tests live in `#[cfg(test)] mod tests` inside `register_agent.rs`.
- A `call_register_agent` async helper wraps `tool.register_agent(Parameters(...))` for convenience.
- Input validation tests (synchronous logic, but tool is async so use `#[tokio::test]`):
  - Empty `name` is rejected with `success: false` and error message containing "name".
  - Empty `url` is rejected with `success: false` and error message containing "url".
  - Empty `description` is rejected with `success: false` and error message containing "description".
  - `name` containing shell metacharacters (`;`, `|`, `$`, backtick) is rejected with `success: false`.
- Error JSON structure test:
  - On any validation failure, response parses as JSON with fields: `success` (false), `agent_name` (String), `registered_url` (empty string), `error` (non-empty string).
- Mock HTTP server tests:
  - Successful registration: mock returns 200 with a JSON body; tool returns `success: true`, correct `agent_name`, correct `registered_url`.
  - Orchestrator returns 4xx (e.g., 400 or 409): tool returns `success: false` with meaningful `error` string.
  - Orchestrator returns 5xx (e.g., 500): tool returns `success: false` with meaningful `error` string.
  - Orchestrator unreachable (connect to a port with nothing listening): tool returns `success: false` with an `error` string that indicates connection failure.

## Implementation Details

### Test helper

```rust
async fn call_register_agent(
    tool: &RegisterAgentTool,
    name: &str,
    url: &str,
    description: &str,
) -> String {
    tool.register_agent(Parameters(RegisterAgentRequest {
        name: name.to_string(),
        url: url.to_string(),
        description: description.to_string(),
    }))
    .await
}
```

### Mock server approach

Use `tokio::net::TcpListener` bound to `127.0.0.1:0` (OS-assigned port) to avoid port conflicts. For each test that needs a server:

1. Bind a `TcpListener` on port 0, extract the assigned port.
2. Spawn a `tokio::spawn` task that accepts one connection and writes a canned HTTP response.
3. Set the tool's orchestrator URL (or `ORCHESTRATOR_URL` env var) to `http://127.0.0.1:{port}`.
4. Call the tool and assert the response.
5. The spawned task completes after serving the single request.

Alternatively, if `axum` is already a dev-dependency (check Cargo.toml), use a minimal `axum::Router` with a single POST `/register` handler. This gives cleaner request parsing. Bind with `axum::Server::bind` on port 0.

For the "unreachable" test, find a free port, do NOT start a listener, and use that port as the orchestrator URL.

### Test cases

| Test name | Type | What it asserts |
|---|---|---|
| `rejects_empty_name` | Validation | `success: false`, error mentions "name" |
| `rejects_empty_url` | Validation | `success: false`, error mentions "url" |
| `rejects_empty_description` | Validation | `success: false`, error mentions "description" |
| `rejects_name_with_shell_metachar` | Validation | `success: false` for names like `"foo;bar"`, `"foo|bar"`, `"$(cmd)"` |
| `error_json_has_expected_structure` | Structure | Response has `success`, `agent_name`, `registered_url`, `error` fields |
| `successful_registration_returns_correct_json` | HTTP mock | Mock returns 200; assert `success: true`, `agent_name` matches, `registered_url` matches |
| `orchestrator_4xx_produces_error_json` | HTTP mock | Mock returns 400; assert `success: false`, `error` non-empty |
| `orchestrator_5xx_produces_error_json` | HTTP mock | Mock returns 500; assert `success: false`, `error` non-empty |
| `orchestrator_unreachable_produces_error_json` | HTTP mock | No listener; assert `success: false`, `error` mentions connection failure |

### Key assertions pattern

Follow the existing pattern from docker-push/docker-build:
```rust
let json: serde_json::Value = serde_json::from_str(&result).unwrap();
assert_eq!(json["success"], false);
assert!(json["error"].as_str().unwrap().contains("expected substring"));
```

### Environment variable handling

The tool reads `ORCHESTRATOR_URL` to know where to POST. Tests that use mock servers must set this env var or pass the URL through the request/tool constructor. Since env vars are global state and tests run in parallel, prefer one of:
- A constructor parameter on `RegisterAgentTool` that accepts an optional override URL.
- Or use `std::sync::Mutex` / serial test execution for env var manipulation.

The cleaner approach (constructor parameter) should be coordinated with the "Implement register_agent tool logic" task -- the tool should accept an optional `orchestrator_url` override in its constructor (e.g., `RegisterAgentTool::with_orchestrator_url(url)`) that takes precedence over the env var. This makes tests deterministic and parallel-safe.

## Dependencies

- Blocked by: "Implement register_agent tool logic" -- the `RegisterAgentTool`, `RegisterAgentRequest`, validation functions, and tool method must exist before tests can be written.
- Blocking: None

## Risks & Edge Cases

- **Async test runtime**: All tests must use `#[tokio::test]` since the tool method is async. Use `#[tokio::test(flavor = "current_thread")]` to keep tests lightweight.
- **Port conflicts**: Using port 0 for mock servers avoids conflicts. Each test gets its own listener.
- **Env var race conditions**: If the tool reads `ORCHESTRATOR_URL` from the environment, parallel tests that set different values will conflict. Mitigate by passing the URL via a constructor/method parameter rather than env vars in tests.
- **Mock server cleanup**: Spawned tasks must not leak. Either `.await` the join handle or ensure the task exits after one request.
- **No new dependencies**: `tokio::net::TcpListener` is already available via the `tokio` dependency. If using `axum`, confirm it is listed in dev-dependencies in the Cargo.toml (the scaffolding task should add it). If not, prefer raw `TcpListener` with hand-crafted HTTP responses to avoid adding a dependency.

## Verification

- `cargo test -p register-agent` passes with all tests green.
- `cargo test -p register-agent -- --list` shows all 9 test names listed above.
- `cargo clippy -p register-agent` produces no warnings.
- Each validation test confirms `success: false` and a meaningful error message.
- Each mock server test confirms the tool correctly interprets 200, 4xx, 5xx, and unreachable scenarios.
