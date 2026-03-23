# Spec: Run verification suite
> From: .claude/tasks/issue-51.md

## Objective
Run the full verification suite for the `list-agents` crate to confirm that all acceptance criteria from issue-51 are satisfied: the crate builds, tests pass, clippy is clean, and the workspace type-checks.

## Current State
This is the final task in the issue-51 breakdown. All previous tasks (scaffold, core implementation, integration tests, README) must be complete before this task runs. No code changes are produced by this task — it is purely a validation gate.

## Requirements
1. `cargo build -p list-agents` succeeds with no errors
2. `cargo test -p list-agents` passes all unit and integration tests
3. `cargo clippy -p list-agents` reports no warnings or errors
4. `cargo check` (workspace-wide) succeeds, confirming the new crate does not break any existing workspace member
5. The MCP tool is named `list_agents` in tools/list output
6. The tool returns structured JSON: `{"agents": [{name, url, description}, ...]}`
7. The tool reads `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` environment variables
8. The tool returns an empty array `{"agents": []}` when no agents are registered

## Implementation Details
This task produces no code. Execute the following commands in order, stopping on the first failure:

1. **Build**: `cargo build -p list-agents`
2. **Test**: `cargo test -p list-agents`
3. **Lint**: `cargo clippy -p list-agents -- -D warnings`
4. **Workspace check**: `cargo check`

If all four commands exit 0, review test names and output to confirm that the acceptance criteria in Requirements 5-8 are covered by existing tests. Specifically verify:
- An integration test asserts the tool name is `"list_agents"`
- A unit or integration test asserts the JSON output shape contains an `"agents"` array
- A test sets `AGENT_ENDPOINTS` / `AGENT_DESCRIPTIONS` and verifies parsed output
- A test with no env vars set asserts an empty agents array is returned

## Dependencies
- Blocked by: All previous tasks (Create Cargo.toml, Add to workspace, Implement ListAgentsTool, Write main.rs, Write integration tests, Write README)
- Blocking: None

## Risks & Edge Cases
- **Flaky env var tests**: Unit tests that mutate `std::env` can race with each other under the default multi-threaded test runner. If tests fail intermittently, re-run with `cargo test -p list-agents -- --test-threads=1` to confirm, then fix the underlying isolation issue rather than masking it.
- **Clippy false positives**: If a new clippy lint was introduced in a recent toolchain update, address it before marking verification complete — do not allow `#[allow(...)]` without justification.
- **Workspace breakage**: `cargo check` covers the entire workspace. A failure here may indicate a dependency conflict or feature flag issue introduced by the new crate, not necessarily a bug in `list-agents` itself.

## Verification
This task *is* the verification step. Success means all four commands pass and the acceptance criteria are confirmed covered by tests. Report the results (pass/fail, any warnings) to the user. Do not merge or push — just report status.
