# Spec: Write `src/main.rs`

> From: .claude/tasks/issue-48.md

## Objective
Create the binary entrypoint for the `docker-build` MCP tool. This file wires together the `DockerBuildTool` struct and the shared `mcp_tool_harness` runtime so the tool can be launched as a standalone stdio-based MCP server.

## Current State
`tools/cargo-build/src/main.rs` provides the established pattern: declare the module, import the tool struct, and call `mcp_tool_harness::serve_stdio_tool` inside a single-threaded Tokio main function. The `docker-build` crate directory does not yet contain a `src/main.rs`.

## Requirements
- File must be located at `tools/docker-build/src/main.rs`.
- Declare `mod docker_build;` to pull in the sibling module.
- Import `DockerBuildTool` from that module.
- Use `#[tokio::main(flavor = "current_thread")]` on an async `main` function.
- Call `mcp_tool_harness::serve_stdio_tool(DockerBuildTool::new(), "docker-build").await` and return its result.
- Return type must be `Result<(), Box<dyn std::error::Error>>`.
- File must be under 10 lines total (matching the cargo-build pattern exactly).
- No additional logic, imports, or feature flags beyond what is listed above.

## Implementation Details
- **Create** `tools/docker-build/src/main.rs`
  - Line 1: `mod docker_build;`
  - Line 2: `use docker_build::DockerBuildTool;`
  - Lines 4-7: Tokio main function calling `serve_stdio_tool`

## Dependencies
- Blocked by: "Implement `DockerBuildTool` struct and handler" (the `docker_build` module must exist and export `DockerBuildTool` with a `new()` constructor)
- Blocking: "Write integration tests" (tests need a compilable binary to run against)

## Risks & Edge Cases
- If `DockerBuildTool::new()` signature changes (e.g., requires arguments), this file must be updated to match.
- The module file must be named `docker_build.rs` (not `docker-build.rs`) due to Rust module naming rules; ensure the sibling module uses underscores.

## Verification
- `cargo check -p docker-build` compiles without errors (requires the `DockerBuildTool` struct to exist).
- `cargo build -p docker-build` produces a binary at `target/debug/docker-build`.
- The file is 7 lines (or fewer than 10), matching the cargo-build pattern.
