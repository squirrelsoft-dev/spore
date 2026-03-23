# Spec: Run verification suite

> From: .claude/tasks/issue-49.md

## Objective
Run the package-scoped verification commands (`cargo build -p docker-push`, `cargo test -p docker-push`, `cargo clippy -p docker-push`) followed by a workspace-wide `cargo check` to confirm that the `docker-push` MCP tool crate compiles cleanly, passes all tests, produces no lint warnings, and introduces no regressions across the workspace. This is the final gate task for issue-49 -- it validates that every preceding task (scaffold, implementation, integration tests, README) integrates correctly.

## Current State
The workspace is defined in the root `Cargo.toml` with `resolver = "2"`. By the time this task runs, the workspace members will include all existing crates plus the new `tools/docker-push` entry:
- `crates/agent-sdk`, `crates/skill-loader`, `crates/tool-registry`, `crates/agent-runtime`, `crates/mcp-tool-harness`, `crates/orchestrator`, `crates/mcp-test-utils`
- `tools/echo-tool`, `tools/read-file`, `tools/write-file`, `tools/validate-skill`, `tools/cargo-build`
- `tools/docker-push` -- new MCP docker push tool server (added by preceding issue-49 tasks)

By the time this task runs, the preceding issue-49 tasks will have:
1. Created `tools/docker-push/Cargo.toml` with dependencies on `mcp-tool-harness`, `rmcp` (with `transport-io`, `server`, `macros` features), `tokio`, `serde`, and `serde_json`
2. Added `"tools/docker-push"` to the workspace `members` array in the root `Cargo.toml`
3. Implemented the `DockerPushTool` struct with `#[tool_router]` and `ServerHandler` in `tools/docker-push/src/docker_push.rs`, including input validation, registry URL resolution, digest extraction, and six unit tests
4. Created `tools/docker-push/src/main.rs` mirroring the `cargo-build` pattern
5. Written integration tests in `tools/docker-push/tests/docker_push_server_test.rs` (four tests: `tools/list` verification, invalid image error, valid image structured JSON, empty image error)
6. Created `tools/docker-push/README.md`

## Requirements
- `cargo build -p docker-push` succeeds with zero errors
- `cargo test -p docker-push` succeeds with all tests passing, including:
  - Unit tests: `rejects_empty_image`, `rejects_image_with_shell_metachar`, `rejects_image_with_pipe`, `accepts_valid_image_reference`, `registry_url_is_prepended`, `digest_extraction_from_output`
  - Integration tests: `tools_list_returns_docker_push_tool`, `tools_call_with_invalid_image_returns_error`, `tools_call_with_valid_image_returns_structured_json`, `tools_call_with_empty_image_returns_error`
- `cargo clippy -p docker-push` succeeds with zero warnings (no `#[allow(...)]` suppressions added solely to silence legitimate warnings)
- `cargo check` (workspace-wide) succeeds with zero errors, confirming no regressions in any existing crate
- All acceptance criteria from the issue are satisfied:
  - Build succeeds
  - Tests pass
  - Tool returns structured JSON with `success`, `image`, `digest`, and `push_log` fields
  - Tool returns structured errors on auth/network/validation failure
  - Tool is named `docker_push` in MCP `tools/list`
- No commented-out code or debug statements remain in the `docker-push` source files
- No unused imports, dead code, or other Clippy lint violations in the `docker-push` crate

## Implementation Details
This task does not create or modify source files. It is a verification-only task. The steps are:

1. **Run `cargo build -p docker-push`** from the workspace root. This compiles only the `docker-push` crate and its dependencies. If it fails, diagnose the root cause -- likely candidates include:
   - Missing or incorrect dependency features in `tools/docker-push/Cargo.toml`
   - Import errors in `docker_push.rs` or `main.rs` (e.g., wrong `rmcp` module paths)
   - Type mismatches in the `ServerHandler` or `#[tool_router]` implementations
   - The `tools/docker-push` path not present in the root `Cargo.toml` `members` array

2. **Run `cargo test -p docker-push`** from the workspace root. This compiles and executes all `#[test]` and `#[tokio::test]` functions within the `docker-push` crate. Verify:
   - All six unit tests pass (input validation, registry URL prepending, digest extraction)
   - All four integration tests pass (MCP `tools/list` schema, invalid image error, valid image structured JSON, empty image error)
   - The integration test `tools_list_returns_docker_push_tool` confirms the tool is named `docker_push` (snake_case) in the MCP `tools/list` response

