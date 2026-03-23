# Spec: Write `README.md`

> From: .claude/tasks/issue-48.md

## Objective
Create a README for the `docker-build` tool crate that documents its purpose, usage, parameters, output format, and security model. This gives developers and users a self-contained reference for the tool, following the established pattern used by `echo-tool` and `validate-skill`.

## Current State
- `tools/echo-tool/README.md` and `tools/validate-skill/README.md` exist as reference patterns.
- `tools/cargo-build/` has no README yet, so `validate-skill` is the best feature-tool reference (it includes Parameters and Output tables).
- The task breakdown in `.claude/tasks/issue-48.md` defines the tool's four input parameters (`context`, `tag`, `build_args`, `dockerfile`), output JSON structure (`success`, `image_id`, `tag`, `build_log`), and security validation rules.

## Requirements
- Follow the section structure from existing tool READMEs: description paragraph, Build, Run, Test with MCP Inspector, Test, Parameters table, Output table with success/failure examples, and a Notes/Security section.
- **Description**: Explain that `docker-build` is an MCP tool server that builds Docker images from a Dockerfile and context directory, returning structured JSON results.
- **Build/Run/Test commands**: Use `-p docker-build` consistently (`cargo build -p docker-build`, `cargo run -p docker-build`, `cargo test -p docker-build`, and `npx @modelcontextprotocol/inspector cargo run -p docker-build`).
- **Parameters table**: Document all four parameters with types and descriptions:
  - `context` (string, required) — build context directory path
  - `tag` (string, required) — image tag to assign
  - `build_args` (object, optional) — key-value map of Docker build arguments
  - `dockerfile` (string, optional) — path to Dockerfile (defaults to `context/Dockerfile`)
- **Output table**: Document the four output fields: `success` (boolean), `image_id` (string), `tag` (string), `build_log` (string). Include a success JSON example and a failure JSON example.
- **Security Considerations section**: Cover these points:
  - Path validation: `context` and `dockerfile` are canonicalized and checked to be within the project root; `..` segments are rejected before canonicalization.
  - Tag validation: only `[a-zA-Z0-9._:/-]` characters are allowed.
  - Build-arg sanitization: keys and values are rejected if they contain shell metacharacters or newlines.
  - No shell execution: commands run via `std::process::Command`, not through a shell, preventing shell injection.
- **Docker-in-Docker caveat**: Note that when running inside a container (e.g., CI or devcontainers), the Docker socket must be mounted or Docker-in-Docker must be configured for the tool to function. If Docker is unavailable, the tool returns a graceful JSON error with `success: false`.

## Implementation Details
- File to create: `tools/docker-build/README.md`
- Use markdown tables for Parameters and Output (matching `tools/validate-skill/README.md` style).
- Use fenced code blocks with `json` language tag for output examples and `sh` for commands.
- Keep the document concise; no "Creating a New Tool" section (that belongs only in `echo-tool`).
- Sections in order:
  1. Title and description paragraph
  2. Build (`cargo build -p docker-build`)
  3. Run (`cargo run -p docker-build`) with stdio transport note
  4. Test with MCP Inspector (`npx @modelcontextprotocol/inspector cargo run -p docker-build`)
  5. Test (`cargo test -p docker-build`)
  6. Parameters (markdown table: Name, Type, Required, Description)
  7. Output (markdown table: Field, Type, Description; then success and failure JSON examples)
  8. Security Considerations (bullet list covering path validation, tag validation, build-arg sanitization, no shell execution)
  9. Notes (Docker-in-Docker caveat, `image_id` may be `"unknown"` when ID cannot be parsed)

## Dependencies
- Blocked by: None (documentation can be written before or after implementation)
- Blocking: None (non-blocking task per the task breakdown)

## Risks & Edge Cases
- The output JSON structure may evolve during implementation (e.g., additional fields). The README should be updated if the final implementation diverges from the spec.
- The image ID extraction logic handles both legacy Docker builder and BuildKit output formats; the README should mention that `image_id` may be `"unknown"` if neither format is detected, without overexplaining the parsing internals.

## Verification
- The README file exists at `tools/docker-build/README.md`.
- It contains all required sections: description, Build, Run, Test with MCP Inspector, Test, Parameters, Output (with success and failure examples), Security Considerations, and a Docker-in-Docker note.
- Parameter names and types match the `DockerBuildRequest` struct defined in the task breakdown (`context`, `tag`, `build_args`, `dockerfile`).
- Output fields match the specified JSON response shape (`success`, `image_id`, `tag`, `build_log`).
- Commands use `-p docker-build` consistently.
- No broken markdown formatting (tables render correctly, code blocks are properly fenced).
