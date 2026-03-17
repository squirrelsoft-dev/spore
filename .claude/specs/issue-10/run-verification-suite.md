# Spec: Run verification suite

> From: .claude/tasks/issue-10.md

## Objective
Run the full workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that the `echo-tool` crate and all existing workspace crates compile cleanly, produce no warnings, and pass all tests. Additionally verify that `cargo run -p echo-tool` starts without errors. This is the final gate task for issue-10 -- it validates that the echo-tool scaffolding, implementation, unit tests, and integration tests all integrate correctly with the rest of the workspace.

## Current State
The workspace is defined in the root `Cargo.toml` with `resolver = "2"`. By the time this task runs, the workspace members will include:
- `crates/agent-sdk` -- core types (`SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`, `MicroAgent` trait, etc.)
- `crates/skill-loader` -- markdown frontmatter parsing, validation, `SkillLoader` struct
- `crates/tool-registry` -- `ToolEntry`, `RegistryError`, `ToolRegistry`, `ToolExists` trait
- `crates/agent-runtime` -- placeholder
- `crates/orchestrator` -- placeholder
- `tools/echo-tool` -- new MCP echo tool server (added by preceding issue-10 tasks)

By the time this task runs, the preceding issue-10 tasks will have:
1. Created `tools/echo-tool/Cargo.toml` with dependencies on `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, and `tracing-subscriber`
2. Added `"tools/echo-tool"` to the workspace `members` array in the root `Cargo.toml`
3. Implemented the `EchoTool` struct with `#[tool_router]` and `ServerHandler` in `tools/echo-tool/src/main.rs` (and optionally `tools/echo-tool/src/echo.rs`)
4. Written unit tests for the echo tool handler (verifying the tool method returns the input message unchanged)
5. Written an integration test in `tools/echo-tool/tests/echo_server_test.rs` (spawning the binary as a child process, connecting as an MCP client, and performing a round-trip `tools/call` and `tools/list`)

## Requirements
- `cargo check` succeeds across the entire workspace with zero errors
- `cargo clippy` succeeds across the entire workspace with zero warnings (no `#[allow(...)]` suppressions added solely to silence legitimate warnings)
- `cargo test` succeeds across the entire workspace with all tests passing, including:
  - All existing `agent-sdk` tests (serialization, construction, object safety)
  - All existing `skill-loader` unit tests (frontmatter extraction, validation)
  - All existing `skill-loader` integration tests
  - All existing `tool-registry` unit and integration tests
  - New `echo-tool` unit tests (echo handler returns the input message unchanged)
  - New `echo-tool` integration tests (MCP server round-trip via `tools/call`, `tools/list` schema verification)
  - Any pre-existing tests in `agent-runtime` and `orchestrator` (currently none expected)
- `cargo run -p echo-tool` starts without errors (the process will block on stdin waiting for MCP messages; verify it starts and does not immediately crash)
- No commented-out code or debug statements remain in the `echo-tool` source files
- No unused imports, dead code, or other Clippy lint violations in the `echo-tool` crate

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo check`** from the workspace root (`/workspaces/spore`). This performs type-checking across all workspace members. If it fails, diagnose the root cause -- likely candidates include:
   - Missing or incorrect `rmcp` dependency features in `tools/echo-tool/Cargo.toml`
   - Import errors in `echo-tool` source files (e.g., wrong `rmcp` module paths)
   - Type mismatches in the `ServerHandler` or `#[tool_router]` implementations
   - The `tools/echo-tool` path not present in the root `Cargo.toml` `members` array

2. **Run `cargo clippy`** from the workspace root. This applies Rust's standard lints plus Clippy's extended checks. Pay attention to:
   - Unused imports or variables in `echo-tool` modules
   - Warnings about the `EchoTool` struct or its methods
   - Clippy suggestions for `rmcp` API usage patterns
   - Warnings in test modules (both unit and integration tests)
   - Any new warnings introduced in existing crates by the `rmcp` dependency tree

3. **Run `cargo test`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions. Verify:
   - All new `echo-tool` unit tests pass (echo handler returns input unchanged)
   - All new `echo-tool` integration tests pass (MCP round-trip via child process, `tools/list` schema)
   - All existing tests across all crates still pass (no regressions)

4. **Verify `cargo run -p echo-tool` starts** by running it and confirming the process initializes without errors. Since the echo tool uses stdio transport, it will block waiting for input. Spawn it, wait briefly for startup, then terminate it. Confirm:
   - The process starts without panics or error output on stderr (tracing initialization messages are acceptable)
   - The process does not immediately exit with a non-zero code

5. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `echo-tool` crate unless a workspace-level issue is discovered.

### Files potentially touched (fixes only, if needed)
- `tools/echo-tool/Cargo.toml` -- dependency version or feature adjustments
- `tools/echo-tool/src/main.rs` -- import, type, or implementation fixes
- `tools/echo-tool/src/echo.rs` -- tool handler fixes (if this file exists)
- `tools/echo-tool/tests/echo_server_test.rs` -- test fixture or assertion corrections
- `Cargo.toml` -- workspace member path fix (unlikely)

## Dependencies
- Blocked by: "Write unit tests for echo tool logic", "Write integration test for MCP server round-trip"
- Blocking: None (this is the final task for issue-10)

## Risks & Edge Cases
- **`rmcp` compilation issues**: The `rmcp` crate is new to the workspace (first introduced by echo-tool). If the crate version is incompatible with edition 2024 or the specified features do not exist in the pinned version, `cargo check` will fail. Mitigation: verify `rmcp` version compatibility and feature flags in `tools/echo-tool/Cargo.toml` before running verification.
- **Async runtime conflicts**: The echo-tool uses `tokio` with specific features (`macros`, `rt`, `io-std`). If a feature mismatch exists between the binary target and test targets, compilation or runtime errors may occur. Mitigation: ensure the `[dev-dependencies]` section also includes `tokio` with `macros` and `rt` features.
- **Integration test timing**: The MCP round-trip integration test spawns the echo-tool binary as a child process. If the binary takes too long to start or the test does not wait long enough, the test may fail intermittently. Mitigation: the test should use proper synchronization (e.g., waiting for the MCP handshake) rather than fixed sleeps.
- **Stdio transport interference during testing**: Since the echo-tool uses stdin/stdout as the MCP transport channel, any accidental stdout writes (e.g., from `println!` or tracing to stdout) will corrupt the transport. Mitigation: verify that tracing is configured to write to stderr only.
- **Regressions in other crates**: The verification runs workspace-wide, so a failing test in `agent-sdk`, `skill-loader`, or `tool-registry` would block this task even though it is unrelated. Mitigation: if a pre-existing test fails, confirm it also fails on the `main` branch before attributing it to echo-tool changes.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Mitigation: address each lint individually rather than blanket-suppressing with `#[allow]`.
- **Binary startup verification**: The echo-tool blocks on stdin, so `cargo run -p echo-tool` will not exit on its own. The verification must spawn the process, confirm it started, and then kill it. A naive approach of just running `cargo run` would hang indefinitely.

## Verification
- `cargo check` exits with code 0 and produces no error output
- `cargo clippy` exits with code 0 and produces no warning output (run with `-- -D warnings` to treat warnings as errors)
- `cargo test` exits with code 0, all test cases report `ok`, and the summary line shows 0 failures
- `cargo run -p echo-tool` starts without panics or immediate exit; the process can be terminated cleanly after confirming startup
- The above commands are run from the workspace root `/workspaces/spore` without any `--package` filter (except for the `cargo run` step), confirming workspace-wide health