3. **Run `cargo clippy -p docker-push`** from the workspace root. This applies Rust's standard lints plus Clippy's extended checks scoped to the `docker-push` crate. Pay attention to:
   - Unused imports or variables
   - Warnings about the `DockerPushTool` struct or its methods
   - Clippy suggestions for `std::process::Command` usage patterns
   - Warnings in test modules (both unit and integration tests)

4. **Run `cargo check`** (workspace-wide, no `-p` filter) from the workspace root. This performs type-checking across all workspace members. This confirms that adding `docker-push` to the workspace and its dependency tree does not break any existing crate.

5. If any step fails, **diagnose before fixing** (per project rules). Explain the root cause, then apply the minimal fix to the relevant file(s) introduced by the preceding tasks. Do not modify files outside the `docker-push` crate unless a workspace-level issue is discovered.

### Files potentially touched (fixes only, if needed)
- `tools/docker-push/Cargo.toml` -- dependency version or feature adjustments
- `tools/docker-push/src/docker_push.rs` -- import, type, validation logic, or handler fixes
- `tools/docker-push/src/main.rs` -- module declaration or startup fixes
- `tools/docker-push/tests/docker_push_server_test.rs` -- test fixture or assertion corrections
- `Cargo.toml` -- workspace member path fix (unlikely)

## Dependencies
- Blocked by: All preceding issue-49 tasks ("Create `tools/docker-push/Cargo.toml`", "Add `tools/docker-push` to workspace `Cargo.toml`", "Implement `DockerPushTool` struct and handler", "Write `main.rs`", "Write integration tests", "Write README")
- Blocking: None (this is the final task for issue-49)

## Risks & Edge Cases
- **Docker availability in tests**: The `accepts_valid_image_reference` unit test and the `tools_call_with_valid_image_returns_structured_json` integration test invoke the tool with image references that do not exist locally. These tests must not assert `success: true` because Docker may not be installed or the daemon may not be running. They should only verify that the response is well-formed JSON containing the four expected fields.
- **Binary name vs package name**: The package is `docker-push` (hyphen) but the MCP tool name is `docker_push` (underscore). The integration test binary env macro is `CARGO_BIN_EXE_docker-push`. A mismatch in any of these would cause test failures.
- **`schemars` dependency**: The `DockerPushRequest` struct uses `#[derive(schemars::JsonSchema)]` to generate the MCP tool input schema. If `schemars` is not available transitively through `mcp-tool-harness` or `rmcp`, `cargo build` will fail. Verify the dependency chain provides `schemars`.
- **Shell metacharacter validation completeness**: The input validation must reject all specified metacharacters (`;`, `|`, `&`, `$`, backtick, `(`, `)`, `{`, `}`, `<`, `>`, `!`, `\n`). If a character is missed, the unit tests will catch it, but the security gap would exist for any untested characters.
- **Regressions in other crates**: The workspace-wide `cargo check` may surface pre-existing issues in other crates unrelated to docker-push. If a failure is found, confirm it also exists on the `main` branch before attributing it to docker-push changes.
- **Integration test child process lifecycle**: Each integration test spawns the `docker-push` binary and must call `client.cancel().await` at the end. If a test panics before reaching the cancel call, orphan processes could accumulate. This is a known pattern across all MCP tool integration tests in this workspace.
- **Edition 2024 lint behavior**: The workspace uses `edition = "2024"`, which may trigger lints not present in older editions. Address each lint individually rather than blanket-suppressing with `#[allow]`.
- **Digest extraction edge cases**: The digest extraction helper parses `docker push` output for `digest: sha256:<hex>`. If the Docker CLI output format changes across versions, this parsing could break. The unit test `digest_extraction_from_output` validates the expected format, but real-world output variations are not covered.

## Verification
- `cargo build -p docker-push` exits with code 0 and produces no error output
- `cargo test -p docker-push` exits with code 0, all test cases (unit and integration) report `ok`, and the summary line shows 0 failures
- `cargo clippy -p docker-push -- -D warnings` exits with code 0 and produces no warning output
- `cargo check` (workspace-wide) exits with code 0 and produces no error output
- The integration test `tools_list_returns_docker_push_tool` confirms the tool name is `docker_push` in the MCP `tools/list` response
- The integration test `tools_call_with_invalid_image_returns_error` confirms structured error JSON with `success: false` and `push_log` containing "Invalid image reference"
- The integration test `tools_call_with_valid_image_returns_structured_json` confirms the response contains all four fields: `success`, `image`, `digest`, `push_log`
