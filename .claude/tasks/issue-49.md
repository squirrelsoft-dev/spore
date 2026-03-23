# Task Breakdown: Implement docker_push MCP tool

> Implement `docker_push` as a standalone Rust MCP server binary that pushes a tagged Docker image to a container registry, following the echo-tool and cargo-build reference patterns.

## Group 1 — Scaffold the crate

_Tasks in this group can be done in parallel._

- [x] **Create `tools/docker-push/Cargo.toml`** `[S]`
      Copy and adapt `tools/cargo-build/Cargo.toml`. Change `name = "docker-push"`. Keep the same dependencies: `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }`, `rmcp` with `transport-io`, `server`, `macros` features; `tokio` with `macros`, `rt`, `io-std`; `serde` with `derive`; `serde_json`. Add the same `[dev-dependencies]` block: `mcp-test-utils = { path = "../../crates/mcp-test-utils" }`, `tokio` with `macros`, `rt`, `rt-multi-thread`; `rmcp` with `client` and `transport-child-process`; `serde_json`.
      Files: `tools/docker-push/Cargo.toml`
      Blocking: "Implement `DockerPushTool` struct and handler", "Write `main.rs`", "Write integration tests"

- [x] **Add `"tools/docker-push"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/docker-push"` to the `members` list in the root `Cargo.toml`, after the existing `"tools/cargo-build"` entry.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 2 — Core implementation

_Depends on: Group 1_

