# Spec: Implement `DockerPushTool` struct and handler in `src/docker_push.rs`

> From: .claude/tasks/issue-49.md

## Objective
Create `tools/docker-push/src/docker_push.rs` containing the core MCP tool implementation for pushing Docker images to a container registry. This file defines the request struct, input validation, registry URL resolution, subprocess invocation, digest extraction, and structured JSON response. The implementation follows the `cargo-build` tool pattern exactly, since both tools invoke an external command via `std::process::Command` and return structured JSON.

## Current State
- `tools/cargo-build/src/cargo_build.rs` exists as the reference pattern. It defines `CargoBuildRequest`, `CargoBuildTool`, validates input, invokes `cargo build` via `std::process::Command`, and returns structured JSON. It uses `#[tool_router]`, `#[tool_handler]`, and `ServerHandler` from `rmcp`.
- `tools/cargo-build/src/main.rs` is 7 lines: declares `mod cargo_build;`, imports the tool struct, and calls `mcp_tool_harness::serve_stdio_tool`.
- `crates/mcp-tool-harness/src/lib.rs` provides `serve_stdio_tool<T: ServerHandler>` which initializes tracing to stderr and serves the tool over stdio transport.
- The `tools/docker-push/` directory and `Cargo.toml` do not yet exist. This task is blocked by the "Create `tools/docker-push/Cargo.toml`" task which will provide identical dependencies to `tools/cargo-build/Cargo.toml` (with `schemars` re-exported by `rmcp`).
- No `docker_push.rs` file exists anywhere in the workspace.

## Requirements

### Request struct: `DockerPushRequest`
- Derive `Debug`, `serde::Deserialize`, `schemars::JsonSchema`.
- Field `image: String` with doc comment `/// Full image reference (e.g., ghcr.io/spore/spore-agent:0.1)`.
- Field `registry_url: Option<String>` with doc comment `/// Override registry URL; falls back to REGISTRY_URL env var`.

### Tool struct: `DockerPushTool`
- Fields: `tool_router: ToolRouter<Self>`.
- Derive `Debug, Clone` (matching the `CargoBuildTool` pattern).
- Constructor `new()` that calls `Self::tool_router()`.

### Input validation
- Reject empty `image` strings.
- Reject `image` strings containing any shell metacharacter: `;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `{`, `}`, `<`, `>`, `!`, `\n`.
- Accept only valid Docker image reference characters: alphanumeric, `.`, `-`, `_`, `/`, `:`.
- Use simple character iteration (no regex dependency).
- On validation failure, return `{"success": false, "image": "<input>", "digest": "", "push_log": "Invalid image reference: <reason>"}`.

### Registry URL resolution
- If `registry_url` is `Some(url)`, use that URL.
- Otherwise, attempt `std::env::var("REGISTRY_URL")`. If `Ok(url)`, use that.
- If a registry URL is available and the `image` does not already start with it, prepend: `format!("{registry_url}/{image}")`.
- If no registry URL is available, proceed with the image as-is.
- Extract this logic into a standalone helper function (`resolve_image_ref` or similar) so it can be unit-tested without invoking Docker.

### Docker push invocation
- Use `std::process::Command::new("docker").args(["push", &final_image_ref]).output()`.
- On `Command` spawn failure (`.output()` returns `Err`), return `{"success": false, "image": "<final_ref>", "digest": "", "push_log": "Failed to execute docker: <error>"}`.

### Digest extraction
- Combine stdout and stderr into a single string (stdout first, then stderr).
- Search line by line for a substring containing `digest: sha256:`.
- Extract the full `sha256:<hex>` token (the word immediately following `digest: `).
- If no match, return empty string.
- Extract this logic into a standalone helper function (`extract_digest` or similar) so it can be unit-tested without invoking Docker.

### Return format
- On successful `Command` execution (regardless of exit code): `serde_json::json!({"success": output.status.success(), "image": final_image_ref, "digest": extracted_digest, "push_log": combined_output})`.
- Return value is `.to_string()` (the tool method returns `String`, matching cargo-build).

### ServerHandler implementation
- `#[tool_handler] impl ServerHandler for DockerPushTool` with `get_info()` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.

