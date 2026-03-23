# Spec: Run verification suite

> From: .claude/tasks/issue-22.md

## Objective
Verify that all new and existing code compiles cleanly, passes linting without warnings, and all tests pass. Confirm shell scripts are executable. Validate the docker-compose E2E configuration file syntax. Do NOT run the full E2E test.

## Current State
The workspace at `/workspaces/spore/Cargo.toml` contains six members: `agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, `orchestrator`, and `echo-tool`. By the time this task runs, all prior issue-22 tasks will have added:
- `docker-compose.e2e.yml` at the project root
- `tests/e2e/SCENARIO.md`
- `tests/e2e/orchestrator-config.yml`
- `tests/e2e/validate_step1_skill.sh`
- `tests/e2e/validate_step2_tools.sh`
- `tests/e2e/validate_step3_deploy.sh`
- `tests/e2e/validate_step4_route.sh`
- `scripts/e2e-test.sh`
- `tests/e2e_bootstrap_test.rs`
- Updates to `README.md`

Shell scripts live under `scripts/`, `tests/e2e/`, and `.claude/hooks/`.

## Requirements
- `cargo check` (full workspace) exits with code 0.
- `cargo clippy` (full workspace) exits with code 0 with warnings treated as errors.
- `cargo test` (full workspace) exits with code 0 with all non-`#[ignore]` tests passing.
- All shell scripts (`*.sh`) under `scripts/`, `tests/e2e/`, and `.claude/hooks/` have the executable bit set.
- `docker compose -f docker-compose.e2e.yml config` exits with code 0, confirming valid compose syntax.
- Any failures must be diagnosed and fixed before the task is marked complete.

## Implementation Details
Run the following commands in order. Each must succeed before proceeding to the next.

1. **Type-check the full workspace**
   ```
   cargo check
   ```
   Expected: clean exit, no errors across all workspace members.

2. **Lint the full workspace with clippy**
   ```
   cargo clippy -- -D warnings
   ```
   Expected: clean exit, zero warnings. The `-D warnings` flag promotes warnings to errors so the command fails if any lint fires.

3. **Run full workspace tests**
   ```
   cargo test
   ```
   Expected: all compiled test binaries run, all non-ignored tests pass. Tests marked `#[ignore]` (e.g., the E2E wrapper requiring Docker and API keys) are skipped automatically and do not count as failures.

4. **Verify shell scripts are executable**
   ```
   for f in scripts/e2e-test.sh tests/e2e/validate_step1_skill.sh tests/e2e/validate_step2_tools.sh tests/e2e/validate_step3_deploy.sh tests/e2e/validate_step4_route.sh; do
     test -x "$f" || { echo "FAIL: $f is not executable"; exit 1; }
   done
   echo "All shell scripts are executable."
   ```
   Expected: all listed scripts have the executable bit set. If any script is missing the executable bit, fix it with `chmod +x <file>` and re-verify.

5. **Validate docker-compose configuration (dry run)**
   ```
   docker compose -f docker-compose.e2e.yml config
   ```
   Expected: the command parses and resolves the compose file, printing the fully resolved YAML to stdout and exiting with code 0. This catches syntax errors, invalid service definitions, missing required fields, and unresolved variable references without starting any containers.

If any step fails:
- Read the error output carefully.
- Diagnose the root cause (type error, missing import, failing assertion, clippy lint, permission issue, YAML syntax error, etc.).
- Fix the issue in the appropriate file.
- Re-run from step 1 to confirm the fix does not introduce new problems.

## Dependencies
- Blocked by: "Define the test scenario document", "Create `docker-compose.e2e.yml`", "Create orchestrator config file for E2E", "Write step 1 validator", "Write step 2 validator", "Write step 3 validator", "Write step 4 validator", "Write the E2E shell script orchestrator", "Add Rust integration test wrapper", "Write README section for E2E testing"
- Blocking: none (this is the final task in issue-22)

## Risks & Edge Cases
- **Docker not installed**: The `docker compose config` command requires Docker to be installed on the machine. If Docker is unavailable, this step should be skipped with a warning rather than failing the entire verification. Check for `docker` in `$PATH` before running step 5.
- **Clippy version drift**: Different Rust toolchain versions may introduce new clippy lints. If a new lint fires that is clearly a false positive or stylistic disagreement, suppress it with an `#[allow(...)]` attribute and a comment explaining why, rather than restructuring code.
- **Ignored tests miscounted as failures**: `cargo test` does not fail on `#[ignore]` tests by default. The E2E wrapper test should be `#[ignore]` and optionally gated behind `#[cfg(feature = "e2e")]`. Verify it does not run during normal `cargo test`.
- **Shell script paths**: The script list in step 4 is based on expected files from prior tasks. If a file was created at a different path, adjust the check accordingly.
- **Compose file variable resolution**: The compose file may reference environment variables (e.g., `SKILL_NAME`). `docker compose config` resolves these using defaults or `.env` files. If variables are unset and have no defaults, the command will fail. Ensure all variables have default values in the compose file or in a `.env` file.

## Pass/Fail Criteria
- PASS: All five steps exit with code 0 (or step 5 is skipped with a documented reason).
- FAIL: Any step exits non-zero and the root cause cannot be resolved by fixing project files.

## Verification
- `cargo check` exits with code 0.
- `cargo clippy -- -D warnings` produces no output other than compilation and "Finished" lines.
- `cargo test` summary line shows 0 failures across all crates (ignored tests are acceptable).
- Every `*.sh` file under `scripts/` and `tests/e2e/` is executable.
- `docker compose -f docker-compose.e2e.yml config` exits with code 0 (or is skipped if Docker is unavailable).
- No source files were left with commented-out code or debug `println!` statements as part of fixes.
