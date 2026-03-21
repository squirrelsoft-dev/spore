# Spec: Run verification suite

> From: .claude/tasks/issue-15.md

## Objective

Run the full Rust verification pipeline (`cargo check`, `cargo clippy`, `cargo test`) across the entire workspace to confirm that all new orchestrator code compiles cleanly, passes linting, and passes tests -- and that no regressions have been introduced in the four existing crates (`agent-sdk`, `agent-runtime`, `skill-loader`, `tool-registry`).

This is the final gate task for issue-15. It must not run until every other Group 5 task (all four test suites) and all preceding groups are complete.

## Current State

- **Workspace root** (`/workspaces/spore/Cargo.toml`) defines six workspace members:
  - `crates/agent-sdk`
  - `crates/skill-loader`
  - `crates/tool-registry`
  - `crates/agent-runtime`
  - `crates/orchestrator`
  - `tools/echo-tool`
- **Existing crates** have integration tests in their respective `tests/` directories:
  - `agent-sdk`: `envelope_types_test.rs`, `micro_agent_test.rs`, `skill_manifest_test.rs`
  - `skill-loader`: `validation_test.rs`, `example_skills_test.rs`, `skill_loader_test.rs`, `validation_integration_test.rs`
  - `tool-registry`: `mcp_connection_test.rs`, `tool_entry_test.rs`, `tool_registry_test.rs`
  - `agent-runtime`: `constraint_enforcer_test.rs`, `http_test.rs`, `runtime_agent_test.rs`
- **Orchestrator crate** is currently a stub binary (`src/main.rs` with `println!("Hello, world!")`). By the time this task runs, it will have been converted to a library crate with four source modules (`lib.rs`, `error.rs`, `agent_endpoint.rs`, `config.rs`, `orchestrator.rs`) and four test files (`tests/error_test.rs`, `tests/agent_endpoint_test.rs`, `tests/config_test.rs`, `tests/orchestrator_test.rs`).
- **Orchestrator Cargo.toml** currently has no dependencies. By the time this task runs, it will have been updated with dependencies on `agent-sdk`, `reqwest`, `tokio`, `serde`, `serde_json`, `serde_yaml`, `async-trait`, and dev-dependencies `tokio` (with macros) and `axum`.

## Requirements

1. `cargo check` must pass with zero errors across all workspace members.
2. `cargo clippy` must pass with zero warnings across all workspace members (using default lint levels, no `--allow` suppression of legitimate warnings).
3. `cargo test` must pass with all tests green across all workspace members.
4. All pre-existing tests in `agent-sdk`, `agent-runtime`, `skill-loader`, and `tool-registry` must continue to pass without modification (no regressions).
5. All four new orchestrator test files must pass:
   - `crates/orchestrator/tests/error_test.rs`
   - `crates/orchestrator/tests/agent_endpoint_test.rs`
   - `crates/orchestrator/tests/config_test.rs`
   - `crates/orchestrator/tests/orchestrator_test.rs`
6. No compiler warnings in any crate (treat warnings as signals to fix, not suppress).

## Implementation Details

This is a verification-only task. No source files are created or modified. The implementation consists of running three commands sequentially and inspecting their output.

### Step 1: Type checking

```bash
cargo check --workspace 2>&1
```

- Confirm exit code 0.
- Scan output for any `error[E...]` lines -- there must be none.
- Note any warnings for resolution.

### Step 2: Linting

```bash
cargo clippy --workspace --all-targets 2>&1
```

- Confirm exit code 0.
- The `--all-targets` flag ensures clippy runs against library code, test code, benchmarks, and examples.
- Scan output for any `warning:` lines originating from workspace crates (ignore external dependency warnings if any).
- If warnings are found, they must be fixed in the relevant source files before this task can be marked complete.

### Step 3: Testing

```bash
cargo test --workspace 2>&1
```

