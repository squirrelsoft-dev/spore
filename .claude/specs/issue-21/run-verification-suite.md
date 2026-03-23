# Spec: Run verification suite

> From: .claude/tasks/issue-21.md

## Objective
Run the full Cargo verification suite (check, clippy, test) across the entire workspace to confirm that all existing code compiles, passes linting, and that every test -- including the new `load_deploy_agent_skill` test -- passes. This is the final gate before the issue can be considered complete.

## Current State
The workspace contains six crates:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/orchestrator`
- `tools/echo-tool`

Existing integration tests in `crates/skill-loader/tests/example_skills_test.rs`:
- `load_cogs_analyst_skill`
- `load_echo_skill`
- `load_skill_writer_skill`
- `load_orchestrator_skill`
- `load_tool_coder_skill`

The new test expected after all other issue-21 tasks are complete:
- `load_deploy_agent_skill`

## Requirements
- `cargo check --workspace` must exit 0 with no errors.
- `cargo clippy --workspace -- -D warnings` must exit 0 with no warnings treated as errors.
- `cargo test --workspace` must exit 0 with all tests passing.
- The test `load_deploy_agent_skill` must appear in the test output and pass.

## Implementation Details
Run the following commands in order from the workspace root (`/workspaces/spore`):

1. **Type check**
   ```bash
   cargo check --workspace
   ```
   Expected: exit code 0, no errors.

2. **Lint**
   ```bash
   cargo clippy --workspace -- -D warnings
   ```
   Expected: exit code 0, no warnings.

3. **Test**
   ```bash
   cargo test --workspace
   ```
   Expected: exit code 0, all tests pass. Output must include a passing result for `load_deploy_agent_skill`.

4. **Targeted confirmation** (optional, for explicit proof)
   ```bash
   cargo test --package skill-loader --test example_skills_test load_deploy_agent_skill
   ```
   Expected: `test load_deploy_agent_skill ... ok`

### Pass/fail criteria
- All three commands must exit 0.
- Zero test failures across the workspace.
- The `load_deploy_agent_skill` test is present and passes.

## Dependencies
- Blocked by: All other issue-21 tasks (the deploy-agent skill file, its test, and any supporting code changes must already be merged/committed).
- Blocking: None -- this is the final task.

## Risks & Edge Cases
- **Missing skill file**: If the `skills/deploy-agent/skill.yaml` file was not created by a prior task, the `load_deploy_agent_skill` test will fail with a file-not-found or parse error.
- **Tool validation failures**: If the skill references tools that do not satisfy the `AllToolsExist` validator at test time, the loader will return an error. The test helper currently uses `AllToolsExist` (always-true stub), so this is low risk.
- **Flaky async tests**: Tests use `#[tokio::test]`. Failures due to filesystem timing are unlikely but possible on very slow I/O; a simple re-run confirms.
- **Clippy version drift**: A newer Rust toolchain may introduce new lints. If clippy fails on pre-existing code unrelated to this issue, that should be reported but not block the verification of the new test.

## Verification
- All three commands (`cargo check`, `cargo clippy`, `cargo test`) exit with code 0.
- `cargo test` output contains the line `test load_deploy_agent_skill ... ok`.
- No test in the workspace reports `FAILED`.
