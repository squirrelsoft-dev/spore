# Spec: Run verification suite

> From: .claude/tasks/issue-13.md

## Objective

Execute the full Cargo quality-gate sequence (`cargo check`, `cargo clippy`, `cargo test`) across the entire workspace to confirm that all issue-13 constraint-enforcement changes compile cleanly, produce no clippy warnings, and pass all tests. This is the final task in the issue-13 series and produces no source files. It is a gate that either confirms the work is done or surfaces failures that must be fixed in predecessor tasks.

## Current State

### Workspace layout

The workspace root `Cargo.toml` lists six members:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/orchestrator`
- `tools/echo-tool`

### Changes made by predecessor tasks (issue-13)

By the time this verification task runs, all preceding issue-13 tasks will have been applied:

1. **`AgentResponse`** (`crates/agent-sdk/src/agent_response.rs`) -- new `escalate_to: Option<String>` field with `#[serde(default, skip_serializing_if = "Option::is_none")]`. `AgentResponse::success()` initializes it as `None`.
2. **`AgentError`** (`crates/agent-sdk/src/agent_error.rs`) -- new `ActionDisallowed { action: String, allowed: Vec<String> }` variant with `Display` impl and HTTP 403 mapping.
3. **`AppError`** (`crates/agent-runtime/src/http.rs`) -- `into_response()` updated with `ActionDisallowed` mapping to `403 Forbidden`.
4. **`ToolEntry`** (`crates/tool-registry/src/tool_entry.rs`) -- new `action_type: Option<String>` field.
5. **`tool_bridge.rs`** (`crates/agent-runtime/src/tool_bridge.rs`) -- `build_agent_with_tools()` accepts `max_turns` and calls `.default_max_turns()`. `resolve_mcp_tools()` accepts `allowed_actions` and filters tools by `action_type`.
6. **`provider.rs`** (`crates/agent-runtime/src/provider.rs`) -- passes constraints through to `build_agent_with_tools`.
7. **`constraint_enforcer.rs`** (`crates/agent-runtime/src/constraint_enforcer.rs`) -- new module wrapping `Arc<dyn MicroAgent>` with post-invocation confidence and escalation checks.
8. **`runtime_agent.rs`** (`crates/agent-runtime/src/runtime_agent.rs`) -- maps `MaxTurnsError` substring to `AgentError::MaxTurnsExceeded`.
9. **`lib.rs`** (`crates/agent-runtime/src/lib.rs`) -- `pub mod constraint_enforcer;` added.
10. **`main.rs`** (`crates/agent-runtime/src/main.rs`) -- `RuntimeAgent` wrapped with `ConstraintEnforcer` before HTTP layer.
11. **New test files**:
    - `crates/agent-runtime/tests/constraint_enforcer_test.rs` (5 tests for confidence/escalation logic)
    - `crates/agent-runtime/tests/runtime_agent_test.rs` or equivalent (max_turns mapping test)
    - `crates/tool-registry/tests/tool_registry_test.rs` or `crates/agent-runtime/tests/tool_bridge_test.rs` (allowed_actions filtering tests)
12. **Updated test files** (to include `escalate_to` field in `AgentResponse` struct literals):
    - `crates/agent-sdk/tests/micro_agent_test.rs`
    - `crates/agent-runtime/tests/http_test.rs`

### Regression-sensitive test files

These existing test files construct `AgentResponse` directly with struct literals and will fail to compile if `escalate_to` is not included:

- `crates/agent-runtime/tests/http_test.rs` -- 6 integration tests (lines 69-75 construct `AgentResponse` without `escalate_to`; predecessor task must have added the field)
- `crates/agent-sdk/tests/micro_agent_test.rs` -- 7 tests (lines 58-64 construct `AgentResponse` without `escalate_to`; predecessor task must have added the field)

Additionally, `crates/agent-runtime/src/http.rs` has 5 unit tests in a `#[cfg(test)]` block that exercise `AppError::into_response()` for each `AgentError` variant. The new `ActionDisallowed` variant must have a corresponding test, or at minimum must not cause an exhaustive-match compile error.

## Requirements

