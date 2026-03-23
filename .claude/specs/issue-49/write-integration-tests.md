# Spec: Write integration tests in `tests/docker_push_server_test.rs`

> From: .claude/tasks/issue-49.md

## Objective

Create an integration test file that exercises the `docker-push` MCP server binary end-to-end over stdio transport, validating tool listing, input validation error paths, structured JSON responses, and empty-input rejection.

## Current State

- The file `tools/docker-push/tests/docker_push_server_test.rs` does not yet exist (the `tools/docker-push` crate itself does not exist yet).
- The reference implementation at `tools/cargo-build/tests/cargo_build_server_test.rs` demonstrates the established pattern: spawn the binary via `mcp_test_utils::spawn_mcp_client!`, interact through `client.peer()`, and tear down with `client.cancel().await`.
- The `mcp-test-utils` crate provides `spawn_mcp_client!` (macro) and `assert_single_tool` (async function) as the two primary test helpers.

## Requirements

### Test 1: `tools_list_returns_docker_push_tool`

- Spawn the binary using `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-push"))`.
- Call `mcp_test_utils::assert_single_tool(&client, "docker_push", "Push", &["image", "registry_url"])`.
- This validates: exactly one tool is exposed, its name is `"docker_push"`, its description contains `"Push"` (case-sensitive), and its input schema has exactly two properties `image` and `registry_url`.
- End with `client.cancel().await.expect("failed to cancel client")`.

### Test 2: `tools_call_with_invalid_image_returns_error`

- Spawn the binary.
- Construct `CallToolRequestParams::new("docker_push")` with arguments `{"image": "foo;bar"}`.
- Call `client.peer().call_tool(params).await`.
- Extract the first content element as text, parse it as `serde_json::Value`.
- Assert `json["success"] == false`.
- Assert `json["push_log"]` (as string) contains `"Invalid image reference"`.
- End with `client.cancel().await.expect("failed to cancel client")`.

### Test 3: `tools_call_with_valid_image_returns_structured_json`

- Spawn the binary.
- Construct `CallToolRequestParams::new("docker_push")` with arguments `{"image": "nonexistent-image:latest"}`.
- Call `client.peer().call_tool(params).await`.
- Extract text, parse as JSON.
- Assert that the JSON object contains all four expected fields: `success`, `image`, `digest`, `push_log`. Do NOT assert `success: true` because Docker daemon may not be available in CI, and the image does not exist regardless.
- End with `client.cancel().await.expect("failed to cancel client")`.

### Test 4: `tools_call_with_empty_image_returns_error`

- Spawn the binary.
- Construct `CallToolRequestParams::new("docker_push")` with arguments `{"image": ""}`.
- Call `client.peer().call_tool(params).await`.
- Extract text, parse as JSON.
- Assert `json["success"] == false`.
- End with `client.cancel().await.expect("failed to cancel client")`.

### Cross-cutting

- Every test function uses the attribute `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
- Every test ends with `client.cancel().await.expect("failed to cancel client");`.
- The file imports `rmcp::model::CallToolRequestParams` (needed by tests 2-4).
- No other imports are required; `mcp_test_utils` macros and functions are called via their full path.

## Implementation Details

### File location

`tools/docker-push/tests/docker_push_server_test.rs`

### Binary reference

Use `env!("CARGO_BIN_EXE_docker-push")` which Cargo sets automatically for integration tests in the same crate. This requires the `docker-push` binary target to exist in `tools/docker-push/Cargo.toml` and the `mcp-test-utils` dev-dependency to be declared.

### Pattern to mirror

Follow `tools/cargo-build/tests/cargo_build_server_test.rs` exactly in structure:
1. Single `use` statement for `rmcp::model::CallToolRequestParams`.
2. Each test is a standalone async function — no shared setup, no test fixtures.
3. Client is spawned fresh in each test for isolation.
4. Response text extraction follows the chain: `result.content.first().expect(...).as_text().expect(...)` then `serde_json::from_str(&text.text)`.
5. JSON field assertions use indexing (`json["field"]`) and compare with `==`.
6. String-contains assertions use `json["field"].as_str().unwrap_or("").contains(...)`.

### Assertion strategy for Test 3

Since Docker may not be installed or the daemon may not be running, and the image `nonexistent-image:latest` does not exist in any registry, the push will fail. The test only validates that the response is well-formed JSON with all four expected keys. Use `.get("field").is_some()` or equivalent to check key presence without asserting values.

### No new dependencies

The test file uses only `rmcp` (already a dev-dependency) and `mcp-test-utils` (already a dev-dependency). No additional crates needed.

## Dependencies

- **Blocked by**: "Write `main.rs`" — the binary must exist and compile before integration tests can spawn it.
- **Blocked by**: "Implement `DockerPushTool` struct and handler" — the tool handler must be implemented so the MCP server responds to tool calls.
- **Blocked by**: "Create `tools/docker-push/Cargo.toml`" — the crate manifest with dev-dependencies must exist.
- **Blocking**: "Run verification suite" — `cargo test -p docker-push` must pass these integration tests.

## Risks & Edge Cases

1. **Docker daemon availability**: Test 3 intentionally avoids asserting `success: true` because Docker may not be installed or running. Tests 2 and 4 exercise validation paths that reject the input before invoking Docker, so they work regardless of Docker availability.

2. **Binary name mapping**: The Cargo env macro `CARGO_BIN_EXE_docker-push` uses the hyphenated package name. If the package name in `Cargo.toml` changes, this macro breaks at compile time (which is the desired behavior — fail fast).

3. **Tool name vs binary name**: The MCP tool name is `docker_push` (snake_case, from the `#[tool_router]` attribute on the handler method). The binary name is `docker-push` (hyphenated). Tests must use the correct name in each context.

4. **Test isolation**: Each test spawns and tears down its own MCP client process. No shared state, no ordering dependency between tests.

5. **Response format contract**: Tests 2-4 depend on the response JSON having keys `success`, `image`, `digest`, `push_log`. If the handler implementation changes field names, these tests should fail and surface the breakage.

## Verification

After implementing this file, run:

```bash
cargo test -p docker-push
```

All four integration tests must pass. Additionally:

```bash
cargo clippy -p docker-push
```

must report no warnings in the test file.
