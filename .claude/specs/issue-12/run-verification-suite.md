# Spec: Run verification suite

> From: .claude/tasks/issue-12.md

## Objective

Execute a sequence of `cargo` commands to confirm that all prior tasks in the issue-12 series have been implemented correctly. This task validates three properties: (1) the `agent-runtime` crate compiles cleanly after adding the axum HTTP module; (2) no clippy warnings remain in `agent-runtime`; (3) all integration tests in `agent-runtime` pass. A final workspace-wide check ensures no regressions were introduced in the five other crates (`agent-sdk`, `skill-loader`, `tool-registry`, `orchestrator`, `tools/echo-tool`).

This is the final task in the issue-12 series — it produces no source files and no commits; it is a gate that either confirms the work is done or surfaces failures to address.

## Current State

### Workspace layout

The workspace root `Cargo.toml` lists six members:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/orchestrator`
- `tools/echo-tool`

### `agent-runtime` as of the start of issue-12

`crates/agent-runtime/Cargo.toml` declares these runtime dependencies: `rig-core`, `rmcp`, `tool-registry`, `agent-sdk`, `skill-loader`, `serde_json`, `tokio` (features = ["full"]), `tracing`, `tracing-subscriber`, `futures`. No dev-dependencies exist yet.

`crates/agent-runtime/src/lib.rs` currently exports four modules: `config`, `provider`, `runtime_agent`, `tool_bridge`. The `http` module does not yet exist.

### Changes made by predecessor tasks

By the time this verification task runs, the following will have been applied:

1. `crates/agent-runtime/Cargo.toml` — `axum = "0.8"` added to `[dependencies]`; `tower = { version = "0.5", features = ["util"] }` and `http-body-util = "0.1"` added to `[dev-dependencies]`.
2. `crates/agent-runtime/src/http.rs` — new file containing `AppError`, `AppState`, `HealthResponse`, `invoke_handler`, `health_handler`, `build_router`, and `start_server`.
3. `crates/agent-runtime/src/lib.rs` — `pub mod http;` added.
4. `crates/agent-runtime/src/main.rs` — HTTP server startup wired in after agent construction.
5. `crates/agent-runtime/tests/http_test.rs` — six integration tests using `tower::ServiceExt::oneshot()`.

## Requirements

1. `cargo check -p agent-runtime` exits with code 0 (no compile errors in the library, binary, or test targets of `agent-runtime`).
2. `cargo clippy -p agent-runtime` exits with code 0 and emits zero warnings. This includes test targets; use `-- -D warnings` to make warnings fail the run.
3. `cargo test -p agent-runtime` exits with code 0 and reports all six tests in `tests/http_test.rs` as `ok`:
   - `invoke_valid_request_returns_200`
   - `invoke_internal_error_returns_500`
   - `invoke_tool_call_failed_returns_502`
   - `invoke_invalid_json_returns_400`
   - `health_returns_200_with_healthy_status`
   - `health_returns_200_with_degraded_status`
4. `cargo check` (workspace-wide) exits with code 0 — all six workspace members compile.
5. `cargo test` (workspace-wide) exits with code 0 — all tests across all crates pass, including any existing tests in `agent-sdk`, `skill-loader`, `tool-registry`, `orchestrator`, and `tools/echo-tool`.

## Implementation Details

### Files to create or modify

None. This task runs only command-line tools and inspects their output.

### Commands to run, in order

Run each command from the workspace root (`/workspaces/spore`):

1. **Type-check `agent-runtime` only**
   ```
   cargo check -p agent-runtime
   ```
   Validates: the HTTP module, `AppError`, `AppState`, `HealthResponse`, `build_router`, `start_server`, and the test file all parse and type-check. Faster than a full build; catches missing imports, signature mismatches, and trait bound violations.

2. **Lint `agent-runtime` only**
   ```
   cargo clippy -p agent-runtime -- -D warnings
   ```
   Validates: no clippy warnings remain (dead code, unused imports, redundant clones, needless borrows, etc.). The `-D warnings` flag converts warnings into errors so that marginal issues are not silently ignored. Run against both library and test targets.

3. **Test `agent-runtime` only**
   ```
   cargo test -p agent-runtime
   ```
   Validates: all six integration tests in `tests/http_test.rs` pass. Also runs any unit tests inside `src/` (e.g., inline `#[cfg(test)]` blocks). Requires `axum`, `tower`, and `http-body-util` to be correctly listed in `Cargo.toml`.

