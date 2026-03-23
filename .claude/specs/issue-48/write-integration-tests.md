# Spec: Write integration tests

> From: .claude/tasks/issue-48.md

## Objective

Create integration tests for the `docker-build` MCP tool binary that verify tool discovery, input validation (path traversal and tag injection), and graceful degradation when Docker is unavailable. These tests exercise the full MCP server process over stdio transport, matching the project's established integration test pattern.

## Current State

The project has a well-established integration test pattern used across all tool crates:

- **`crates/mcp-test-utils/src/lib.rs`** provides:
  - `spawn_mcp_client!($bin_path)` macro: spawns the MCP server binary as a child process via `TokioChildProcess`, connects an `rmcp` client, and returns a `RunningService<RoleClient, ()>`.
  - `assert_single_tool(client, name, description_contains, expected_params)`: asserts the server exposes exactly one tool with matching name, description substring, and parameter names.
  - `unique_temp_dir(test_name)`: creates isolated temp directories for tests (not needed here).

- **Existing test files follow an identical structure** (see `tools/cargo-build/tests/cargo_build_server_test.rs`, `tools/echo-tool/tests/echo_server_test.rs`):
  - Single `use rmcp::model::CallToolRequestParams;` import at top.
  - Each test is `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
  - Client is spawned via `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_<crate-name>")).await`.
  - Tool calls use `CallToolRequestParams::new("<tool_name>").with_arguments(serde_json::json!({...}).as_object().unwrap().clone())`.
  - Response text is extracted via `result.content.first().expect(...).as_text().expect(...)`.
  - JSON is parsed via `serde_json::from_str::<serde_json::Value>(&text.text)`.
  - Assertions check `json["success"]`, `json["stderr"]`, etc.
  - Each test ends with `client.cancel().await.expect("failed to cancel client");`.

- The `docker-build` crate does not yet exist. This spec depends on the `main.rs` and `docker_build.rs` implementation being complete first.

## Requirements

1. **File location**: `tools/docker-build/tests/docker_build_server_test.rs`.

2. **Test: `tools_list_returns_docker_build_tool`**
   - Spawn the `docker-build` binary via `spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build"))`.
   - Call `mcp_test_utils::assert_single_tool` with:
     - `expected_name`: `"docker_build"`
     - `description_contains`: `"Docker image"`
     - `expected_params`: `["context", "tag", "build_args", "dockerfile"]`
   - Cancel the client.

3. **Test: `tools_call_rejects_path_traversal`**
   - Spawn the client.
   - Call `docker_build` with arguments `{"context": "../../etc", "tag": "test:latest"}`.
   - Parse response content as JSON.
   - Assert `json["success"] == false`.
   - Assert the response text contains a validation-related error message (e.g., check that `json["build_log"]` or `json["stderr"]` contains a substring like `"validation"`, `"path traversal"`, `"invalid"`, or similar -- the exact field and message depends on the `docker_build.rs` implementation, so the assertion should be flexible enough to match any reasonable validation error wording).
   - Cancel the client.

4. **Test: `tools_call_rejects_invalid_tag`**
   - Spawn the client.
   - Call `docker_build` with arguments `{"context": ".", "tag": "test;evil"}`.
   - Parse response content as JSON.
   - Assert `json["success"] == false`.
   - Cancel the client.

5. **Test: `tools_call_returns_error_when_docker_unavailable`**
   - Spawn the client.
   - Call `docker_build` with valid arguments `{"context": ".", "tag": "test:latest"}`.
   - Parse response content as JSON.
   - Assert `json["success"] == false` (Docker is not expected to be available in CI).
   - Assert the response contains a meaningful error message (non-empty `build_log` or error field).
   - This test validates graceful degradation: the tool must return well-formed JSON even when Docker is missing, rather than crashing or returning unparseable output.
   - Cancel the client.

6. **All tests must use** `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.

7. **Imports**: Only `use rmcp::model::CallToolRequestParams;` at the top, consistent with existing tests. The `mcp_test_utils` and `serde_json` crates are accessed via their fully qualified paths.

## Implementation Details

### File to create

**`tools/docker-build/tests/docker_build_server_test.rs`**

Structure (pseudocode outline):

```
use rmcp::model::CallToolRequestParams;

