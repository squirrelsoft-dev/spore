# Spec: Migrate read-file integration tests

> From: .claude/tasks/issue-56.md

## Objective

Replace the hand-rolled `spawn_read_file_client` helper in the read-file integration tests with the shared `spawn_mcp_client!` macro from `mcp-test-utils`, and consolidate the three `tools_list_*` tests into a single test that calls `assert_single_tool`. Keep the tool-specific call tests (`tools_call_read_file_returns_content` and `tools_call_read_file_returns_error_for_missing_file`) as-is, only updating them to use the macro for client creation.

## Current State

- **`tools/read-file/tests/read_file_server_test.rs`** contains 5 integration tests with a local `spawn_read_file_client()` async function that duplicates the same spawn-and-connect boilerplate found in echo-tool, write-file, and validate-skill.
- Three of those tests (`tools_list_returns_read_file_tool`, `tools_list_read_file_has_correct_description`, `tools_list_read_file_has_path_parameter`) each independently spawn a client, call `list_tools`, and assert one property. This pattern is identical across all 4 tool crates.
- **`crates/mcp-test-utils`** will provide:
  - `spawn_mcp_client!` macro: accepts a binary path expression and returns `RunningService<RoleClient, ()>`.
  - `assert_single_tool` async function: takes a client reference, expected tool name, description substring, and list of expected parameter names; asserts all of these in one call.
- **`tools/read-file/Cargo.toml`** currently has `rmcp` (client, transport-child-process), `tokio`, and `serde_json` as dev-dependencies. It does not yet depend on `mcp-test-utils`.

## Requirements

1. **Add `mcp-test-utils` dev-dependency** to `tools/read-file/Cargo.toml`:
   ```toml
   [dev-dependencies]
   mcp-test-utils = { path = "../../crates/mcp-test-utils" }
   ```
   Keep existing dev-dependencies (`tokio`, `rmcp`, `serde_json`) since the call tests still use `CallToolRequestParams` and `serde_json::json!`.

2. **Remove `spawn_read_file_client` function** from `read_file_server_test.rs`. Replace all usages with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_read-file"))`.

3. **Remove the three `tools_list_*` tests:**
   - `tools_list_returns_read_file_tool`
   - `tools_list_read_file_has_correct_description`
   - `tools_list_read_file_has_path_parameter`

4. **Add a single consolidated list test** that replaces the three removed tests:
   ```rust
   #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
   async fn tools_list_returns_read_file_tool() {
       let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_read-file"));
       mcp_test_utils::assert_single_tool(
           &client,
           "read_file",
           "Read the contents of a file",
           &["path"],
       )
       .await;
       client.cancel().await.expect("failed to cancel client");
   }
   ```

5. **Update `tools_call_read_file_returns_content`** to use the macro for client creation instead of `spawn_read_file_client().await`. The rest of the test body (temp file creation, `CallToolRequestParams`, assertions, cleanup) remains unchanged.

6. **Update `tools_call_read_file_returns_error_for_missing_file`** to use the macro for client creation. The rest of the test body remains unchanged.

7. **Remove unused imports** that were only needed by the deleted spawn helper or removed list tests. After migration, the imports should be:
   ```rust
   use rmcp::model::CallToolRequestParams;
   ```
   The `RunningService`, `TokioChildProcess`, `RoleClient`, `ServiceExt`, and `tokio::process::Command` imports should be removed since the macro handles those internally.

## Implementation Details

### File to modify: `tools/read-file/tests/read_file_server_test.rs`

The file should go from 5 tests + 1 helper function down to 3 tests + 0 helper functions:

| Before | After |
|--------|-------|
| `spawn_read_file_client()` helper | Removed (replaced by `spawn_mcp_client!` macro) |
| `tools_list_returns_read_file_tool` | Consolidated into single test using `assert_single_tool` |
| `tools_list_read_file_has_correct_description` | Removed (covered by `assert_single_tool`) |
| `tools_list_read_file_has_path_parameter` | Removed (covered by `assert_single_tool`) |
| `tools_call_read_file_returns_content` | Kept, updated to use macro |
| `tools_call_read_file_returns_error_for_missing_file` | Kept, updated to use macro |

### File to modify: `tools/read-file/Cargo.toml`

Add `mcp-test-utils` path dependency under `[dev-dependencies]`. Do not remove existing dev-dependencies that are still used by the call tests.

### No other files created or modified

This task only modifies the two files listed above.

## Dependencies

- **Blocked by:**
  - "Add `assert_single_tool` helper to `mcp-test-utils`" (Group 2, issue #56) -- the `assert_single_tool` function must exist.
  - "Create `crates/mcp-test-utils` crate with `spawn_mcp_client!` macro" (Group 1, issue #56) -- the macro must exist.
- **Blocking:**
  - "Run verification suite" (Group 5, issue #56)

## Risks & Edge Cases

1. **Description substring sensitivity:** The `assert_single_tool` call checks that the tool description contains `"Read the contents of a file"`. If the tool's description is changed, this test will fail. This matches the current assertion in the existing test.

2. **Macro re-export visibility:** The `spawn_mcp_client!` macro must be exported from `mcp_test_utils` via `#[macro_export]`. The calling test file uses the fully qualified `mcp_test_utils::spawn_mcp_client!` path.

3. **Import cleanup:** If any removed import is still needed by the remaining call tests, the build will catch it immediately. The `CallToolRequestParams` import must be kept for the two remaining call tests.

4. **Temp file cleanup in call test:** The `tools_call_read_file_returns_content` test creates a temp file but does not clean it up. This matches the current behavior and is acceptable for integration tests.

## Verification

1. `cargo test -p read-file` compiles and all 3 integration tests pass.
2. `cargo clippy -p read-file --tests` reports no warnings.
3. No `spawn_read_file_client` function remains in the codebase.
4. The `tools_list_returns_read_file_tool` test validates name, description, and parameters in a single test via `assert_single_tool`.
5. The two `tools_call_*` tests are functionally unchanged (same assertions, same temp file behavior).