4. **Type-check full workspace**
   ```
   cargo check
   ```
   Validates: the axum dependency addition and the new `pub mod http` export in `lib.rs` do not break any crate that depends on `agent-runtime` directly or transitively.

5. **Test full workspace**
   ```
   cargo test
   ```
   Validates: no regressions in `agent-sdk`, `skill-loader`, `tool-registry`, `orchestrator`, or `tools/echo-tool`. Also re-runs `agent-runtime` tests, confirming end-to-end integrity.

### Expected output indicators

- Each `cargo check` call ends with `Finished` and no `error[E...]` lines.
- `cargo clippy -- -D warnings` ends with `Finished` and no `warning:` or `error:` lines.
- `cargo test -p agent-runtime` shows `test result: ok. 6 passed; 0 failed` (or more, if unit tests exist inside `src/`).
- `cargo test` (workspace) shows `test result: ok` for every crate that has tests.

### What to do if a command fails

- **Compile error in `agent-runtime`**: Return to the relevant predecessor task spec and fix the issue there (do not patch here).
- **Clippy warning**: Fix the warning in the source file it originates from, then re-run the full sequence from step 1.
- **Test failure in `http_test.rs`**: Diagnose whether the handler logic or the `AppError` status mapping is wrong; fix in `src/http.rs` or `tests/http_test.rs`, then re-run from step 1.
- **Regression in another crate**: Check whether the axum version pulled in conflicts with a transitive dependency. `cargo tree -p agent-runtime` can reveal version conflicts. Fix in `Cargo.toml` with a compatible version constraint, then re-run the full sequence.

## Dependencies

- Blocked by: Write handler integration tests (which itself is blocked by "Wire router into main.rs", which is blocked by "Create HTTP handler module", which is blocked by "Add axum dependency" and "Create AppError wrapper").
- Blocking: (none — this is the final task in the issue-12 series)

## Risks & Edge Cases

- **Version conflicts with axum vs. existing transitive deps**: `hyper` and `tower` are already in the lockfile via `rig-core`. If `axum 0.8` requires a `hyper` or `tower` version incompatible with what `rig-core` has pinned, `cargo check` will fail with a resolver error. Mitigation: `axum 0.8` targets `hyper 1.x` and `tower 0.4/0.5`, which are the same major versions pulled by recent `rig-core`. Check `Cargo.lock` before assuming a conflict exists.

- **Clippy false positives on generated/test code**: Some clippy lints (e.g., `clippy::too_many_arguments`) may fire on test helper functions. If a lint is genuinely inapplicable, suppress it with a targeted `#[allow(...)]` attribute on the specific item, not a crate-level allow. Do not use `-- -A clippy::all` as a workaround.

- **`cargo test` picking up integration tests from other crates**: Workspace-level `cargo test` runs tests in all members. If any existing crate has flaky or environment-dependent tests, they may fail here. Isolate by re-running `cargo test -p <crate>` for the failing crate to confirm the failure is pre-existing and unrelated to issue-12 changes.

- **`start_server` causes `cargo test` to hang**: `start_server` binds a `TcpListener` and blocks on `axum::serve()`. If any test accidentally calls `start_server` instead of `build_router`, the test binary will hang. Mitigation: ensure all tests in `http_test.rs` use `build_router` + `oneshot()` only. `start_server` is exercised only at runtime, not in tests.

- **Missing `pub` on `HealthResponse` or `build_router`**: If the "Create HTTP handler module" task left these items private, the test file will fail to compile. The fix is in `src/http.rs`, not here.

- **Edition 2024 compatibility**: `crates/agent-runtime/Cargo.toml` uses `edition = "2024"`. Confirm that all new code in `http.rs` and `http_test.rs` is compatible with Rust's 2024 edition rules (e.g., changes to `impl Trait` capture semantics). `cargo check` will surface any edition-related errors.

## Verification

This task is complete when all five of the following conditions are simultaneously true:

1. `cargo check -p agent-runtime` exits 0 with no error lines.
2. `cargo clippy -p agent-runtime -- -D warnings` exits 0 with no warning or error lines.
3. `cargo test -p agent-runtime` exits 0 and shows at minimum 6 passing tests with 0 failures.
4. `cargo check` (workspace) exits 0.
5. `cargo test` (workspace) exits 0 with no failing tests in any crate.

Record the final output of each command (the `Finished` / `test result` lines) as evidence that the verification gate passed before closing the issue-12 branch.