### Unit tests (`#[cfg(test)] mod tests`)
Six tests, none of which require Docker to be installed:

1. **`rejects_empty_image`** -- Call `docker_push` with `image: ""`. Assert `success: false` and `push_log` contains "Invalid image reference".

2. **`rejects_image_with_shell_metachar`** -- Call with `image: "foo;bar"`. Assert `success: false` and `push_log` contains "Invalid image reference".

3. **`rejects_image_with_pipe`** -- Call with `image: "foo|bar"`. Assert `success: false` and `push_log` contains "Invalid image reference".

4. **`accepts_valid_image_reference`** -- Call with `image: "ghcr.io/spore/spore-agent:0.1"`. Assert the result is valid JSON containing all four fields (`success`, `image`, `digest`, `push_log`). Note: this test will attempt to execute `docker push` which may fail if Docker is not installed, but the response still has the expected JSON structure.

5. **`registry_url_is_prepended`** -- Call the `resolve_image_ref` helper directly. Test cases:
   - `image = "spore-agent:0.1"`, `registry_url = Some("ghcr.io/spore")` produces `"ghcr.io/spore/spore-agent:0.1"`.
   - `image = "ghcr.io/spore/spore-agent:0.1"`, `registry_url = Some("ghcr.io/spore")` does not double the prefix (returns unchanged).

6. **`digest_extraction_from_output`** -- Call the `extract_digest` helper directly.
   - Input `"latest: digest: sha256:abc123def456 size: 1234"` returns `"sha256:abc123def456"`.
   - Input with no digest line returns `""`.

### Test helper pattern
Follow the `cargo_build.rs` test pattern: define a helper function `call_docker_push(tool, image, registry_url)` that constructs a `DockerPushRequest` and invokes the tool method via `Parameters(...)`. Parse the returned string as `serde_json::Value` for assertions.

## Implementation Details

### Imports
```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

### Validation function: `validate_image_ref`
A standalone `fn validate_image_ref(image: &str) -> Result<(), String>` that:
1. Returns `Err("image must not be empty")` if empty.
2. Iterates over each character. If any character is not alphanumeric and not in the set `.`, `-`, `_`, `/`, `:`, returns `Err` with a description of the invalid character.
3. Returns `Ok(())` on success.

The shell metacharacter check is implicitly handled: characters like `;`, `|`, `&`, `$`, etc. are not in the allowed set and will be caught by the character allowlist. This is simpler and more secure than maintaining a separate blocklist.

### Registry URL resolution function: `resolve_image_ref`
A standalone `fn resolve_image_ref(image: &str, registry_url: Option<&str>) -> String` that:
1. Determines the effective registry URL: `registry_url` parameter first, then `std::env::var("REGISTRY_URL").ok()`.
2. If a registry URL is available and `image` does not start with it, returns `format!("{}/{}", registry_url, image)`.
3. Otherwise returns `image.to_string()`.

For testability, consider accepting the env var as a parameter or using a separate internal function that takes all inputs explicitly, so the unit test for `registry_url_is_prepended` does not depend on the environment variable.

### Digest extraction function: `extract_digest`
A standalone `fn extract_digest(output: &str) -> String` that:
1. Iterates over lines.
2. For each line, searches for the substring `"digest: sha256:"`.
3. If found, extracts the token starting at `sha256:` up to the next whitespace.
4. Returns the token, or empty string if not found.

### Tool method: `docker_push`
```rust
#[tool(description = "Push a tagged Docker image to a container registry")]
fn docker_push(&self, Parameters(request): Parameters<DockerPushRequest>) -> String
```

Follows this flow:
1. Validate `request.image` via `validate_image_ref`. On failure, return error JSON.
2. Resolve final image ref via `resolve_image_ref`.
3. Invoke `Command::new("docker").args(["push", &final_ref]).output()`.
4. On spawn error, return error JSON.
5. On success, combine stdout + stderr, extract digest, return result JSON.

### Line budget
- Imports: ~6 lines
- `DockerPushRequest` struct: ~7 lines
- `DockerPushTool` struct + derives: ~5 lines
- `validate_image_ref`: ~15 lines
- `resolve_image_ref`: ~15 lines
- `extract_digest`: ~12 lines
- `#[tool_router] impl` with `new()` + `docker_push()`: ~35 lines
- `#[tool_handler] impl ServerHandler`: ~8 lines
- Tests module: ~80 lines
- Total: ~183 lines

