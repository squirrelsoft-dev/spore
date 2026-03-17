# Spec: Run verification suite

> From: .claude/tasks/issue-9.md

## Objective

Run the full workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that all code introduced by the issue-9 task groups compiles cleanly, passes linting, and has passing tests. This task also verifies that the new `rmcp 0.16` and `rig-core 0.32` dependencies resolve without version conflicts in the Cargo dependency graph.

This is the final gate before the issue-9 branch can be considered complete.

## Current State

The workspace has five crates defined in the root `Cargo.toml`:

- `crates/agent-sdk` -- shared types (SkillManifest, AgentError, etc.), depends on serde/schemars/async-trait
- `crates/skill-loader` -- markdown frontmatter parsing and validation, depends on agent-sdk and tool-registry
- `crates/tool-registry` -- currently a stub (`pub struct ToolRegistry;`), no dependencies yet
- `crates/agent-runtime` -- currently a `println!("Hello, world!")` stub, no dependencies yet
- `crates/orchestrator` -- stub, no dependencies

By the time this task runs, the preceding issue-9 tasks will have:

1. Added `rmcp 0.16` and `tokio` to `tool-registry` dependencies
2. Created `mcp_handle.rs`, `transport.rs`, and expanded `tool_registry.rs` and `tool_entry.rs` in tool-registry
3. Added `rig-core 0.32`, `rmcp 0.16`, and workspace-path dependencies to `agent-runtime`
4. Created `tool_bridge.rs` and updated `main.rs` in agent-runtime
5. Added unit tests in `transport.rs` and integration tests in `tests/mcp_connection_test.rs`

Existing tests that must continue to pass:

- `crates/agent-sdk/tests/envelope_types_test.rs`
- `crates/agent-sdk/tests/micro_agent_test.rs`
- `crates/agent-sdk/tests/skill_manifest_test.rs`
- `crates/skill-loader/tests/skill_loader_test.rs`
- `crates/skill-loader/tests/validation_integration_test.rs`
- `crates/skill-loader/tests/validation_test.rs`

## Requirements

- `cargo check` succeeds with zero compiler errors across all workspace members
- `cargo clippy` succeeds with zero warnings across all workspace members (using default lint levels)
- `cargo test` succeeds with all tests passing (both existing tests and new tests introduced by issue-9)
- `rmcp 0.16` resolves in `Cargo.lock` -- not `0.17`, not `1.x`
- `rig-core 0.32` resolves in `Cargo.lock` -- not a newer major/minor
- No duplicate/conflicting versions of `rmcp` appear in the dependency graph (i.e., `cargo tree -d` does not show `rmcp` as a duplicated package)
- No new clippy `#[allow(...)]` annotations were added solely to suppress warnings introduced by this issue's code

## Implementation Details

This task does not create or modify source files. It is a verification-only task that runs commands and inspects their output.

Steps to execute:

1. **Dependency resolution check**
   - Run `cargo tree -p rmcp` and confirm the resolved version is `0.16.x`
   - Run `cargo tree -p rig-core` and confirm the resolved version is `0.32.x`
   - Run `cargo tree --duplicates` and confirm `rmcp` does not appear as a duplicated package (rig-core and tool-registry must share the same rmcp version)

2. **Type check**
   - Run `cargo check --workspace` and confirm exit code 0 with no errors

3. **Lint check**
   - Run `cargo clippy --workspace -- -D warnings` and confirm exit code 0 with no warnings
   - The `-D warnings` flag ensures any clippy warning is treated as an error, providing a strict pass/fail signal

4. **Test suite**
   - Run `cargo test --workspace` and confirm exit code 0 with all tests passing
   - Verify output includes test results from:
     - `tool-registry` unit tests (transport parsing tests)
     - `tool-registry` integration tests (MCP connection tests)
     - `agent-sdk` existing tests (envelope, micro_agent, skill_manifest)
     - `skill-loader` existing tests (skill_loader, validation, validation_integration)

5. **If any step fails**
   - Diagnose the root cause (compiler error, clippy warning, test failure, version conflict)
   - Fix the issue in the relevant source file(s) introduced by the earlier issue-9 tasks
   - Re-run the failing command to confirm the fix
   - Repeat until all four checks pass cleanly

## Dependencies

- Blocked by: All preceding issue-9 tasks (Groups 1-5):
  - "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml"
  - "Define `McpHandle` newtype wrapping the rmcp client session"
  - "Create transport module with endpoint URL parsing"
  - "Add `McpHandle` field to `ToolEntry`"
  - "Implement `connect()` with real MCP client logic"
  - "Add `rig-core` and `rmcp` dependencies to agent-runtime Cargo.toml"
  - "Implement MCP-to-rig-core bridge in agent-runtime"
  - "Update agent-runtime main.rs with skeleton startup flow"
  - "Write transport unit tests"
  - "Write MCP connection integration tests"
- Blocking: None (this is the final task in issue-9)

## Risks & Edge Cases

- **rmcp version conflict**: `rig-core 0.32` depends on `rmcp ^0.16`. If tool-registry accidentally specifies `rmcp = "1"` or `rmcp = "0.17"`, Cargo will either pull two versions (causing duplicate-type errors) or fail to resolve. Mitigation: confirm `cargo tree --duplicates` shows no rmcp duplication.
- **Feature flag mismatches**: `rig-core`'s `rmcp` feature may require specific rmcp features that differ from what tool-registry enables. If types don't unify, compilation will fail with "expected type from crate X, found type from crate X" errors. Mitigation: both crates should use compatible feature sets and the same version.
- **Platform-specific failures**: `UnixStream` is not available on Windows. If CI runs on Windows, Unix socket tests will fail. Mitigation: Unix socket tests should be gated with `#[cfg(unix)]`.
- **Existing test regressions**: Changes to `ToolRegistry` or `ToolEntry` could break skill-loader tests that depend on these types. Mitigation: run full workspace test suite, not just new tests.
- **Clippy false positives on generated/derived code**: Clippy may warn on patterns in derived trait impls or macro-generated code. Mitigation: address warnings case-by-case; prefer fixing the source over adding `#[allow]`.
- **Network-dependent integration tests**: MCP connection integration tests that spin up in-process servers should bind to `localhost:0` (OS-assigned port) to avoid port conflicts. If they hardcode ports, they may fail in CI.

## Verification

This task is itself the verification step for the entire issue-9 implementation. It is confirmed complete when:

1. `cargo check --workspace` exits with code 0
2. `cargo clippy --workspace -- -D warnings` exits with code 0
3. `cargo test --workspace` exits with code 0 and all tests pass
4. `cargo tree -p rmcp` shows version `0.16.x` (not duplicated)
5. `cargo tree -p rig-core` shows version `0.32.x`
6. `cargo tree --duplicates` does not list `rmcp` as a duplicated package
