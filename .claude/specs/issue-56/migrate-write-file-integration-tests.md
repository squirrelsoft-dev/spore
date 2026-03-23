# Spec: Migrate write-file integration tests

> From: .claude/tasks/issue-56.md

## Objective

Replace duplicated test boilerplate in the `write-file` crate with shared utilities from `mcp-test-utils`. This covers three changes: (1) replace the hand-rolled `spawn_write_file_client()` helper with the `spawn_mcp_client!` macro, (2) consolidate the three `tools_list_*` integration tests into a single test using `assert_single_tool`, and (3) replace the local `unique_temp_dir` function in the unit tests with `mcp_test_utils::unique_temp_dir`. Add `mcp-test-utils` as a dev-dependency.

## Current State

- **`tools/write-file/tests/write_file_server_test.rs`** contains:
  - A private `spawn_write_file_client()` async function (lines 10-17) that spawns the binary via `TokioChildProcess` and returns `RunningService<RoleClient, ()>`. This is identical in structure to the helpers in the other 3 tool crates.
  - Three `tools_list_*` tests (`tools_list_returns_write_file_tool`, `tools_list_write_file_has_correct_description`, `tools_list_write_file_has_path_and_content_parameters`) that each independently spawn a client, call `list_tools`, assert one property, and tear down. These can be collapsed into one test.
  - One tool-specific call test (`tools_call_write_file_creates_file`) that must be preserved as-is (aside from replacing the spawn helper).

- **`tools/write-file/src/write_file.rs`** contains a local `unique_temp_dir(test_name: &str) -> PathBuf` function (lines 77-85) inside `#[cfg(test)] mod tests`. It creates a directory under `env::temp_dir().join("write_file_tests")`. This is the same function being extracted to `mcp-test-utils` (with the prefix changed to `spore_tests`).

- **`tools/write-file/Cargo.toml`** has dev-dependencies for `tokio`, `rmcp` (client features), and `serde_json`, but does not yet reference `mcp-test-utils`.

## Requirements

### 1. Add `mcp-test-utils` dev-dependency

Add the following to `tools/write-file/Cargo.toml` under `[dev-dependencies]`:

```toml
mcp-test-utils = { path = "../../crates/mcp-test-utils" }
```

### 2. Replace `spawn_write_file_client()` with `spawn_mcp_client!` macro

In `tools/write-file/tests/write_file_server_test.rs`:

- Remove the `spawn_write_file_client()` function entirely (lines 9-17).
- Remove the now-unnecessary imports: `transport::TokioChildProcess`, `ServiceExt`, `tokio::process::Command`. Keep `rmcp::{model::CallToolRequestParams, service::RunningService, RoleClient}` since they are still used by the call test.
- At each call site, replace `spawn_write_file_client().await` with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_write-file"))`.

### 3. Consolidate three `tools_list_*` tests into one

Replace the three separate tests (`tools_list_returns_write_file_tool`, `tools_list_write_file_has_correct_description`, `tools_list_write_file_has_path_and_content_parameters`) with a single test:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_returns_write_file_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_write-file"));

    mcp_test_utils::assert_single_tool(
        &client,
        "write_file",
        "Write content to a file",
        &["path", "content"],
    )
    .await;

    client.cancel().await.expect("failed to cancel client");
}
```

This single test validates all three properties (tool name, description substring, parameter names) that were previously spread across three tests.

### 4. Update the call test to use the macro

The `tools_call_write_file_creates_file` test must be preserved with its existing logic, but replace the `spawn_write_file_client().await` call with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_write-file"))`.

### 5. Replace local `unique_temp_dir` in unit tests

In `tools/write-file/src/write_file.rs`, inside the `#[cfg(test)] mod tests` block:

- Remove the local `unique_temp_dir` function (lines 77-85).
- Remove `use std::env;` if it is no longer used after the removal.
- Replace all calls to `unique_temp_dir("...")` with `mcp_test_utils::unique_temp_dir("...")`.

The five unit tests that call `unique_temp_dir` are: `write_file_creates_file_with_content`, `write_file_creates_parent_directories`, `write_file_returns_byte_count`, `write_file_overwrites_existing`, `write_file_preserves_unicode`. The test `write_file_empty_path` does not use temp dirs and needs no change.

Note: the shared `unique_temp_dir` uses the prefix `spore_tests` instead of `write_file_tests`. This is an intentional generalization and does not affect test behavior.

## Implementation Details

### File: `tools/write-file/Cargo.toml`

Add under `[dev-dependencies]`:

```toml
mcp-test-utils = { path = "../../crates/mcp-test-utils" }
```

### File: `tools/write-file/tests/write_file_server_test.rs`

The final file should contain:
- Imports for `rmcp::{model::CallToolRequestParams, service::RunningService, RoleClient}` (no `TokioChildProcess`, `ServiceExt`, or `tokio::process::Command`).
- Two tests total: `tools_list_returns_write_file_tool` (consolidated) and `tools_call_write_file_creates_file` (preserved logic, updated spawn).
- No `spawn_write_file_client` function.

### File: `tools/write-file/src/write_file.rs`

In the `#[cfg(test)] mod tests` block:
- Remove the `unique_temp_dir` function definition.
- Remove `use std::env;` (no longer needed; `std::fs` is still used).
- All five temp-dir-using tests call `mcp_test_utils::unique_temp_dir(...)` instead.

## Dependencies

- **Blocked by:**
  - "Add `assert_single_tool` helper" -- the `assert_single_tool` function must exist in `mcp-test-utils` before the consolidated list test can compile.
  - "Add `unique_temp_dir` helper" -- the shared `unique_temp_dir` function must exist in `mcp-test-utils` before the unit tests can compile.
- **Blocking:**
  - "Run verification suite"

## Risks & Edge Cases

1. **Temp dir prefix change:** The shared helper uses `spore_tests` instead of `write_file_tests`. If any CI cleanup scripts or other tooling references the old prefix, they need updating. In practice this is unlikely since the old prefix was only used locally in this test module.

2. **Import cleanup:** After removing `spawn_write_file_client`, ensure `TokioChildProcess`, `ServiceExt`, and `tokio::process::Command` are fully removed. The macro handles these internally.

3. **`mcp-test-utils` as dev-dependency for unit tests:** In Rust, `dev-dependencies` are available to both integration tests (`tests/`) and unit tests (`#[cfg(test)]` in `src/`), so adding `mcp-test-utils` once in `[dev-dependencies]` covers both use cases.

4. **Macro invocation syntax:** `spawn_mcp_client!` is a macro, not a function. Ensure the call uses `!` and passes the `env!()` expression directly (not as a variable), since `env!("CARGO_BIN_EXE_write-file")` must resolve at compile time in the calling crate.

5. **Test name preservation:** The consolidated list test retains the name `tools_list_returns_write_file_tool` for continuity. The two removed test names (`tools_list_write_file_has_correct_description`, `tools_list_write_file_has_path_and_content_parameters`) are intentionally dropped since their assertions are covered by the consolidated test.

## Verification

1. `cargo test -p write-file` passes -- both integration tests and all six unit tests.
2. `cargo clippy -p write-file --tests` reports no warnings.
3. No `spawn_write_file_client` function exists in the codebase.
4. No `unique_temp_dir` function exists in `tools/write-file/src/write_file.rs`.
5. `tools/write-file/tests/write_file_server_test.rs` contains exactly 2 test functions.
6. `tools/write-file/src/write_file.rs` unit test module references `mcp_test_utils::unique_temp_dir` (not a local definition).
