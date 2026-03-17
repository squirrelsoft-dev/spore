# Spec: Run verification suite

> From: .claude/tasks/issue-11.md

## Objective
Verify that the entire `agent-runtime` crate compiles cleanly, passes linting without warnings, and all tests (unit and integration) pass. Then confirm that the full workspace still builds and tests successfully, ensuring nothing in `agent-runtime` caused regressions in other crates (`agent-sdk`, `skill-loader`, `tool-registry`, `orchestrator`, `echo-tool`).

## Current State
The `agent-runtime` crate lives at `crates/agent-runtime/` and is a member of the root workspace defined in `/workspaces/spore/Cargo.toml`. The workspace contains six members: `agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, `orchestrator`, and `echo-tool`. Currently, `agent-runtime` has source files in `src/` (`main.rs`, `tool_bridge.rs`) and no test files under `tests/`. By the time this task runs, the preceding tasks will have added `config.rs`, `provider.rs`, `runtime_agent.rs`, a refactored `main.rs`, and test files under `tests/` (`config_test.rs`, `provider_test.rs`, `runtime_agent_test.rs`).

## Requirements
- `cargo check -p agent-runtime` exits with code 0 and produces no errors.
- `cargo clippy -p agent-runtime` exits with code 0 and produces no warnings (treat warnings as errors via `-- -D warnings`).
- `cargo test -p agent-runtime` exits with code 0 with all non-`#[ignore]` tests passing.
- `cargo check` (full workspace) exits with code 0 and produces no errors.
- `cargo test` (full workspace) exits with code 0 with all non-`#[ignore]` tests passing.
- Any failures must be diagnosed and fixed before the task is marked complete.

## Implementation Details
Run the following commands in order. Each must succeed before proceeding to the next.

1. **Type-check agent-runtime in isolation**
   ```
   cargo check -p agent-runtime
   ```
   Expected: clean exit, no errors. Confirms all types resolve and dependencies are wired correctly.

2. **Lint agent-runtime with clippy**
   ```
   cargo clippy -p agent-runtime -- -D warnings
   ```
   Expected: clean exit, zero warnings. The `-D warnings` flag promotes warnings to errors so the command fails if any lint fires.

3. **Run agent-runtime tests**
   ```
   cargo test -p agent-runtime
   ```
   Expected: all compiled test binaries run, all non-ignored tests pass. Tests marked `#[ignore]` (e.g., those requiring `OPENAI_API_KEY`) are skipped automatically and do not count as failures.

4. **Type-check the full workspace**
   ```
   cargo check
   ```
   Expected: clean exit across all six workspace members. Catches any cross-crate breakage (e.g., a changed public API in `agent-sdk` that `orchestrator` depends on).

5. **Run full workspace tests**
   ```
   cargo test
   ```
   Expected: all non-ignored tests pass across every workspace member.

If any step fails:
- Read the error output carefully.
- Diagnose the root cause (type error, missing import, failing assertion, clippy lint, etc.).
- Fix the issue in the appropriate source or test file.
- Re-run from step 1 to confirm the fix does not introduce new problems.

## Dependencies
- Blocked by: "Write unit tests for config and provider modules", "Write integration test for RuntimeAgent construction"
- Blocking: none (this is the final task in issue-11)

## Risks & Edge Cases
- **Env var pollution in tests**: Unit tests that use `std::env::set_var` can interfere with each other when run in parallel. The preceding test tasks should use `#[serial]` (from `serial_test`) or a scoped env helper. If flaky failures appear, check for parallel env var conflicts.
- **Ignored tests miscounted as failures**: `cargo test` does not fail on `#[ignore]` tests by default, but verify this is the case. If the CI environment sets `RUST_TEST_THREADS=1` or uses `--include-ignored`, those tests may run and fail due to missing API keys.
- **Clippy version drift**: Different Rust toolchain versions may introduce new clippy lints. If a new lint fires that is clearly a false positive or stylistic disagreement, suppress it with an `#[allow(...)]` attribute and a comment explaining why, rather than restructuring code.
- **Long compile times**: The `rig-core` dependency tree is large. First compilation may be slow. Subsequent steps benefit from cached artifacts.
- **Cross-crate API changes**: If earlier tasks in this issue changed public types in `agent-sdk`, `skill-loader`, or `tool-registry`, the full workspace check in step 4 will catch breakage in downstream consumers like `orchestrator`.

## Verification
- All five commands listed in Implementation Details exit with code 0.
- `cargo clippy -p agent-runtime -- -D warnings` produces no output other than the "Finished" line.
- `cargo test -p agent-runtime` summary line shows 0 failures (ignored tests are acceptable).
- `cargo test` (workspace) summary line shows 0 failures across all crates.
- No source files were left with commented-out code or debug `println!` statements as part of fixes.
