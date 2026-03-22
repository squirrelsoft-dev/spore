# Task Breakdown: Implement cargo_build MCP tool

> Implement `cargo_build` as a standalone Rust MCP server binary that runs `cargo build` on a specified package and returns structured output, following the echo-tool reference pattern.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/cargo-build/Cargo.toml`** `[S]`
      Copy and adapt `tools/echo-tool/Cargo.toml`. Change `name = "cargo-build"`. Keep the same dependencies: `rmcp` with `transport-io`, `server`, `macros` features; `tokio` with `macros`, `rt`, `io-std`; `serde` with `derive`; `serde_json`; `mcp-tool-harness` path dependency. Add the same `[dev-dependencies]` block with `tokio` `rt-multi-thread`, `rmcp` with `client` and `transport-child-process`, `serde_json`, and `mcp-test-utils` path dependency.
      Files: `tools/cargo-build/Cargo.toml`
      Blocking: "Implement `CargoBuildTool` struct and handler", "Write `main.rs`", "Write integration test"

- [x] **Add `"tools/cargo-build"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/cargo-build"` to the `members` list in the root `Cargo.toml`, following the same ordering pattern as the other tool entries.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `CargoBuildTool` struct and handler in `src/cargo_build.rs`** `[M]`
      Create `tools/cargo-build/src/cargo_build.rs`. Define `CargoBuildRequest` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` containing two fields: `package: String` (doc comment: `/// Package name to build (passed as -p <package>)`) and `release: Option<bool>` (doc comment: `/// Whether to build in release mode`). Define `CargoBuildTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`.

      Before invoking the build, validate the `package` field: it must match `^[a-zA-Z0-9_-]+$` (alphanumeric, hyphens, underscores only) to prevent command injection. Use a simple character check without adding a regex dependency. If validation fails, return a JSON error string immediately.

      Annotate the impl block with `#[tool_router]` and add a `cargo_build` method with `#[tool(description = "Run cargo build on a specified package and return the result")]`. The method uses `std::process::Command::new("cargo")` with args `["build", "-p", &request.package]`, conditionally appending `"--release"` if `request.release == Some(true)`. Capture output with `.output()`. Return a JSON string (via `serde_json::json!`) containing `success` (bool from `status.success()`), `stdout` (String from lossy UTF-8), `stderr` (String from lossy UTF-8), and `exit_code` (i32 from `status.code().unwrap_or(-1)`). On `Command` spawn failure, return a JSON error with `success: false` and the error message in `stderr`.

      Implement `ServerHandler` with `#[tool_handler]` returning tools-enabled capabilities.

      Add `#[cfg(test)] mod tests` with unit tests: (1) `rejects_invalid_package_name` — call with `"foo; rm -rf /"`, assert result contains an error indication; (2) `rejects_package_with_path_separator` — call with `"../evil"`, assert result contains an error; (3) `validates_clean_package_name` — call with `"echo-tool"`, assert the result is valid JSON containing expected fields (this test will actually run cargo build, so it exercises the real path).
      Files: `tools/cargo-build/src/cargo_build.rs`
      Blocked by: "Create `tools/cargo-build/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write integration test"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/cargo-build/src/main.rs`. Mirror `tools/echo-tool/src/main.rs` exactly: declare `mod cargo_build;`, use the struct, and call `mcp_tool_harness::serve_stdio_tool(CargoBuildTool::new(), "cargo-build").await`. The file should be under 10 lines.
      Files: `tools/cargo-build/src/main.rs`
      Blocked by: "Implement `CargoBuildTool` struct and handler in `src/cargo_build.rs`"
      Blocking: "Write integration test"

## Group 3 — Integration test

_Depends on: Group 2_

- [x] **Write integration test in `tests/cargo_build_server_test.rs`** `[M]`
      Create `tools/cargo-build/tests/cargo_build_server_test.rs`. Mirror the pattern from `tools/echo-tool/tests/echo_server_test.rs`. Use `env!("CARGO_BIN_EXE_cargo-build")` to spawn the binary. Write these tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):

      (1) `tools_list_returns_cargo_build_tool` — use `mcp_test_utils::assert_single_tool` to verify the tool name is `"cargo_build"`, description contains `"cargo build"`, and parameters include `["package", "release"]`.

      (2) `tools_call_builds_echo_tool_successfully` — call the tool with `{"package": "echo-tool"}`, parse the response text as JSON, assert `success` is `true` and `exit_code` is `0`.

      (3) `tools_call_returns_error_for_nonexistent_package` — call with `{"package": "nonexistent-package-xyz"}`, parse JSON, assert `success` is `false` and `stderr` is non-empty.

      (4) `tools_call_rejects_invalid_package_name` — call with `{"package": "foo;bar"}`, assert the response indicates an error (validation rejection before command execution).

      Files: `tools/cargo-build/tests/cargo_build_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: None

## Group 4 — Verification

_Depends on: Groups 1–3_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p cargo-build`, then `cargo test -p cargo-build`, then `cargo clippy -p cargo-build`, then `cargo check` (workspace-wide) to confirm no regressions. All acceptance criteria from the issue must pass: build succeeds, tests pass, successfully builds a target package with structured output, returns compiler errors in `stderr` on failure, tool is named `cargo_build` in MCP tools/list.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

---

## Implementation Notes

1. **No new dependencies**: All dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `mcp-tool-harness`) already appear in `tools/echo-tool/Cargo.toml`. No regex crate is needed — use a simple `.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_')` check for package name validation.

2. **Security**: The `package` input is validated to contain only `[a-zA-Z0-9_-]` characters before being passed to `std::process::Command`. This prevents shell injection. The command is invoked directly via `Command::new("cargo")` (not via a shell), which provides an additional layer of safety since arguments are not interpreted by a shell.

3. **Return format**: The tool returns a JSON string (not a Rust `Result`) because the `#[tool_router]` macro expects `String` return type, matching the pattern established by `echo-tool` and `read-file`. The JSON structure (`success`, `stdout`, `stderr`, `exit_code`) provides structured data that calling agents can parse.

4. **Binary vs tool name**: The `[package] name = "cargo-build"` means the binary is `cargo-build` and the env macro is `CARGO_BIN_EXE_cargo-build`. The MCP tool name exposed over the protocol is `cargo_build` (the snake_case method name from the `#[tool_router]` macro).

5. **Integration test uses real cargo**: The test `tools_call_builds_echo_tool_successfully` invokes a real `cargo build -p echo-tool`. This is acceptable because `echo-tool` is a workspace member that is already built during test runs.

6. **Reference files**:
   - `tools/echo-tool/src/echo.rs` — Reference pattern for tool struct, macros, unit tests
   - `tools/echo-tool/Cargo.toml` — Template for dependencies
   - `tools/echo-tool/tests/echo_server_test.rs` — Pattern for integration tests
   - `crates/mcp-test-utils/src/lib.rs` — Test utilities (spawn_mcp_client, assert_single_tool)