// Test 1: tools_list_returns_docker_build_tool
//   - spawn client
//   - assert_single_tool("docker_build", "Docker image", &["context", "tag", "build_args", "dockerfile"])
//   - cancel client

// Test 2: tools_call_rejects_path_traversal
//   - spawn client
//   - call docker_build with {"context": "../../etc", "tag": "test:latest"}
//   - parse JSON from response text
//   - assert success == false
//   - assert error message is present and indicates validation failure
//   - cancel client

// Test 3: tools_call_rejects_invalid_tag
//   - spawn client
//   - call docker_build with {"context": ".", "tag": "test;evil"}
//   - parse JSON from response text
//   - assert success == false
//   - cancel client

// Test 4: tools_call_returns_error_when_docker_unavailable
//   - spawn client
//   - call docker_build with {"context": ".", "tag": "test:latest"}
//   - parse JSON from response text
//   - assert success == false
//   - assert error/build_log field is non-empty
//   - cancel client
```

### Helper pattern for call + parse (repeated in tests 2-4)

Each tool-call test follows this exact sequence from `cargo_build_server_test.rs`:

```rust
let params = CallToolRequestParams::new("docker_build").with_arguments(
    serde_json::json!({ "context": "...", "tag": "..." })
        .as_object()
        .unwrap()
        .clone(),
);
let result = client.peer().call_tool(params).await.expect("call_tool");
let text = result.content.first().expect("should have content")
    .as_text().expect("first content should be text");
let json: serde_json::Value = serde_json::from_str(&text.text).expect("should parse as JSON");
```

### JSON response field assumptions

The `docker_build.rs` implementation (per the task breakdown) returns JSON with at minimum:
- `success`: bool
- `build_log`: string (contains error details on failure)

Tests should assert on `json["success"]` directly. For error message content, assert that `build_log` (or whichever error field exists) is non-empty or contains a relevant substring. If the field name differs from what is expected, the test will fail clearly, making it easy to align.

## Dependencies

- **Blocked by**: "Write `main.rs`" (the binary must exist for `env!("CARGO_BIN_EXE_docker-build")` to resolve) and "Implement `DockerBuildTool` struct and handler" (the tool logic must be implemented for tests to exercise it).
- **Blocking**: None.
- **Crate dev-dependencies** (must be in `Cargo.toml`, handled by the "Create Cargo.toml" task): `mcp-test-utils`, `tokio` with `rt-multi-thread`, `rmcp` with `client`/`transport-child-process`, `serde_json`.

## Risks & Edge Cases

1. **JSON field naming mismatch**: The tests assume the response JSON uses `success` and `build_log` fields. If `docker_build.rs` uses different field names (e.g., `error`, `stderr`, `message`), assertions will need adjustment. Mitigation: the task breakdown specifies `success`, `build_log`, `image_id`, and `tag` as output fields.

2. **Validation error message wording**: The path traversal test should not assert an exact error string, since the wording may change. Use a loose check (e.g., non-empty error field, or `contains` with a broad term like `"path"` or `"invalid"` or `"traversal"`).

3. **Docker actually being available**: Test 4 assumes Docker is NOT installed in the CI environment. If Docker IS available, the test would pass with `success: true` instead of `false`. Mitigation: the test should accept either outcome -- if `success` is `true`, that is also valid (Docker happened to be present). Alternatively, the test can simply verify the response is well-formed JSON regardless of `success` value, and only check `success: false` behavior as a secondary assertion. The simplest approach is: assert the response parses as JSON and has a `success` field. If `success` is `false`, also assert `build_log` is non-empty.

4. **Binary name resolution**: The `env!("CARGO_BIN_EXE_docker-build")` macro requires the crate to produce a binary named `docker-build`. This is determined by the `[[bin]]` section or crate name in `Cargo.toml`.

## Verification

1. `cargo test -p docker-build --test docker_build_server_test` -- all four tests pass.
2. `cargo clippy -p docker-build` -- no warnings in the test file.
3. Each test independently spawns and tears down its own MCP client (no shared state).
4. Tests run in under 30 seconds each (no long-running Docker builds; validation failures and missing-Docker errors are fast).
