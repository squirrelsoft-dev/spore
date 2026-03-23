# Spec: Implement `DockerBuildTool` struct and handler

> From: .claude/tasks/issue-48.md

## Objective

Create `tools/docker-build/src/docker_build.rs` containing the `DockerBuildTool` struct and its MCP handler. The tool accepts a build context path, an image tag, optional build arguments, and an optional Dockerfile path. It performs security-critical input validation on all fields, shells out to `docker build`, parses the image ID from build output, and returns structured JSON with the result.

## Current State

The project has established MCP tool patterns in `cargo-build`, `echo-tool`, `read-file`, and `write-file`. Each tool follows the same structure: a request struct with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]`, a tool struct wrapping `ToolRouter<Self>`, a `#[tool_router]` impl block with a `#[tool]`-annotated method, a `#[tool_handler]` ServerHandler impl, and `#[cfg(test)]` unit tests. The `CargoBuildTool` in `tools/cargo-build/src/cargo_build.rs` is the closest reference -- it validates input, constructs a `std::process::Command`, captures output, and returns JSON. The `DockerBuildTool` follows this same pattern but with more complex validation (path traversal checks, shell metacharacter rejection) and output parsing (image ID extraction).

## Requirements

1. **Request type** -- `DockerBuildRequest` with four fields:
   - `context: String` with doc comment `/// Build context path (required)`
   - `tag: String` with doc comment `/// Image tag (required)`
   - `build_args: Option<std::collections::HashMap<String, String>>` with doc comment `/// Optional build arguments`
   - `dockerfile: Option<String>` with doc comment `/// Optional Dockerfile path`
   - Derive `Debug, serde::Deserialize, schemars::JsonSchema`

2. **Tool struct** -- `DockerBuildTool` with field `tool_router: ToolRouter<Self>`, derive `Debug, Clone`, and a `new()` constructor calling `Self::tool_router()`.

3. **Context path validation (security-critical)**:
   - Reject if `context` contains `..` path segments before any canonicalization (check for `..` as a standalone path component, not just as a substring).
   - Canonicalize the path with `std::fs::canonicalize`. If canonicalization fails (path does not exist), return a validation error.
   - Verify the canonicalized path starts with `std::env::current_dir()` (or a configured project root). Reject if the resolved path escapes the working directory.
   - Return JSON error: `{ "success": false, "build_log": "Invalid context path: <reason>" }`.

4. **Tag validation**:
   - Only allow characters matching `[a-zA-Z0-9._:/-]`. Reject if any character falls outside this set.
   - The tag must be non-empty.
   - Return JSON error: `{ "success": false, "build_log": "Invalid tag: <tag>" }`.

5. **Dockerfile path validation** (if provided):
   - Apply the same path traversal checks as `context` (reject `..` segments, canonicalize, verify within working directory).
   - Return JSON error: `{ "success": false, "build_log": "Invalid dockerfile path: <reason>" }`.

6. **Build args validation** (if provided):
   - Reject any key or value containing shell metacharacters: `;`, `&`, `|`, `$`, backtick, `\n`, `\r`, `(`, `)`, `{`, `}`, `<`, `>`, `'`, `"`, `\\`.
   - Keys must also be non-empty.
   - Return JSON error: `{ "success": false, "build_log": "Invalid build argument: <reason>" }`.

7. **Build execution**:
   - Use `std::process::Command::new("docker")` with base args `["build", "-t", &tag]`.
   - If `dockerfile` is provided, append `["-f", &dockerfile]` before the context argument.
   - For each entry in `build_args`, append `["--build-arg", &format!("{key}={value}")]`.
   - Append `&context` as the final argument.
   - Capture output with `.output()`.

8. **Output parsing**:
   - On successful spawn, combine stdout and stderr into a single `build_log` string.
   - Extract image ID by searching for a line matching either:
     - `Successfully built <id>` (legacy builder) -- capture `<id>`
     - `writing image sha256:<id>` (BuildKit) -- capture the sha256 prefix (first 12+ chars)
   - Return JSON: `{ "success": <bool>, "image_id": "<extracted or empty>", "tag": "<tag>", "build_log": "<combined output>" }`.
   - On spawn failure (`Err(e)`), return: `{ "success": false, "image_id": "", "tag": "<tag>", "build_log": "Failed to execute docker: <error>" }`.

9. **Tool annotation** -- The method is annotated `#[tool(description = "Run docker build and return the result with image ID")]` and named `docker_build`.

10. **ServerHandler** -- `#[tool_handler] impl ServerHandler for DockerBuildTool` with `get_info` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.

11. **Unit tests** -- `#[cfg(test)] mod tests` with four tests (details in Verification section).

## Implementation Details

### Imports

```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use std::collections::HashMap;
```

### Validation helpers

Extract private functions to keep the tool method under 50 lines and maintain single responsibility:

- `validate_path(path: &str, label: &str) -> Result<std::path::PathBuf, String>` -- Checks for `..` segments by splitting on path separators (`/` and `\`) and checking if any component equals `..`. Then calls `std::fs::canonicalize`. Then verifies the result starts with `std::env::current_dir().unwrap()`. Returns `Ok(canonicalized)` or `Err(reason)`.

- `validate_tag(tag: &str) -> bool` -- Returns `true` only if non-empty and every character satisfies `ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | ':' | '/' | '-')`.

- `validate_build_args(args: &HashMap<String, String>) -> Result<(), String>` -- Iterates all keys and values, checking for shell metacharacters and newlines. Returns `Ok(())` or `Err(description)`.

- `has_shell_metacharacters(s: &str) -> bool` -- Returns `true` if `s` contains any of: `;`, `&`, `|`, `$`, backtick, `\n`, `\r`, `(`, `)`, `{`, `}`, `<`, `>`, `'`, `"`, `\\`.

