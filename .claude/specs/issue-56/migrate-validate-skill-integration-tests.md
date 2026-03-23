# Spec: Migrate validate-skill integration tests

> From: .claude/tasks/issue-56.md

## Objective

Replace duplicated test helpers and fixtures in the validate-skill integration and unit tests with shared utilities from `mcp-test-utils`. Specifically: (1) replace the local `spawn_validate_skill_client()` function with the `spawn_mcp_client!` macro, (2) consolidate the three `tools_list_*` integration tests into a single test using `assert_single_tool`, (3) replace both `valid_skill_content()` (integration tests) and `valid_content()` (unit tests) with `mcp_test_utils::valid_skill_content()`, and (4) add `mcp-test-utils` as a dev-dependency.

## Current State

### Integration tests (`tools/validate-skill/tests/validate_skill_server_test.rs`)

- Defines a local `spawn_validate_skill_client()` async function that creates a `TokioChildProcess` from `env!("CARGO_BIN_EXE_validate-skill")` and connects an MCP client. This is identical in structure to helpers in the other 3 tool test files.
- Defines a local `valid_skill_content()` function returning a canonical valid skill YAML frontmatter string. This is identical to the fixture that will live in `mcp-test-utils`.
- Contains three separate `tools_list_*` tests that each spawn a client, call `list_tools`, and assert on a single aspect:
  - `tools_list_returns_validate_skill_tool` — asserts exactly 1 tool named `"validate_skill"`
  - `tools_list_has_correct_description` — asserts the description contains `"Validate"`
  - `tools_list_has_content_parameter` — asserts `input_schema.properties` contains `"content"`
- Contains three tool-call tests that exercise the actual validation logic:
  - `tools_call_with_valid_skill_returns_valid_true`
  - `tools_call_with_missing_frontmatter_returns_valid_false`
  - `tools_call_with_invalid_yaml_returns_valid_false`

### Unit tests (`tools/validate-skill/src/validate_skill.rs`)

- Contains a `valid_content()` function inside `#[cfg(test)] mod tests` that returns the same canonical skill fixture as the integration test's `valid_skill_content()`.
- Used by the `valid_content_returns_success` unit test.

### Cargo.toml (`tools/validate-skill/Cargo.toml`)

- Dev-dependencies currently include `tokio`, `rmcp`, and `serde_json`. Does not include `mcp-test-utils`.

## Requirements

### 1. Add `mcp-test-utils` dev-dependency

Add the following to `tools/validate-skill/Cargo.toml` under `[dev-dependencies]`:

```toml
mcp-test-utils = { path = "../../crates/mcp-test-utils" }
```

### 2. Replace `spawn_validate_skill_client()` with macro

In `tools/validate-skill/tests/validate_skill_server_test.rs`:

- Remove the `spawn_validate_skill_client()` function entirely.
- Remove the direct imports of `rmcp::transport::TokioChildProcess` and `tokio::process::Command` (no longer needed).
- In every test that previously called `spawn_validate_skill_client().await`, replace with:
  ```rust
  let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill"));
  ```

### 3. Consolidate `tools_list_*` tests with `assert_single_tool`

- Remove all three tests: `tools_list_returns_validate_skill_tool`, `tools_list_has_correct_description`, `tools_list_has_content_parameter`.
- Replace with a single test:

```rust
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tools_list_validates_single_tool() {
    let client = mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_validate-skill"));

    mcp_test_utils::assert_single_tool(
        &client,
        "validate_skill",
        "Validate",
        &["content"],
    )
    .await;

    client.cancel().await.expect("failed to cancel client");
}
```

This single test covers all three prior assertions: tool count is 1, name matches `"validate_skill"`, description contains `"Validate"`, and `input_schema.properties` contains `"content"`.

### 4. Replace `valid_skill_content()` in integration tests

- Remove the local `valid_skill_content()` function from `validate_skill_server_test.rs`.
- In `tools_call_with_valid_skill_returns_valid_true`, replace `valid_skill_content()` with `mcp_test_utils::valid_skill_content()`.

### 5. Replace `valid_content()` in unit tests

In `tools/validate-skill/src/validate_skill.rs`:

- Remove the `valid_content()` function from `mod tests`.
- In `valid_content_returns_success`, replace `valid_content()` with `mcp_test_utils::valid_skill_content()`.
- The `call_validate` helper and all other unit tests remain unchanged.
- Since `mcp-test-utils` is a dev-dependency, it is available in `#[cfg(test)]` unit test modules.

### 6. Keep tool-call integration tests intact

The following three tests stay in the integration test file with only the spawn-helper change (macro replaces function call). Their assertions and logic remain unchanged:

- `tools_call_with_valid_skill_returns_valid_true`
- `tools_call_with_missing_frontmatter_returns_valid_false`
- `tools_call_with_invalid_yaml_returns_valid_false`

## Implementation Details

### Files to modify

- `tools/validate-skill/Cargo.toml` — add `mcp-test-utils` dev-dependency
- `tools/validate-skill/tests/validate_skill_server_test.rs` — replace spawn helper, consolidate list tests, replace fixture
- `tools/validate-skill/src/validate_skill.rs` — replace `valid_content()` in unit tests with shared fixture

### No new files to create

### Imports after migration (integration test file)

```rust
use rmcp::{
    model::CallToolRequestParams,
    service::RunningService,
    RoleClient, ServiceExt,
};
```

Note: `TokioChildProcess` and `tokio::process::Command` are no longer imported directly — the macro handles them internally.

### Expected test count change

- Before: 6 integration tests + 5 unit tests = 11 total
- After: 4 integration tests + 5 unit tests = 9 total
- The 3 list tests collapse into 1; the 3 call tests remain.

## Dependencies

- **Blocked by:** "Add `assert_single_tool` helper" (Group 2), "Add shared skill fixture" (Group 2) — both must exist in `mcp-test-utils` before this migration.
- **Blocking:** "Run verification suite" (Group 5)

## Risks & Edge Cases

- **Fixture content mismatch:** The shared `mcp_test_utils::valid_skill_content()` must return a string identical to the current local `valid_skill_content()` and `valid_content()`. If the shared fixture differs (e.g., different field values), unit test assertions like `assert_eq!(result["manifest"]["name"], "test-skill")` will fail. The shared fixture is defined as the canonical version; local assertions must match it.
- **`output.format` discrepancy:** The task notes mention that skill-loader's fixture uses `format: markdown` while validate-skill uses `format: json`. The shared fixture uses `json`, which matches the current validate-skill fixture, so no assertion changes are needed here.
- **Macro import path:** The `spawn_mcp_client!` macro is invoked as `mcp_test_utils::spawn_mcp_client!()`. Ensure the macro is `#[macro_export]`ed in the `mcp-test-utils` crate so it is available at the crate root path.
- **Dev-dependency scope:** `mcp-test-utils` as a dev-dependency is available in both integration tests and `#[cfg(test)]` unit test modules, so using it in `validate_skill.rs` unit tests is valid.

## Verification

- `cargo test -p validate-skill` passes (all 9 remaining tests pass).
- `cargo test -p validate-skill -- tools_list_validates_single_tool` passes.
- `cargo test -p validate-skill -- valid_content_returns_success` passes (unit test using shared fixture).
- `cargo clippy -p validate-skill` reports no new warnings.
- No local `spawn_validate_skill_client` function exists in the test file.
- No local `valid_skill_content` or `valid_content` fixture function exists in any validate-skill source file.