1. `cargo check` (workspace-wide) exits with code 0 -- all six workspace members compile, including all test targets.
2. `cargo clippy -- -D warnings` (workspace-wide) exits with code 0 and produces zero warnings. The `-D warnings` flag converts warnings to errors.
3. `cargo test` (workspace-wide) exits with code 0 and all tests pass with 0 failures across all crates.
4. No regressions in the pre-existing tests from issue-12:
   - `crates/agent-runtime/tests/http_test.rs`: all 6 tests pass (`invoke_valid_request_returns_200`, `invoke_internal_error_returns_500`, `invoke_tool_call_failed_returns_502`, `invoke_invalid_json_returns_422`, `health_returns_200_with_healthy_status`, `health_returns_200_with_degraded_status`).
   - `crates/agent-sdk/tests/micro_agent_test.rs`: all 7 tests pass (`mock_agent_implements_trait`, `trait_object_is_dyn_compatible`, `invoke_returns_ok`, `invoke_returns_err`, `health_status_healthy`, `health_status_degraded`, `health_status_unhealthy`).
5. All new issue-13 tests pass:
   - Constraint enforcer tests (confidence above/below threshold, escalation with/without `escalate_to`, delegation of `manifest`/`health`, error propagation).
   - Allowed-actions filtering tests (tools excluded by `action_type`, tools included when `action_type` matches, tools included when `action_type` is `None`, empty `allowed_actions` passes all tools).
   - Max-turns enforcement test (`MaxTurnsError` substring mapped to `AgentError::MaxTurnsExceeded`).

## Implementation Details

### Files to create or modify

None. This task runs only command-line tools and inspects their output.

### Commands to run, in order

Run each command from the workspace root (`/workspaces/spore`):

1. **Type-check full workspace**
   ```
   cargo check
   ```
   Validates: all six workspace members compile, including the new `constraint_enforcer` module, updated `AgentResponse` with `escalate_to`, new `ActionDisallowed` variant, updated `ToolEntry` with `action_type`, and all test targets. Catches missing imports, signature mismatches, exhaustive-match failures from new enum variants, and struct literal completeness.

2. **Lint full workspace**
   ```
   cargo clippy -- -D warnings
   ```
   Validates: no clippy warnings across any crate. Catches dead code from unused new fields, redundant clones, missing `#[serde(default)]` attributes that should be present, unused imports in new modules, and type casting issues (e.g., `f32` to `f64` confidence comparison).

3. **Test `agent-runtime` crate**
   ```
   cargo test -p agent-runtime
   ```
   Validates: all integration tests in `tests/http_test.rs` (6 pre-existing), `tests/constraint_enforcer_test.rs` (new, ~5 tests), and any max-turns tests pass. Also runs unit tests inside `src/http.rs` (5 pre-existing + any new `ActionDisallowed` test). This is the crate most affected by issue-13 changes.

4. **Test `agent-sdk` crate**
   ```
   cargo test -p agent-sdk
   ```
   Validates: the 7 pre-existing tests in `tests/micro_agent_test.rs` still pass with the updated `AgentResponse` struct. Also validates any new tests for `AgentError::ActionDisallowed` display formatting.

5. **Test `tool-registry` crate**
   ```
   cargo test -p tool-registry
   ```
   Validates: pre-existing tests in `tests/tool_entry_test.rs` and `tests/tool_registry_test.rs` pass with the updated `ToolEntry` struct. Also validates any new allowed-actions filtering tests if they live here.

6. **Test full workspace**
   ```
   cargo test
   ```
   Validates: all tests across all six crates pass, confirming no transitive regressions in `skill-loader`, `orchestrator`, or `tools/echo-tool`.

### Expected output indicators

- Each `cargo check` call ends with `Finished` and no `error[E...]` lines.
- `cargo clippy -- -D warnings` ends with `Finished` and no `warning:` or `error:` lines.
- `cargo test -p agent-runtime` shows `test result: ok` with at minimum 11 passing tests (6 http_test + 5 http unit tests + constraint_enforcer tests + max_turns tests) and 0 failures.
- `cargo test -p agent-sdk` shows `test result: ok` with at minimum 7 passing tests and 0 failures.
- `cargo test` (workspace) shows `test result: ok` for every crate with tests.

### What to do if a command fails

