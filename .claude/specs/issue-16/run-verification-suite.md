# Spec: Run verification suite

> From: .claude/tasks/issue-16.md

## Objective
Run the full workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that all changes introduced by issue-16 (semantic routing) compile cleanly, pass lint, and cause no test regressions in any workspace crate. This is the final gate before the issue can be considered complete.

## Current State
The workspace contains six members defined in the root `Cargo.toml`:
- `crates/agent-sdk` -- core SDK types (`AgentRequest`, `AgentResponse`, `MicroAgent` trait, etc.)
- `crates/skill-loader` -- skill manifest loading
- `crates/tool-registry` -- tool discovery and registration
- `crates/agent-runtime` -- runtime execution (already depends on `rig-core`)
- `crates/orchestrator` -- request routing and agent dispatch (target of issue-16 changes)
- `tools/echo-tool` -- example tool binary

The orchestrator crate currently has four test files under `crates/orchestrator/tests/`:
- `agent_endpoint_test.rs`
- `config_test.rs`
- `error_test.rs`
- `orchestrator_test.rs`

After issue-16 work completes, additional test files will exist:
- `crates/orchestrator/tests/semantic_router_test.rs` (new)
- Updated `orchestrator_test.rs` with semantic routing integration tests
- Updated `config_test.rs` with embedding config field tests

The orchestrator crate will have new source files:
- `crates/orchestrator/src/semantic_router.rs` (new module)
- Modified `error.rs` (new `EmbeddingError` variant)
- Modified `config.rs` (new embedding configuration fields)
- Modified `orchestrator.rs` (SemanticRouter integration, removal of `route_by_description_match`)
- Modified `lib.rs` (new `pub mod semantic_router;` declaration)
- Modified `Cargo.toml` (added `rig-core` and `tracing` dependencies, `tokio` dev-dependency update)

There is no CI workflow (no `.github/workflows/` files). The project relies on the stop-quality-gate hook at `.claude/hooks/stop-quality-gate.sh`, which runs `cargo test` as part of its checks. There is no Makefile or dedicated test script.

## Requirements
- `cargo check` must succeed across the entire workspace with zero errors
- `cargo clippy` must succeed across the entire workspace with zero warnings (all default lints clean)
- `cargo test` must succeed across the entire workspace with zero failures
- All pre-existing tests in `agent-sdk`, `agent-runtime`, `skill-loader`, `tool-registry`, and `tools/echo-tool` must continue to pass (no regressions)
- All pre-existing orchestrator tests (`agent_endpoint_test`, `config_test`, `error_test`, `orchestrator_test`) must continue to pass
- All new orchestrator tests (`semantic_router_test`, updated `orchestrator_test`, updated `config_test`) must pass
- The `semantic_router` module must compile without any warnings under `cargo clippy`
- No unused imports, dead code, or other compiler warnings in any modified file

## Implementation Details
This task involves no code changes. It is a command-line verification-only task.

**Commands to run (in order):**

1. **Type check the full workspace:**
   ```
   cargo check --workspace
   ```
   Confirms all crates compile. Catches missing imports, type errors, trait bound issues (especially relevant for `EmbeddingModel` generics in `semantic_router.rs`).

2. **Lint the full workspace:**
   ```
   cargo clippy --workspace -- -D warnings
   ```
   Runs Clippy with warnings-as-errors. Catches idiomatic issues, unnecessary clones, missing error handling, etc. The `-D warnings` flag ensures no warnings are silently ignored.

3. **Run all tests:**
   ```
   cargo test --workspace
   ```
   Runs every `#[test]` and `#[tokio::test]` across all six workspace members. Verifies both unit tests (in `src/`) and integration tests (in `tests/`).

**Per-crate verification focus:**

