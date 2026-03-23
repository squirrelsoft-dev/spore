# Spec: Migrate echo-tool integration tests

> From: .claude/tasks/issue-56.md

## Objective

Migrate `tools/echo-tool/tests/echo_server_test.rs` to use shared test utilities from `crates/mcp-test-utils`. Replace the local `spawn_echo_client()` helper with the `spawn_mcp_client!` macro, consolidate the three `tools_list_*` tests into a single test using `assert_single_tool`, and add `mcp-test-utils` as a dev-dependency. The two tool-specific call tests remain unchanged in logic.

## Current State

- **`tools/echo-tool/tests/echo_server_test.rs`** contains 5 integration tests and a local `spawn_echo_client()` helper function:
  1. `tools_list_returns_echo_tool` -- asserts exactly 1 tool named `"echo"`
  2. `tools_list_echo_has_correct_description` -- asserts description contains `"Returns the input message unchanged"`
  3. `tools_list_echo_has_message_parameter` -- asserts `input_schema.properties` contains `"message"`
  4. `tools_call_echo_returns_message` -- calls echo with `"hello"`, asserts response contains `"hello"`
  5. `tools_call_echo_preserves_unicode` -- calls echo with a unicode string, asserts exact round-trip preservation

- **`spawn_echo_client()`** is a local async function (lines 10-17) that creates a `TokioChildProcess` from `env!("CARGO_BIN_EXE_echo-tool")`, then calls `().serve(transport).await`. This exact pattern is duplicated across all 4 tool test files.

- **Three `tools_list_*` tests** (tests 1-3) each independently spawn a client, call `list_tools`, and assert one property. These three assertions are the same pattern used across all 4 tool test files.

- **`tools/echo-tool/Cargo.toml` `[dev-dependencies]`** currently contains:
  ```toml
  [dev-dependencies]
  tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }
  rmcp = { version = "1", features = ["client", "transport-child-process"] }
  serde_json = "1"
  ```

- **`crates/mcp-test-utils`** (created by a prior task) provides:
  - `spawn_mcp_client!($bin_path_expr)` -- a declarative macro that accepts a compile-time binary path expression (from `env!("CARGO_BIN_EXE_...")`) and returns `RunningService<RoleClient, ()>`. Encapsulates `TokioChildProcess::new` + `().serve(transport).await`.
  - `assert_single_tool(client, expected_name, description_contains, expected_params)` -- an async function that calls `list_tools`, asserts exactly 1 tool, asserts name matches, asserts description contains the given substring, and asserts each expected param exists in `input_schema.properties`.

## Requirements

1. **Remove `spawn_echo_client()` function.** Delete the entire local helper (lines 9-17 of the current file).

2. **Add `mcp-test-utils` dev-dependency.** Add `mcp-test-utils = { path = "../../crates/mcp-test-utils" }` to `[dev-dependencies]` in `tools/echo-tool/Cargo.toml`.

3. **Replace all `spawn_echo_client().await` calls** with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool"))`. This appears in every test function (5 occurrences total, reduced to 3 after consolidation).

4. **Consolidate the three `tools_list_*` tests into a single test.** Replace `tools_list_returns_echo_tool`, `tools_list_echo_has_correct_description`, and `tools_list_echo_has_message_parameter` with a single test:
   ```rust
   #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
   async fn tools_list_advertises_echo_tool() {
       let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool"));
       mcp_test_utils::assert_single_tool(
           &client,
           "echo",
           "Returns the input message unchanged",
           &["message"],
       )
       .await;
       client.cancel().await.expect("failed to cancel client");
   }
   ```

5. **Keep `tools_call_echo_returns_message` and `tools_call_echo_preserves_unicode` tests.** Update only the client spawning line to use the macro. All assertions and logic remain the same.

6. **Retain `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`** on all test functions, consistent with the current configuration.