- `extract_image_id(output: &str) -> String` -- Scans lines for `Successfully built ` or `writing image sha256:`. Returns the extracted ID or an empty string.

### Method body sketch

The `docker_build` method should:
1. Call `validate_path(&request.context, "context")` -- if Err, return JSON error.
2. Call `validate_tag(&request.tag)` -- if false, return JSON error.
3. If `request.dockerfile` is `Some`, call `validate_path` on it -- if Err, return JSON error.
4. If `request.build_args` is `Some`, call `validate_build_args` -- if Err, return JSON error.
5. Build `Command::new("docker")` with args `["build", "-t", &request.tag]`.
6. Conditionally add `["-f", &dockerfile]`.
7. Loop over build_args appending `["--build-arg", "{k}={v}"]`.
8. Append context as the last positional arg.
9. Match on `.output()` -- on `Ok`, extract image ID, build JSON response; on `Err`, return failure JSON.

### Test helper

```rust
fn call_docker_build(
    tool: &DockerBuildTool,
    context: &str,
    tag: &str,
    build_args: Option<HashMap<String, String>>,
    dockerfile: Option<&str>,
) -> String {
    tool.docker_build(Parameters(DockerBuildRequest {
        context: context.to_string(),
        tag: tag.to_string(),
        build_args,
        dockerfile: dockerfile.map(|s| s.to_string()),
    }))
}
```

Tests parse the returned string with `serde_json::from_str::<serde_json::Value>(&result).unwrap()` to assert on individual fields.

### JSON output field names

All responses use exactly these field names: `success` (bool), `image_id` (string), `tag` (string), `build_log` (string). This is distinct from the `cargo_build` tool which uses `stdout`/`stderr`/`exit_code` -- the docker tool merges output into a single `build_log` and adds the parsed `image_id`.

## Dependencies

- **Crate dependencies**: `rmcp`, `serde`, `schemars`, `serde_json` (all already used by sibling tools).
- **No new external crates** -- validation uses stdlib character checks and path operations. Image ID extraction uses simple string searching (no regex needed).
- **Blocked by**: "Create `tools/docker-build/Cargo.toml`" must exist first so this file can compile.
- **Blocking**: "Write `main.rs`" (needs to import and instantiate `DockerBuildTool`) and "Write integration tests".

## Risks & Edge Cases

- **Path traversal attacks** -- Mitigated by rejecting `..` segments before canonicalization and verifying canonicalized paths are within the working directory. Symlinks that escape the working directory are caught by checking the canonicalized result.
- **Command injection via tag** -- Mitigated by the strict character allowlist. `std::process::Command` also passes args as a list (not through a shell), providing a second layer of defense.
- **Command injection via build args** -- Mitigated by rejecting shell metacharacters in both keys and values. Even though `Command` does not use a shell, this prevents misuse if the args are ever logged or processed by other tools.
- **Docker not installed** -- The spawn-failure branch handles this gracefully, returning structured JSON with `success: false` rather than panicking.
- **BuildKit vs legacy builder output** -- The image ID extraction handles both formats (`Successfully built` and `writing image sha256:`). If neither pattern is found (e.g., future Docker output format changes), `image_id` is returned as an empty string -- the caller can still use the `tag` field.
- **Non-existent context path** -- `std::fs::canonicalize` will fail, caught by the validation step with a clear error message.
- **Large build output** -- `Command::output()` captures all stdout/stderr into memory. Acceptable for an MCP tool where the caller expects a complete response.
- **Race conditions on paths** -- A validated path could be modified between validation and Docker invocation (TOCTOU). This is an inherent limitation; Docker itself will fail with a clear error if the path disappears.
- **Signal-killed process** -- Not directly relevant since we do not report `exit_code`; we rely on `status.success()` for the `success` field.
- **Empty build_args HashMap** -- An empty `Some(HashMap::new())` should be treated the same as `None` (no `--build-arg` flags added). The iteration loop handles this naturally.

## Verification

1. `cargo check -p docker-build` -- confirms the file compiles.
2. `cargo test -p docker-build` -- runs all four unit tests:
   - `rejects_context_with_path_traversal` -- calls with `context: "../../etc"`, `tag: "test:latest"`, no build_args, no dockerfile. Parses JSON, asserts `success` is `false` and `build_log` contains a path validation error message.
   - `rejects_tag_with_shell_metacharacters` -- calls with `context: "."`, `tag: "foo;echo injected"`, no build_args, no dockerfile. Parses JSON, asserts `success` is `false` and `build_log` contains `"Invalid tag"`.
   - `rejects_invalid_build_arg_keys` -- calls with `context: "."`, `tag: "test:latest"`, `build_args: Some(HashMap::from([("key;bad".into(), "val".into())]))`, no dockerfile. Parses JSON, asserts `success` is `false` and `build_log` contains `"Invalid build argument"`.
   - `validates_clean_inputs` -- calls with `context: "."`, `tag: "test:latest"`, no build_args, no dockerfile. Parses JSON, asserts the result contains keys `success`, `image_id`, `tag`, and `build_log`. Does not assert `success: true` since Docker may not be installed in the test environment.
3. `cargo clippy -p docker-build` -- no warnings.
4. Manual review that validation functions correctly reject path traversal, shell injection, and metacharacter attacks while accepting legitimate inputs.