- `agent-sdk`: No changes expected. Tests must pass unchanged, confirming the SDK types used by the orchestrator are still compatible.
- `agent-runtime`: No changes expected. Already depends on `rig-core`, so no version conflict should arise from the orchestrator also depending on `rig-core`.
- `skill-loader`: No changes expected. Tests must pass unchanged.
- `tool-registry`: No changes expected. Tests must pass unchanged.
- `tools/echo-tool`: No changes expected. Must still compile.
- `orchestrator`: The primary crate under change. All existing and new tests must pass. The `semantic_router` module must produce zero Clippy warnings.

**Specific things to watch for:**

- `rig-core` version compatibility: Both `agent-runtime` and `orchestrator` depend on `rig-core`. Verify there is no version conflict in `Cargo.lock` (both should resolve to the same `0.32.x` version).
- Async test runtime: New tests use `#[tokio::test]`. Verify the `tokio` dev-dependency features (`macros`, `rt`) are sufficient.
- Mock `EmbeddingModel` implementation: The mock in `semantic_router_test.rs` must satisfy `rig-core`'s `EmbeddingModel` trait bounds. Any trait changes in rig-core 0.32 would surface here.
- Removed `route_by_description_match`: Existing `orchestrator_test.rs` tests that relied on description-match routing must have been updated to use either `target_agent` context or the new `SemanticRouter`. Verify no tests are broken by the removal.

## Dependencies
- Blocked by: All other issue-16 tasks:
  - "Add `EmbeddingError` variant to `OrchestratorError`"
  - "Add `rig-core` dependency to orchestrator `Cargo.toml`"
  - "Implement `SemanticRouter` struct with two-phase routing"
  - "Integrate `SemanticRouter` into `Orchestrator`"
  - "Add embedding model configuration to `OrchestratorConfig`"
  - "Write unit tests for `SemanticRouter`"
  - "Write integration tests for semantic routing in `Orchestrator`"
  - "Write unit tests for config embedding fields"
- Blocking: Nothing (this is the final task)

## Risks & Edge Cases
- **rig-core version mismatch**: If `agent-runtime` pins a different minor version of `rig-core` than the orchestrator, Cargo may pull two copies, causing trait incompatibility (e.g., `agent_runtime::rig_core::Embedding` != `orchestrator::rig_core::Embedding`). Mitigation: both crates should depend on `rig-core = "0.32"` without conflicting feature flags, resolving to a single copy.
- **Flaky network-dependent tests**: If any test accidentally calls a real embedding API instead of using the mock, it will fail in environments without network access or API keys. Mitigation: verify all `SemanticRouter` tests use `MockEmbeddingModel`, not a real provider.
- **Platform-specific compilation**: Floating-point arithmetic in cosine similarity may produce slightly different results on different architectures. Mitigation: tests should use tolerance-based assertions (e.g., `assert!((similarity - expected).abs() < 1e-6)`) rather than exact equality.
- **Test ordering or state leakage**: Integration tests that modify environment variables (e.g., `EMBEDDING_PROVIDER`) could interfere with each other if run in parallel. Mitigation: config tests should use unique env var names or run with `#[serial]` if needed.
- **Clippy false positives on generics**: Complex generic bounds on `EmbeddingModel` methods may trigger Clippy suggestions that conflict with rig-core's API design. Mitigation: targeted `#[allow(...)]` attributes with explanatory comments, only if unavoidable.

## Verification
- `cargo check --workspace` exits with code 0 and produces no error output
- `cargo clippy --workspace -- -D warnings` exits with code 0 and produces no warning output
- `cargo test --workspace` exits with code 0, and the summary line shows all tests passing with 0 failures
- The test output includes results from all six workspace members, confirming none were skipped
- The test output includes the new test names from `semantic_router_test.rs` (e.g., `exact_intent_match`, `semantic_fallback`, `no_match_below_threshold`, etc.)
- The test output includes updated tests from `orchestrator_test.rs` and `config_test.rs`
- No `warning:` lines appear in the compilation output for any file in `crates/orchestrator/src/`