Each function stays well under the 50-line limit from the project rules.

## Dependencies
- **Blocked by:** "Create `tools/docker-push/Cargo.toml`" -- the crate must exist before this file can be compiled.
- **Blocking:** "Write `main.rs`" (which declares `mod docker_push;` and uses `DockerPushTool`), "Write integration tests" (which spawn the binary and exercise the tool over MCP).
- **No new crate dependencies.** All imports (`rmcp`, `serde`, `serde_json`, `schemars` via rmcp re-export) are already specified in the `Cargo.toml` created by the scaffolding task, mirroring `tools/cargo-build/Cargo.toml`.

## Risks & Edge Cases

1. **Docker not installed in test/CI environment.** The `accepts_valid_image_reference` unit test invokes `docker push` via `Command`. If Docker is not installed, `Command::new("docker").output()` returns `Err` and the tool returns the "Failed to execute docker" error JSON. The test must not assert `success: true` -- it should only assert that the response is valid JSON with the four expected fields. The other five unit tests (validation, registry URL, digest extraction) do not require Docker.

2. **Registry URL with trailing slash.** If `registry_url` ends with `/`, the resolved image would have a double slash (e.g., `ghcr.io/spore//image:tag`). Consider trimming trailing slashes from the registry URL. The task description does not specify this, but it is a defensive measure.

3. **Image already contains registry prefix partially.** The `starts_with` check covers exact prefix matching. If the image is `ghcr.io/spore-other/image:tag` and the registry URL is `ghcr.io/spore`, the `starts_with` check would match incorrectly. However, this is an unlikely edge case and the task description specifies using `starts_with`, so follow that behavior.

4. **Environment variable leaking into tests.** The `resolve_image_ref` function reads `REGISTRY_URL` from the environment. If this env var is set in the test runner, it could affect tests unexpectedly. Mitigation: the `registry_url_is_prepended` test should call the helper with an explicit `registry_url` parameter (testing the `Some` path), and should not depend on the env var path for correctness. If the env var path needs testing, use `std::env::set_var`/`remove_var` in a dedicated test (noting that this is not thread-safe).

5. **Large Docker output.** `docker push` can produce verbose output (multiple layers being pushed). The tool captures all of stdout and stderr via `.output()`, which buffers everything in memory. For typical image pushes this is acceptable, but extremely large output could be a concern. This is not addressed in the task and is unlikely in practice.

6. **`schemars` re-export availability.** The `DockerPushRequest` derives `schemars::JsonSchema`. The `rmcp` crate re-exports `schemars`, matching the `cargo-build` pattern which uses `schemars` without a direct dependency. If the re-export is not available, `schemars` would need to be added to `Cargo.toml`. The scaffolding task should match `cargo-build` exactly, so this should not be an issue.

7. **MCP tool name.** The `#[tool_router]` macro derives the tool name from the method name. The method `docker_push` produces tool name `docker_push` (snake_case). This matches the convention described in the task breakdown (binary name `docker-push`, MCP tool name `docker_push`).

## Verification

1. **Compilation:** `cargo check -p docker-push` succeeds with no errors.
2. **Lint:** `cargo clippy -p docker-push` produces no warnings.
3. **Unit tests:** `cargo test -p docker-push -- --lib` passes all six unit tests:
   - `rejects_empty_image`
   - `rejects_image_with_shell_metachar`
   - `rejects_image_with_pipe`
   - `accepts_valid_image_reference`
   - `registry_url_is_prepended`
   - `digest_extraction_from_output`
4. **Line count:** Each function in `docker_push.rs` is under 50 lines. The file overall is under 200 lines.
5. **No commented-out code or debug statements** in the final file.
6. **Pattern conformance:** The file structure (imports, request struct, tool struct, `#[tool_router]` impl, `#[tool_handler]` impl, `#[cfg(test)]` mod) mirrors `tools/cargo-build/src/cargo_build.rs` exactly.
7. **Workspace check:** `cargo check` (full workspace) succeeds with no regressions.
