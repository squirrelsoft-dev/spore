# Task Breakdown: Implement docker_build MCP tool

> Implement `docker_build` as a standalone Rust MCP server binary that builds Docker images from a Dockerfile and context directory, following the established tool pattern (cargo-build, echo-tool).

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/docker-build/Cargo.toml`** `[S]`
      Copy `tools/cargo-build/Cargo.toml` and change `name = "docker-build"`. Keep the same dependency set: `mcp-tool-harness` (path), `rmcp` with `transport-io`/`server`/`macros`, `tokio` with `macros`/`rt`/`io-std`, `serde` with `derive`, `serde_json`. Dev-dependencies: `mcp-test-utils` (path), `tokio` with `rt-multi-thread`, `rmcp` with `client`/`transport-child-process`, `serde_json`.
      Files: `tools/docker-build/Cargo.toml`
      Blocking: "Implement `DockerBuildTool` struct and handler", "Write `main.rs`", "Write integration tests"

- [x] **Add `"tools/docker-build"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/docker-build"` to the `members` list in the root `Cargo.toml`, after the existing `"tools/cargo-build"` entry.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `DockerBuildTool` struct and handler in `src/docker_build.rs`** `[M]`
      Create `tools/docker-build/src/docker_build.rs`. Define `DockerBuildRequest` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` containing four fields:
      - `context: String` — build context path (required)
      - `tag: String` — image tag (required)
      - `build_args: Option<std::collections::HashMap<String, String>>` — optional build arguments
      - `dockerfile: Option<String>` — optional Dockerfile path

      Define `DockerBuildTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`.

      **Input validation (security-critical):**
      - Validate `context` path: canonicalize it and verify it exists and is within the current working directory (or a configured project root) to prevent building from arbitrary paths. Reject paths containing `..` segments before canonicalization as an extra safeguard.
      - Validate `tag`: only allow `[a-zA-Z0-9._:/-]` characters to prevent injection through the tag argument.
      - Validate `dockerfile` (if provided): same path traversal checks as `context`.
      - Validate `build_args` keys and values: reject any containing shell metacharacters or newlines.

      **Execution:** Use `std::process::Command::new("docker")` with args `["build", "-t", &tag, &context]`. If `dockerfile` is provided, prepend `["-f", &dockerfile]`. For each entry in `build_args`, append `["--build-arg", &format!("{key}={value}")]`. Capture output with `.output()`.

      **Output parsing:** After a successful `docker build`, extract the image ID from the build output (parse the line matching `Successfully built <id>` or `writing image sha256:<id>`). Return JSON with `success` (bool), `image_id` (string), `tag` (string), `build_log` (string from combined stdout/stderr). On failure, return JSON with `success: false` and the error in `build_log`.

      Implement `ServerHandler` with `#[tool_handler]` returning tools-enabled capabilities.

      **Unit tests** in `#[cfg(test)] mod tests`:
      1. `rejects_context_with_path_traversal` — call with `context: "../../etc"`, assert validation error
      2. `rejects_tag_with_shell_metacharacters` — call with `tag: "foo;rm -rf /"`, assert validation error
      3. `rejects_invalid_build_arg_keys` — call with a build arg key containing `;`, assert validation error
      4. `validates_clean_inputs` — call with valid inputs, assert the command is attempted (will fail if Docker is not installed, but the JSON output structure should still be correct with `success: false` and a meaningful error)

      Files: `tools/docker-build/src/docker_build.rs`
      Blocked by: "Create `tools/docker-build/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write integration tests"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/docker-build/src/main.rs`. Mirror `tools/cargo-build/src/main.rs`: declare `mod docker_build;`, use `DockerBuildTool`, call `mcp_tool_harness::serve_stdio_tool(DockerBuildTool::new(), "docker-build").await`. Under 10 lines.
      Files: `tools/docker-build/src/main.rs`
      Blocked by: "Implement `DockerBuildTool` struct and handler"
      Blocking: "Write integration tests"

## Group 3 — Integration tests and documentation

_Depends on: Group 2_

- [x] **Write integration tests in `tests/docker_build_server_test.rs`** `[M]`
      Create `tools/docker-build/tests/docker_build_server_test.rs`. Use `spawn_mcp_client!(env!("CARGO_BIN_EXE_docker-build"))` pattern from `mcp-test-utils`.

      Tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):
      1. `tools_list_returns_docker_build_tool` — use `mcp_test_utils::assert_single_tool` to verify tool name is `"docker_build"`, description contains `"Docker image"`, and parameters include `["context", "tag", "build_args", "dockerfile"]`.
      2. `tools_call_rejects_path_traversal` — call with `{"context": "../../etc", "tag": "test:latest"}`, parse response, assert `success` is `false` and the output indicates a validation error.
      3. `tools_call_rejects_invalid_tag` — call with `{"context": ".", "tag": "test;evil"}`, assert `success` is `false`.
      4. `tools_call_returns_error_when_docker_unavailable` — call with valid inputs `{"context": ".", "tag": "test:latest"}`; if Docker is not installed in CI, assert the response is valid JSON with `success: false` and a meaningful error message (graceful degradation).

      Files: `tools/docker-build/tests/docker_build_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: None

- [x] **Write `README.md`** `[S]`
      Create `tools/docker-build/README.md` following the pattern from `tools/echo-tool/README.md`. Include: description, build/run/test commands, input parameters, output format, security considerations (path validation, no shell execution), and Docker-in-Docker caveat.
      Files: `tools/docker-build/README.md`
      Non-blocking

## Group 4 — Verification

_Depends on: Groups 1-3_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p docker-build`, `cargo test -p docker-build`, `cargo clippy -p docker-build`, and `cargo check` (workspace-wide). Verify all acceptance criteria: build succeeds, tests pass, tool is named `docker_build` in MCP tools/list, returns structured JSON on both success and failure, input validation prevents command injection.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

## Implementation Notes

1. **No new dependencies**: All dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `mcp-tool-harness`) are already used by `tools/cargo-build`. The `std::collections::HashMap` needed for `build_args` is in the standard library.

2. **Security is the primary concern**: Unlike `cargo_build` which only validates a package name, `docker_build` accepts filesystem paths and arbitrary key-value pairs. Path traversal prevention (canonicalize and check prefix) and input sanitization are essential. The command is invoked via `std::process::Command` (not through a shell), which prevents shell injection, but argument injection through `--build-arg` values must still be guarded against.

3. **Image ID extraction**: Docker build output format varies between Docker versions (legacy builder vs BuildKit). The implementation should try to parse both `Successfully built <short-id>` (legacy) and `writing image sha256:<full-id>` (BuildKit). If neither pattern matches, return `"unknown"` for `image_id` rather than failing.

4. **Docker availability in CI**: Docker may not be available in all test environments. Integration tests that actually invoke Docker should handle the "docker not found" case gracefully. Unit tests should focus on input validation which does not require Docker.

5. **`build_args` as `HashMap<String, String>`**: The issue specifies `build_args` as an "object". Using `Option<HashMap<String, String>>` with serde will correctly deserialize a JSON object like `{"FOO": "bar"}`. The `schemars::JsonSchema` derive handles this type natively.

6. **Reference files**:
   - `tools/cargo-build/src/cargo_build.rs` — Closest reference pattern for struct, macros, validation, unit tests
   - `tools/cargo-build/Cargo.toml` — Template for dependencies
   - `crates/mcp-test-utils/src/lib.rs` — Test utilities (`spawn_mcp_client!`, `assert_single_tool`)