- [x] **Implement `DockerPushTool` struct and handler in `src/docker_push.rs`** `[M]`
      Create `tools/docker-push/src/docker_push.rs`. Follow the `cargo-build` pattern exactly since both tools invoke an external command and return structured JSON.

      Define `DockerPushRequest` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` containing two fields:
      - `image: String` with doc comment `/// Full image reference (e.g., ghcr.io/spore/spore-agent:0.1)`
      - `registry_url: Option<String>` with doc comment `/// Override registry URL; falls back to REGISTRY_URL env var`

      Define `DockerPushTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`.

      **Input validation**: Before invoking `docker push`, validate the `image` string. It must not be empty, must not contain shell metacharacters (`;`, `|`, `&`, `$`, `` ` ``, `(`, `)`, `{`, `}`, `<`, `>`, `!`, `\n`), and must contain only valid image reference characters (alphanumeric, `.`, `-`, `_`, `/`, `:`). Use a simple character check (no regex dependency). If validation fails, return `{"success": false, "image": "<input>", "digest": "", "push_log": "Invalid image reference: <reason>"}`.

      **Registry URL resolution**: If `registry_url` is `Some`, use it. Otherwise, read `REGISTRY_URL` from the environment via `std::env::var("REGISTRY_URL")`. If the registry URL is available and the `image` string does not already start with it, prepend the registry URL with a `/` separator. If neither is provided and the image has no registry prefix, proceed with the image as-is (Docker will use the default registry).

      **Docker push invocation**: Use `std::process::Command::new("docker")` with args `["push", &final_image_ref]`. Capture output with `.output()`.

      **Digest extraction**: After a successful push, parse stdout+stderr for the digest. `docker push` outputs a line like `<tag>: digest: sha256:<hex> size: <n>`. Search the combined output (stdout then stderr) line by line for a substring matching `digest: sha256:`. Extract the full `sha256:<hex>` value. If no digest is found in the output, set digest to an empty string.

      **Return format**: Return `serde_json::json!` containing:
      - `"success"`: `output.status.success()`
      - `"image"`: the final image reference used
      - `"digest"`: extracted digest string or `""`
      - `"push_log"`: combined stdout + stderr (both via `String::from_utf8_lossy`)

      On `Command` spawn failure, return `{"success": false, "image": "<input>", "digest": "", "push_log": "Failed to execute docker: <error>"}`.

      Implement `ServerHandler` with `#[tool_handler]` returning tools-enabled capabilities.

      Add `#[cfg(test)] mod tests` with unit tests:
      (1) `rejects_empty_image` — call with `""`, assert `success: false` and push_log contains "Invalid image reference"
      (2) `rejects_image_with_shell_metachar` — call with `"foo;bar"`, assert `success: false`
      (3) `rejects_image_with_pipe` — call with `"foo|bar"`, assert `success: false`
      (4) `accepts_valid_image_reference` — call with `"ghcr.io/spore/spore-agent:0.1"`, assert the result is valid JSON containing all four expected fields (`success`, `image`, `digest`, `push_log`). Note: this test will fail if Docker is not installed, but it validates the code path up to Command invocation.
      (5) `registry_url_is_prepended` — test the registry URL prepending logic. Create a helper function that resolves the final image reference, call it with `image = "spore-agent:0.1"` and `registry_url = Some("ghcr.io/spore")`, assert the result is `"ghcr.io/spore/spore-agent:0.1"`. Also test that if the image already starts with the registry URL, it is not doubled.
      (6) `digest_extraction_from_output` — test the digest parsing logic in isolation. Create a helper function that extracts digest from a string, call it with sample docker push output containing `"latest: digest: sha256:abc123def456 size: 1234"`, assert it returns `"sha256:abc123def456"`. Also test with output lacking a digest line, assert empty string.

      Files: `tools/docker-push/src/docker_push.rs`
      Blocked by: "Create `tools/docker-push/Cargo.toml`"
      Blocking: "Write `main.rs`", "Write integration tests"

- [x] **Write `src/main.rs`** `[S]`
      Create `tools/docker-push/src/main.rs`. Mirror `tools/cargo-build/src/main.rs` exactly: declare `mod docker_push;`, use `DockerPushTool`, call `mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await`. The file should be under 10 lines.
      Files: `tools/docker-push/src/main.rs`
      Blocked by: "Implement `DockerPushTool` struct and handler"
      Blocking: "Write integration tests"

## Group 3 — Integration tests and documentation

_Depends on: Group 2_

_Tasks in this group can be done in parallel._

- [x] **Write integration tests in `tests/docker_push_server_test.rs`** `[M]`
      Create `tools/docker-push/tests/docker_push_server_test.rs`. Mirror the pattern from `tools/cargo-build/tests/cargo_build_server_test.rs`. Use `env!("CARGO_BIN_EXE_docker-push")` to spawn the binary via `mcp_test_utils::spawn_mcp_client!`. Write these tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`):

      (1) `tools_list_returns_docker_push_tool` — use `mcp_test_utils::assert_single_tool` to verify the tool name is `"docker_push"`, description contains `"Push"` (case-sensitive from the tool description), and parameters include `["image", "registry_url"]`.

      (2) `tools_call_with_invalid_image_returns_error` — call the tool with `{"image": "foo;bar"}`, parse the response text as JSON, assert `success` is `false` and `push_log` contains "Invalid image reference".

      (3) `tools_call_with_valid_image_returns_structured_json` — call the tool with `{"image": "nonexistent-image:latest"}`. Since Docker daemon may or may not be available in CI, only assert that the response is valid JSON containing all four expected fields (`success`, `image`, `digest`, `push_log`). Do not assert `success: true` since the image does not exist.

      (4) `tools_call_with_empty_image_returns_error` — call with `{"image": ""}`, assert `success: false`.

      Each test must end with `client.cancel().await.expect("failed to cancel client");`.

      Files: `tools/docker-push/tests/docker_push_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: "Run verification suite"

- [x] **Write README** `[S]`
      Create `tools/docker-push/README.md`. Model after `tools/echo-tool/README.md`. Include: build/run/test commands (`cargo build -p docker-push`, `cargo run -p docker-push`, `cargo test -p docker-push`), description of the `image` and `registry_url` input parameters, description of the JSON output format (`success`, `image`, `digest`, `push_log`), note about `REGISTRY_URL` environment variable fallback, MCP Inspector test command, and a note that Docker must be available in the environment for the push to succeed.
      Files: `tools/docker-push/README.md`
      Blocked by: "Implement `DockerPushTool` struct and handler"
      Blocking: None

## Group 4 — Verification

_Depends on: Groups 1–3_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p docker-push`, then `cargo test -p docker-push`, then `cargo clippy -p docker-push`, then `cargo check` (workspace-wide) to confirm no regressions. All acceptance criteria from the issue must pass: build succeeds, tests pass, returns structured JSON with `success`/`image`/`digest`/`push_log`, returns structured errors on auth/network/validation failure, tool is named `docker_push` in MCP `tools/list`.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

---

## Implementation Notes

1. **No new dependencies**: All dependencies (`rmcp`, `tokio`, `serde`, `serde_json`, `mcp-tool-harness`) are already used by `tools/cargo-build/Cargo.toml`. No additional crates are needed.

2. **Security — input validation**: The `image` input is validated to reject shell metacharacters before being passed to `std::process::Command`. The command is invoked directly via `Command::new("docker")` (not via a shell), providing additional safety. This mirrors the `validate_package_name` pattern in `tools/cargo-build/src/cargo_build.rs`.

3. **Security — no secrets in output**: The tool does not log or return authentication credentials. Docker auth is assumed to be pre-configured via `docker login` or credential helpers.

4. **Registry URL resolution**: The `registry_url` optional parameter overrides the `REGISTRY_URL` environment variable. The tool prepends the registry URL to the image only if the image does not already include it.

5. **Digest extraction**: `docker push` outputs digest information in stderr. The format is `<tag>: digest: sha256:<hex> size: <n>`. The tool searches both stdout and stderr for this pattern. This logic is extracted into a testable helper function.

6. **Binary vs tool name**: Package name `docker-push` produces binary `docker-push` and env macro `CARGO_BIN_EXE_docker-push`. The MCP tool name is `docker_push` (snake_case method name from `#[tool_router]`).

7. **Docker availability in tests**: Integration tests that invoke the tool with a real image reference should not assert `success: true` because Docker may not be available in all CI environments. Unit tests for validation, registry URL resolution, and digest extraction do not require Docker.

8. **Reference pattern**: `tools/cargo-build/src/cargo_build.rs` is the primary reference since it also invokes an external command via `std::process::Command` and returns structured JSON.