- Confirm exit code 0.
- Verify output shows test results from all six workspace members.
- Specifically confirm the following test suites ran and passed:
  - **agent-sdk**: `envelope_types_test`, `micro_agent_test`, `skill_manifest_test`
  - **skill-loader**: `validation_test`, `example_skills_test`, `skill_loader_test`, `validation_integration_test`
  - **tool-registry**: `mcp_connection_test`, `tool_entry_test`, `tool_registry_test`
  - **agent-runtime**: `constraint_enforcer_test`, `http_test`, `runtime_agent_test`
  - **orchestrator**: `error_test`, `agent_endpoint_test`, `config_test`, `orchestrator_test`
- Confirm zero test failures and zero test panics.

### Handling failures

If any step fails:
1. Diagnose the root cause by reading the error output.
2. Identify which file(s) need to be fixed. If the failure is in new orchestrator code, fix those files. If the failure is in a pre-existing crate (regression), investigate whether the orchestrator changes caused the regression (e.g., a dependency conflict, a shared type change).
3. Re-run all three verification steps after any fix to confirm no cascading issues.
4. Do not suppress warnings with `#[allow(...)]` attributes unless the warning is a genuine false positive.

## Dependencies

- **Blocked by**: All other tasks in Group 5:
  - "Write unit tests for OrchestratorError"
  - "Write unit tests for AgentEndpoint"
  - "Write unit tests for Orchestrator dispatch and routing"
  - "Write unit tests for config loading"
  - (And transitively, all tasks in Groups 1-4)
- **Blocking**: None. This is the terminal task for issue-15.

## Risks & Edge Cases

1. **Clippy version sensitivity**: Different Rust toolchain versions may surface different clippy lints. The verification should use whatever `rustup` toolchain is configured in the workspace (currently edition 2024, implying Rust nightly or a very recent stable). If clippy produces new lints not present in older toolchains, they should be addressed rather than suppressed.

2. **Test isolation for env-based config tests**: The config tests modify environment variables (`AGENT_ENDPOINTS`, `AGENT_DESCRIPTIONS`). These tests must use a mutex or `serial_test` to avoid races. If `cargo test` runs tests in parallel and env-var tests interfere with each other, the fix is in the test code (use `std::sync::Mutex` or the `serial_test` crate), not in the verification step.

3. **Network-dependent tests**: The `agent_endpoint_test.rs` and `orchestrator_test.rs` tests spin up local `axum` servers on ephemeral ports. These should bind to `127.0.0.1:0` to avoid port conflicts. If tests fail due to port binding issues, the fix is in the test setup code.

4. **Flaky health-check tests**: Tests that rely on timing (e.g., health check timeouts) may be flaky. If a test passes locally but fails in CI, consider whether it needs a retry or a longer timeout.

5. **Edition 2024 compatibility**: The orchestrator Cargo.toml uses `edition = "2024"`. Ensure the Rust toolchain supports this edition. If not, the edition may need to be downgraded to `"2021"` in a prior task.

6. **Workspace lockfile consistency**: Adding new direct dependencies to the orchestrator crate may update `Cargo.lock`. Verify that `Cargo.lock` changes are limited to the new direct dependencies and do not unexpectedly upgrade versions for existing crates.

## Verification

This task IS the verification step for the entire issue-15 implementation. It is complete when:

1. `cargo check --workspace` exits with code 0 and zero errors.
2. `cargo clippy --workspace --all-targets` exits with code 0 and zero workspace warnings.
3. `cargo test --workspace` exits with code 0, all tests pass, and zero tests are ignored/skipped unexpectedly.
4. The output of `cargo test` explicitly shows passing results from all five crates' test suites (agent-sdk, skill-loader, tool-registry, agent-runtime, orchestrator).
5. No source files in `agent-sdk`, `agent-runtime`, `skill-loader`, or `tool-registry` were modified (confirming zero regressions requiring changes to existing code).
