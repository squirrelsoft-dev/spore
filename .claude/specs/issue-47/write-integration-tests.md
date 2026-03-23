# Spec: Write integration tests

> From: .claude/tasks/issue-47.md

## Objective

Create integration tests for the `cargo-build` MCP tool in `tools/cargo-build/tests/cargo_build_server_test.rs`. The tests verify the tool's listing metadata, successful builds, error handling for nonexistent packages, and input validation for malicious package names.

## Current State

No `tools/cargo-build/` directory exists yet. The tool and its `main.rs` are prerequisites (blocked by "Write `main.rs`"). The test file will be created once the binary crate is in place.

## Requirements

1. **File location:** `tools/cargo-build/tests/cargo_build_server_test.rs`
2. **Binary reference:** Use `env!("CARGO_BIN_EXE_cargo-build")` to locate the compiled server binary.
3. **Test runtime:** Every test function must use `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`.
4. **Four test cases** (details in Implementation Details below).

## Implementation Details

Follow the pattern established in `tools/echo-tool/tests/echo_server_test.rs`:
- Spawn the MCP client via `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_cargo-build"))`.
- Use `rmcp::model::CallToolRequestParams` for tool invocations.
- Cancel the client at the end of each test with `client.cancel().await.expect("failed to cancel client")`.

### Test 1: `tools_list_returns_cargo_build_tool`

- Call `mcp_test_utils::assert_single_tool` with:
  - `expected_name`: `"cargo_build"`
  - `description_contains`: `"cargo build"` (case-sensitive substring match)
  - `expected_params`: `&["package", "release"]`
- No tool invocation needed; listing validation only.

### Test 2: `tools_call_builds_echo_tool_successfully`

- Create `CallToolRequestParams::new("cargo_build")` with arguments `{"package": "echo-tool"}`.
- Call the tool via `client.peer().call_tool(params).await`.
- Extract the first content element as text.
- Parse the text as JSON (`serde_json::from_str`).
- Assert `json["success"]` is `true`.
- Assert `json["exit_code"]` is `0`.

### Test 3: `tools_call_returns_error_for_nonexistent_package`

- Create `CallToolRequestParams::new("cargo_build")` with arguments `{"package": "nonexistent-package-xyz"}`.
- Call the tool and extract text from the first content element.
- Parse the text as JSON.
- Assert `json["success"]` is `false`.
- Assert `json["stderr"]` is a non-empty string (use `.as_str().map(|s| !s.is_empty())`).

### Test 4: `tools_call_rejects_invalid_package_name`

- Create `CallToolRequestParams::new("cargo_build")` with arguments `{"package": "foo;bar"}`.
- Call the tool and inspect the response.
- Assert the response indicates an error. This could manifest as:
  - `result.is_error` being `Some(true)`, or
  - The text content containing an error message about invalid package name.
- The key invariant: the tool must reject `foo;bar` **before** spawning any `cargo` subprocess (shell-injection prevention).

## Dependencies

- **Crate dependencies** (in `tools/cargo-build/Cargo.toml` under `[dev-dependencies]`):
  - `tokio` (with `macros`, `rt-multi-thread` features)
  - `rmcp`
  - `serde_json`
  - `mcp-test-utils` (path = `../../crates/mcp-test-utils`)
- **Blocked by:** The `cargo-build` binary (`main.rs`) must exist and compile before these tests can run.

## Risks & Edge Cases

- **Test 2 build time:** Building `echo-tool` from within a test may be slow; no timeout override is specified, so the default `tokio::test` timeout applies. If CI is slow, this test may need a longer timeout or `#[ignore]` annotation. Start without either and adjust if needed.
- **Workspace root assumption:** The `cargo build -p echo-tool` command in Test 2 assumes the working directory or `--manifest-path` resolves to the workspace root. The tool implementation must handle this; the test just validates the contract.
- **Error format in Test 4:** The exact error shape depends on whether validation happens at the MCP schema level (rmcp rejects it) or at the tool handler level (tool returns an error response). The test should accept either `is_error == true` on the result or an error message in the text content.

## Verification

1. `cargo test -p cargo-build` passes all four tests.
2. Each test spawns and cleanly shuts down the MCP server process (no zombie processes).
3. Test 2 produces a successful build of `echo-tool`.
4. Test 3 confirms failure output with non-empty stderr.
5. Test 4 confirms `foo;bar` is rejected without shell execution.
