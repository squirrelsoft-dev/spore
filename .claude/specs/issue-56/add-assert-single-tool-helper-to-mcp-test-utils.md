# Spec: Add `assert_single_tool` helper to `mcp-test-utils`

> From: .claude/tasks/issue-56.md

## Objective

Add a public async helper function `assert_single_tool` to the `crates/mcp-test-utils` crate that consolidates the three repeated `tools_list_*` integration tests (name check, description check, parameter check) into a single reusable call. Every MCP tool in the workspace currently has three near-identical tests that list tools and assert name, description, and input schema properties. This function replaces all of them with one call per tool.

## Current State

- The `crates/mcp-test-utils` crate exists (created by the prerequisite task "Create `crates/mcp-test-utils` crate") and contains the `spawn_mcp_client!` macro.
- Each of the tool crates (`echo-tool`, `write-file`, `validate-skill`, and `read-file`) has three duplicated test functions following this pattern:
  1. **Name test**: calls `client.peer().list_tools(None).await`, asserts `tools.len() == 1`, asserts `tools[0].name == expected_name`.
  2. **Description test**: calls `list_tools`, reads `tools[0].description`, asserts it contains a substring.
  3. **Parameters test**: calls `list_tools`, reads `tools[0].input_schema["properties"]`, asserts each expected parameter key exists.
- All three tests spawn a separate client, make the same `list_tools` call, and differ only in what they assert. The pattern is identical across all four tool crates.

## Requirements

- Add a public async function with this signature to `crates/mcp-test-utils/src/lib.rs`:
  ```rust
  pub async fn assert_single_tool(
      client: &RunningService<RoleClient, ()>,
      expected_name: &str,
      description_contains: &str,
      expected_params: &[&str],
  )
  ```
- The function must perform these assertions in order:
  1. Call `client.peer().list_tools(None).await` and unwrap with a clear message.
  2. Assert that `tools_result.tools.len() == 1` with the message `"expected exactly 1 tool"`.
  3. Assert that `tools_result.tools[0].name == expected_name`.
  4. Assert that the tool has a description (`tools[0].description` is `Some`) and that the description contains the `description_contains` substring. Use a descriptive panic message that includes the actual description on failure.
  5. Assert that `tools[0].input_schema` has a `"properties"` key.
  6. For each entry in `expected_params`, assert that the `"properties"` object contains that key. Use a descriptive panic message that includes the missing parameter name on failure.
- The function must NOT call `client.cancel()` -- that remains the caller's responsibility.
- The function must NOT spawn or manage the MCP client -- it receives a reference.
- No new external dependencies are required. The function uses types already depended on by `mcp-test-utils`: `rmcp::service::RunningService`, `rmcp::RoleClient`.

## Implementation Details

### File to modify

**`crates/mcp-test-utils/src/lib.rs`** -- append the function after the existing `spawn_mcp_client!` macro.

### Function body

The function should follow the exact assertion pattern from the existing tests. Specifically:

1. `list_tools` call:
   ```rust
   let tools_result = client.peer().list_tools(None).await.expect("list_tools failed");
   ```

2. Tool count and name:
   ```rust
   assert_eq!(tools_result.tools.len(), 1, "expected exactly 1 tool");
   assert_eq!(tools_result.tools[0].name, expected_name);
   ```

3. Description check:
   ```rust
   let description = tools_result.tools[0]
       .description
       .as_deref()
       .expect("tool should have a description");
   assert!(
       description.contains(description_contains),
       "expected description to contain '{}', got: {}",
       description_contains,
       description
   );
   ```

4. Parameter checks:
   ```rust
   let properties = tools_result.tools[0]
       .input_schema
       .get("properties")
       .expect("input_schema should have properties");
   for param in expected_params {
       assert!(
           properties.get(*param).is_some(),
           "input_schema properties should contain '{}'",
           param
       );
   }
   ```

### Required imports

The function needs these types in scope (some may already be imported for the macro):
- `rmcp::service::RunningService`
- `rmcp::RoleClient`

### How callers will use it

After migration, each tool's integration test file will replace its three `tools_list_*` tests with a single test like:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_validates_echo_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool"));
    mcp_test_utils::assert_single_tool(
        &client,
        "echo",
        "Returns the input message unchanged",
        &["message"],
    ).await;
    client.cancel().await.expect("failed to cancel client");
}
```

### Key design decisions

- **Async function, not macro**: Unlike `spawn_mcp_client!` which must be a macro (because `env!` resolves at compile time in the calling crate), `assert_single_tool` is a plain async function. It receives a reference to an already-constructed client, so no macro is needed.
- **Borrows client**: Takes `&RunningService` rather than consuming it, so callers can continue using the client for tool-call tests after the assertion and call `cancel()` when done.
- **Slice of param names**: Uses `&[&str]` for `expected_params` to handle tools with varying numbers of parameters (e.g., `echo-tool` has 1 param, `write-file` has 2).
- **No return value**: The function returns `()`. All checks use `assert!`/`assert_eq!` and panic on failure, matching the existing test convention.

### Files NOT modified by this task

- Tool test files (`tools/*/tests/*_server_test.rs`) -- those are modified by the Group 4 migration tasks.
- `crates/mcp-test-utils/Cargo.toml` -- no new dependencies needed; `rmcp` with `client` feature is already present from the macro task.
- `Cargo.toml` (workspace root) -- already updated by the prerequisite task.

## Dependencies

- Blocked by: "Create `crates/mcp-test-utils` crate with `spawn_mcp_client!` macro"
- Blocking: "Migrate echo-tool integration tests", "Migrate read-file integration tests", "Migrate write-file integration tests", "Migrate validate-skill integration tests"

## Risks & Edge Cases

- **`list_tools` return type changes**: The function accesses `tools_result.tools[0].input_schema` as a map-like type via `.get("properties")`. If `rmcp` changes the `input_schema` type in a future version, this will need updating. The current code matches the pattern used across all four existing test files.
- **Tools with zero or multiple parameters**: The `expected_params` slice handles this naturally -- an empty slice `&[]` would skip parameter checks, and a multi-element slice checks each one. All current tools have at least one parameter.
- **Description substring sensitivity**: The `description_contains` check is case-sensitive, matching the existing test behavior. Callers must pass a substring that matches the tool's actual description casing.

## Verification

- `cargo check -p mcp-test-utils` succeeds with no errors.
- `cargo clippy -p mcp-test-utils` produces no warnings.
- `cargo test -p mcp-test-utils` succeeds (the function itself has no unit tests -- it is exercised by the integration tests in the Group 4 migration tasks).
- The function signature matches the one specified in this spec exactly.
- The function is `pub` and accessible from external crates.
- No new dependencies are added to `crates/mcp-test-utils/Cargo.toml`.
