# Spec: Implement CargoBuildTool struct and handler

> From: .claude/tasks/issue-47.md

## Objective

Create `tools/cargo-build/src/cargo_build.rs` containing the `CargoBuildTool` struct and its MCP handler. The tool accepts a package name and an optional release flag, validates the package name to prevent command injection, shells out to `cargo build`, and returns structured JSON with the build result.

## Current State

No `tools/cargo-build/` directory exists yet. The project has established patterns in `echo-tool`, `read-file`, and `write-file` that this implementation must follow: request struct with derive macros, tool struct wrapping a `ToolRouter<Self>`, `#[tool_router]` impl block, `#[tool_handler]` ServerHandler impl, and `#[cfg(test)]` unit tests.

## Requirements

1. **Request type** -- `CargoBuildRequest` with two fields:
   - `package: String` with doc comment `/// Package name to build (passed as -p <package>)`
   - `release: Option<bool>` with doc comment `/// Whether to build in release mode`
   - Derive `Debug, serde::Deserialize, schemars::JsonSchema`

2. **Tool struct** -- `CargoBuildTool` with field `tool_router: ToolRouter<Self>`, derive `Debug, Clone`, and a `new()` constructor calling `Self::tool_router()`.

3. **Package name validation** -- Before invoking `cargo build`, validate `request.package` using a character-by-character check (no regex crate). Every character must be ASCII alphanumeric, hyphen (`-`), or underscore (`_`). The string must also be non-empty. If validation fails, return immediately with a JSON error string: `serde_json::json!({ "success": false, "stderr": "Invalid package name: <name>" }).to_string()`.

4. **Build execution** -- Use `std::process::Command::new("cargo")` with args `["build", "-p", &request.package]`. If `request.release == Some(true)`, append `"--release"` to the args. Capture output via `.output()`.

5. **Result formatting** -- On successful spawn, return `serde_json::json!({ "success": <bool>, "stdout": <String>, "stderr": <String>, "exit_code": <i32> }).to_string()` where:
   - `success` = `status.success()`
   - `stdout` = `String::from_utf8_lossy(&output.stdout)`
   - `stderr` = `String::from_utf8_lossy(&output.stderr)`
   - `exit_code` = `status.code().unwrap_or(-1)`

6. **Spawn failure** -- If `.output()` returns `Err(e)`, return `serde_json::json!({ "success": false, "stderr": format!("Failed to execute cargo: {e}") }).to_string()`.

7. **Tool annotation** -- The method is annotated `#[tool(description = "Run cargo build on a specified package and return the result")]` and named `cargo_build`.

8. **ServerHandler** -- `#[tool_handler] impl ServerHandler for CargoBuildTool` with `get_info` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.

9. **Unit tests** -- `#[cfg(test)] mod tests` with three tests:
   - `rejects_invalid_package_name` -- calls with `"foo; rm -rf /"`, parses result as JSON, asserts `success` is `false` and `stderr` contains `"Invalid package name"`.
   - `rejects_package_with_path_separator` -- calls with `"../evil"`, parses result as JSON, asserts `success` is `false` and `stderr` contains `"Invalid package name"`.
   - `validates_clean_package_name` -- calls with `"echo-tool"`, parses result as JSON, asserts the result contains the keys `success`, `stdout`, `stderr`, and `exit_code` (does not assert build success since the package may not exist in the test environment).

## Implementation Details

### Imports

```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

### Validation helper

Extract a private function `validate_package_name(name: &str) -> bool` that returns `true` only if the string is non-empty and every character satisfies `ch.is_ascii_alphanumeric() || ch == '-' || ch == '_'`. This keeps the tool method itself under 50 lines and follows single-responsibility.

### Method body sketch

The `cargo_build` method should:
1. Call `validate_package_name` -- if false, return JSON error immediately.
2. Build a `Command`, conditionally add `--release`.
3. Match on `.output()` -- `Ok(output)` produces the success JSON, `Err(e)` produces the failure JSON.

### Test helper

Define a `call_cargo_build` helper (like `call_write_file` in write-file) to reduce boilerplate in tests:

```rust
fn call_cargo_build(tool: &CargoBuildTool, package: &str, release: Option<bool>) -> String {
    tool.cargo_build(Parameters(CargoBuildRequest {
        package: package.to_string(),
        release,
    }))
}
```

Tests parse the returned string with `serde_json::from_str::<serde_json::Value>(&result).unwrap()` to assert on individual fields.

## Dependencies

- **Crate dependencies**: `rmcp`, `serde`, `schemars`, `serde_json` (all already used by sibling tools).
- **No new external crates** -- validation is done with stdlib character checks.
- **Blocked by**: `tools/cargo-build/Cargo.toml` must exist first so this file can compile.
- **Blocks**: `main.rs` (needs to import and instantiate `CargoBuildTool`) and the integration test.

## Risks & Edge Cases

- **Command injection** -- Mitigated by the character validation. `std::process::Command` also passes args as a list (not through a shell), providing a second layer of defense.
- **Empty package name** -- Covered by the non-empty check in `validate_package_name`.
- **Path separators in package name** -- Characters like `.`, `/`, `\` are rejected by the allowlist, blocking path traversal attempts like `"../evil"`.
- **Cargo not installed** -- The spawn-failure branch handles this gracefully, returning a JSON error rather than panicking.
- **Signal-killed process** -- `status.code()` returns `None` when the process is killed by a signal; `unwrap_or(-1)` handles this.
- **Large build output** -- `Command::output()` captures all stdout/stderr into memory. For very large builds this could use significant memory, but this is acceptable for an MCP tool where the caller expects a complete response.

## Verification

1. `cargo check -p cargo-build` -- confirms the file compiles.
2. `cargo test -p cargo-build` -- runs all three unit tests.
3. `cargo clippy -p cargo-build` -- no warnings.
4. Manual review that the validation function rejects injection attempts, path traversal strings, and empty strings, while accepting valid names like `"echo-tool"`, `"my_package"`, `"foo123"`.