- **Struct literal missing `escalate_to`**: A predecessor task ("Add `escalate_to` field to `AgentResponse`") was supposed to update `http_test.rs` and `micro_agent_test.rs`. Fix by adding `escalate_to: None` to every `AgentResponse { ... }` struct literal in those files.
- **Exhaustive match on `AgentError`**: The new `ActionDisallowed` variant must be handled in `AppError::into_response()` in `crates/agent-runtime/src/http.rs`. Add the `ActionDisallowed` arm mapping to `StatusCode::FORBIDDEN`.
- **Clippy warning on `action_type` field**: If `action_type` is declared but never read, clippy will flag dead code. Confirm the filtering logic in `resolve_mcp_tools()` actually reads the field.
- **Type mismatch in confidence comparison**: `Constraints.confidence_threshold` is `f64`, `AgentResponse.confidence` is `f32`. The enforcer must cast with `response.confidence as f64` before comparing. A type error here means the cast is missing.
- **`MaxTurnsError` detection fails**: If `BuiltAgent::prompt()` returns an error string that does not contain `"MaxTurnsError"` or `"MaxTurnError"`, the mapping will silently fall through to `AgentError::Internal`. Verify the substring match is correct by checking rig-core's error message format.
- **Regression in another crate**: Run `cargo test -p <crate>` in isolation to confirm the failure is related to issue-13 changes vs. pre-existing. Check `cargo tree -p <crate>` for dependency version conflicts introduced by new dependencies.

## Dependencies

- Blocked by: "Write tests for ConstraintEnforcer", "Write tests for allowed_actions filtering", "Write tests for max_turns enforcement"
- Blocking: none (this is the final task in the issue-13 series)

## Risks & Edge Cases

- **`AgentResponse` struct literal completeness**: Both `http_test.rs` (line 69-75) and `micro_agent_test.rs` (line 58-64) construct `AgentResponse` using struct literal syntax without `..Default::default()`. Adding the `escalate_to` field to the struct will cause a compile error in both files unless the predecessor task updated them. This is the single most likely regression.

- **Exhaustive match breakage from `ActionDisallowed`**: The `AppError::into_response()` match and the `AgentError::Display` match must both handle the new variant. If either match is missing an arm, `cargo check` will fail. The `http.rs` unit tests also use `match` or explicit variant construction that must cover the new variant.

- **Serde compatibility of new fields**: The `escalate_to` field on `AgentResponse` and `action_type` field on `ToolEntry` both use `#[serde(default, skip_serializing_if = "Option::is_none")]`. If these attributes are missing, existing JSON payloads that lack these fields will fail to deserialize, breaking tests that parse JSON responses.

- **`f32`/`f64` precision in confidence comparison**: The `ConstraintEnforcer` compares `response.confidence` (`f32`) against `constraints.confidence_threshold` (`f64`). Floating-point promotion from `f32` to `f64` can introduce rounding artifacts. For example, `0.85_f32 as f64` is `0.8500000238418579`, not `0.85`. Tests must account for this by choosing threshold values that are exact in both precisions, or by using inequality comparisons with appropriate margins.

- **Clippy false positives on test code**: Clippy lints like `clippy::too_many_arguments` or `clippy::needless_pass_by_value` may fire on test helper functions. If a lint is genuinely inapplicable, suppress it with a targeted `#[allow(...)]` on the specific item, not a crate-wide allow.

- **`cargo test` hanging from `start_server`**: If any new test accidentally calls `start_server` (which binds a TCP listener and blocks on `axum::serve()`), the test binary will hang. All tests must use `build_router` + `oneshot()` or direct function calls, never `start_server`.

- **New `ActionDisallowed` variant in `AgentError` may affect `PartialEq`-based assertions**: Existing tests assert `AgentError` equality (e.g., `micro_agent_test.rs` line 127). The new variant does not break `PartialEq` derive, but any test that does exhaustive matching on `AgentError` values must be updated.

## Verification

This task is complete when all of the following conditions are simultaneously true:

1. `cargo check` (workspace) exits 0 with no error lines.
2. `cargo clippy -- -D warnings` (workspace) exits 0 with no warning or error lines.
3. `cargo test -p agent-runtime` exits 0 with 0 failures. All pre-existing http_test.rs tests pass. All new constraint_enforcer and max_turns tests pass.
4. `cargo test -p agent-sdk` exits 0 with 0 failures. All 7 pre-existing micro_agent_test.rs tests pass.
5. `cargo test -p tool-registry` exits 0 with 0 failures. All pre-existing and new allowed_actions tests pass.
6. `cargo test` (workspace) exits 0 with no failing tests in any crate.

Record the final output of each command (the `Finished` / `test result` lines) as evidence that the verification gate passed before closing the issue-13 branch.
