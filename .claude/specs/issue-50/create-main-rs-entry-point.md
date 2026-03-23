# Spec: Create main.rs entry point

> From: .claude/tasks/issue-50.md

## Objective
Create the `main.rs` entry point for the `register-agent` MCP tool binary, following the established pattern used by other tools (e.g., `docker-push`). This wires the `RegisterAgentTool` implementation into the MCP stdio harness so the tool can be invoked as a standalone process.

## Current State
All other MCP tool binaries in `tools/*/src/main.rs` follow an identical pattern:
1. Declare a module for the tool implementation.
2. Import the tool struct.
3. Define an async `main` that calls `mcp_tool_harness::serve_stdio_tool`.

Reference (`tools/docker-push/src/main.rs`):
```rust
mod docker_push;
use docker_push::DockerPushTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(DockerPushTool::new(), "docker-push").await
}
```

The file `tools/register-agent/src/main.rs` does not yet exist.

## Requirements
- Create `tools/register-agent/src/main.rs` with the standard entry point pattern.
- The module declaration must be `mod register_agent;`.
- The use statement must import `register_agent::RegisterAgentTool`.
- The `main` function must use `#[tokio::main(flavor = "current_thread")]`.
- The `main` function must return `Result<(), Box<dyn std::error::Error>>`.
- The body must call `mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await`.
- The file must be exactly 7 lines (matching the docker-push convention), with no extraneous code.

## Implementation Details
- **Create** `tools/register-agent/src/main.rs`:
  - Line 1: `mod register_agent;`
  - Line 2: `use register_agent::RegisterAgentTool;`
  - Line 3: blank
  - Line 4: `#[tokio::main(flavor = "current_thread")]`
  - Line 5: `async fn main() -> Result<(), Box<dyn std::error::Error>> {`
  - Line 6: `    mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await`
  - Line 7: `}`

## Dependencies
- Blocked by: "Implement register_agent tool logic" (the `RegisterAgentTool` struct and its `new()` constructor must exist in `tools/register-agent/src/register_agent.rs` before this file compiles)
- Blocking: "Write integration tests" (tests need a compilable binary to run against)

## Risks & Edge Cases
- If the `register_agent` module file does not exist or does not export `RegisterAgentTool`, compilation will fail. This is expected since this task is blocked by the tool logic implementation.
- The tool name string `"register-agent"` must match what the MCP harness and any upstream configuration expect. Verify consistency with `Cargo.toml` binary name.

## Verification
- `cargo check -p register-agent` succeeds (once the blocking task is complete).
- `cargo build -p register-agent` produces a binary.
- The file content matches the docker-push pattern exactly (substituting module and type names).
- `cargo clippy -p register-agent` reports no warnings.