7. **Update imports.** Remove `rmcp::service::RunningService`, `rmcp::RoleClient`, and `tokio::process::Command` since they are no longer needed directly. Keep `rmcp::model::CallToolRequestParams`, `rmcp::ServiceExt`, and `serde_json` for the call tests.

## Implementation Details

### File to modify: `tools/echo-tool/tests/echo_server_test.rs`

The resulting file should contain 3 tests (down from 5) and no local helper function:

| # | Test name | What it does |
|---|-----------|--------------|
| 1 | `tools_list_advertises_echo_tool` | Spawns client, calls `assert_single_tool` with name `"echo"`, description substring `"Returns the input message unchanged"`, and params `["message"]` |
| 2 | `tools_call_echo_returns_message` | Spawns client, calls echo with `{ "message": "hello" }`, asserts response text contains `"hello"` |
| 3 | `tools_call_echo_preserves_unicode` | Spawns client, calls echo with a unicode string, asserts exact text match |

### File to modify: `tools/echo-tool/Cargo.toml`

Add to `[dev-dependencies]`:

```toml
mcp-test-utils = { path = "../../crates/mcp-test-utils" }
```

The existing `rmcp`, `tokio`, and `serde_json` dev-dependencies remain because the call tests still use `CallToolRequestParams`, `serde_json::json!`, and `#[tokio::test]`.

### No other files created or modified

This task only modifies the integration test file and `Cargo.toml`. No source code changes.

## Dependencies

- **Blocked by:**
  - "Create `crates/mcp-test-utils` crate with `spawn_mcp_client!` macro" (Group 1, issue #56) -- the macro must exist.
  - "Add `assert_single_tool` helper to `mcp-test-utils`" (Group 2, issue #56) -- the helper function must exist.

- **Blocking:**
  - "Run verification suite" (Group 5, issue #56) -- verification depends on all migrations being complete and passing.

## Risks & Edge Cases

1. **`assert_single_tool` signature mismatch.** The exact signature of `assert_single_tool` (whether it takes `&RunningService<RoleClient, ()>` or uses a trait bound) must match how the macro returns the client. If the signature differs from what is described in the task breakdown, adjust the call accordingly.

2. **Macro import path.** The `spawn_mcp_client!` macro is invoked via `mcp_test_utils::spawn_mcp_client!`. If the crate re-exports the macro differently (e.g., requiring `use mcp_test_utils::spawn_mcp_client;`), adapt the import. Declarative macros exported with `#[macro_export]` live at the crate root.

3. **Description substring stability.** The consolidated test asserts `description_contains: "Returns the input message unchanged"`. If the echo tool's description changes, this test will fail. This is the same risk as the current `tools_list_echo_has_correct_description` test -- no new risk introduced.

4. **Test count reduction.** Going from 5 tests to 3 tests means that a failure in `assert_single_tool` will report as a single failure rather than pinpointing which of the three properties (name, description, params) failed. This is an acceptable trade-off because `assert_single_tool` should include descriptive assertion messages for each check internally.

5. **Unused imports after refactor.** Removing the local helper eliminates the need for `TokioChildProcess`, `Command`, `RunningService`, and `RoleClient` imports. Failing to clean up imports will cause `cargo clippy` warnings.

## Verification

1. `cargo test -p echo-tool --test echo_server_test` compiles and all 3 test functions pass.
2. `cargo clippy -p echo-tool --tests` reports no warnings.
3. The `spawn_echo_client` function no longer exists in the file.
4. The three `tools_list_returns_echo_tool`, `tools_list_echo_has_correct_description`, and `tools_list_echo_has_message_parameter` tests no longer exist.
5. The `tools_call_echo_returns_message` and `tools_call_echo_preserves_unicode` tests still pass with identical assertion logic.
6. `mcp-test-utils` appears in `[dev-dependencies]` of `tools/echo-tool/Cargo.toml`.
7. `cargo test` across the full workspace still passes (no regressions).
