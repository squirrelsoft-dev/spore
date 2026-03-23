# Spec: Write README

> From: .claude/tasks/issue-49.md

## Objective

Create a README for the `tools/docker-push/` crate that documents the tool's purpose, usage, input parameters, output format, environment variable fallback behavior, and runtime requirements. Model the structure and tone after `tools/echo-tool/README.md`.

## Current State

- The `tools/docker-push/` directory does not yet exist; it will be created by preceding tasks in Groups 1 and 2 (scaffold and core implementation).
- The `DockerPushTool` struct and handler will be implemented in `tools/docker-push/src/docker_push.rs` as part of the "Implement `DockerPushTool` struct and handler" task, which this task is blocked by.
- The echo-tool README (`tools/echo-tool/README.md`) serves as the reference template. It uses a flat heading structure with sections for description, build, run, MCP Inspector testing, and additional guidance.
- The tool accepts two input parameters (`image`, `registry_url`) and returns structured JSON with four fields (`success`, `image`, `digest`, `push_log`).

## Requirements

- The README must contain the following sections:
  1. **Tool description** -- describe `docker-push` as an MCP tool server that pushes a tagged Docker image to a container registry, returning structured JSON with push status, digest, and logs.
  2. **Build** -- `cargo build -p docker-push`.
  3. **Run** -- `cargo run -p docker-push` with a note about stdio transport.
  4. **Test** -- `cargo test -p docker-push`.
  5. **Input Parameters** -- document the `image` parameter (required, full image reference such as `ghcr.io/spore/spore-agent:0.1`) and the `registry_url` parameter (optional, overrides the `REGISTRY_URL` environment variable).
  6. **Output Format** -- describe the JSON response containing `success` (boolean), `image` (the final image reference used), `digest` (sha256 digest extracted from docker push output, or empty string), and `push_log` (combined stdout/stderr from the docker push command).
  7. **Environment Variables** -- document the `REGISTRY_URL` fallback: if `registry_url` is not provided as a parameter, the tool reads `REGISTRY_URL` from the environment. If available and the image does not already include the registry prefix, it is prepended automatically.
  8. **Test with MCP Inspector** -- `npx @modelcontextprotocol/inspector cargo run -p docker-push`.
  9. **Prerequisites** -- note that Docker must be installed and available in the environment for the push to succeed, and that authentication should be configured via `docker login` or credential helpers before invoking the tool.
- The README must use standard markdown formatting consistent with the echo-tool README (heading hierarchy, fenced code blocks for commands).
- The README must not contain speculative information about features not implemented in the tool.

## Implementation Details

### File to create

- **`tools/docker-push/README.md`**

### Document structure

```
# docker-push

<One-paragraph description: MCP tool server that pushes tagged Docker images to a container registry>

## Build

<cargo build command>

## Run

<cargo run command with stdio transport note>

## Test

<cargo test command>

## Input Parameters

<Table or list describing `image` and `registry_url`>

## Output Format

<Description of JSON response with `success`, `image`, `digest`, `push_log` fields>

## Environment Variables

<REGISTRY_URL fallback behavior>

## Test with MCP Inspector

<npx inspector command with brief explanation>

## Prerequisites

<Docker availability requirement and authentication note>
```

### Key content details

- The "Run" section should note that the tool uses stdio transport (stdin/stdout for MCP messages, stderr for logging), matching the echo-tool pattern.
- The "Input Parameters" section should clearly indicate that `image` is required and `registry_url` is optional.
- The "Output Format" section should show an example JSON response structure to make the format concrete.
- The "Environment Variables" section should explain the resolution order: `registry_url` parameter takes precedence over `REGISTRY_URL` env var. If neither is provided and the image has no registry prefix, Docker's default registry is used.
- The "Prerequisites" section should note two requirements: (1) Docker must be installed and the `docker` command must be available on PATH, and (2) authentication to the target registry must be pre-configured (the tool does not handle login).
- No functions, types, or interfaces are added by this task.

## Dependencies

- Blocked by: "Implement `DockerPushTool` struct and handler" -- the README documents the implemented tool's input parameters, output format, and behavior. Writing it before the tool exists risks documenting behavior that changes during implementation.
- Blocking: None

## Risks & Edge Cases

1. **API drift:** If the `DockerPushTool` implementation changes after the README is written (e.g., field names, parameter names, validation behavior), the README will become inaccurate. Mitigate by reviewing the final implementation before writing the README and keeping descriptions aligned with the actual struct definitions.
2. **MCP Inspector availability:** The `npx @modelcontextprotocol/inspector` command depends on the inspector package being published to npm. This is an external dependency outside project control.
3. **Docker-specific behavior:** The digest extraction format (`sha256:<hex>`) and push output format are Docker-specific. If the tool is later extended to support other container runtimes (e.g., Podman), the README will need updates.
4. **Environment variable naming:** The `REGISTRY_URL` environment variable name is established in the task specification. If it changes during implementation, the README must be updated accordingly.

## Verification

1. The file `tools/docker-push/README.md` exists and is valid markdown.
2. All required sections are present: description, build, run, test, input parameters, output format, environment variables, MCP Inspector, and prerequisites.
3. The build command is exactly `cargo build -p docker-push`.
4. The run command is exactly `cargo run -p docker-push`.
5. The test command is exactly `cargo test -p docker-push`.
6. The inspector command is exactly `npx @modelcontextprotocol/inspector cargo run -p docker-push`.
7. Both input parameters (`image`, `registry_url`) are documented with their types and required/optional status.
8. All four output JSON fields (`success`, `image`, `digest`, `push_log`) are documented.
9. The `REGISTRY_URL` environment variable fallback behavior is documented.
10. A note about Docker being required in the environment is present.
11. A note about pre-configured authentication is present.
12. `cargo test` and `cargo clippy` pass (no code changes, but confirm nothing is broken).
