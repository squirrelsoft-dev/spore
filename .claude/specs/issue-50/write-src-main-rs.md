# Spec: Write `src/main.rs`

> From: .claude/tasks/issue-50.md

## Objective
Create `tools/register-agent/src/main.rs` as the binary entrypoint for the `register-agent` MCP tool. It must mirror the pattern established by `tools/docker-push/src/main.rs` exactly, adapted for the `RegisterAgentTool` type.

## Current State
The file `tools/register-agent/src/main.rs` does not yet exist. The reference implementation at `tools/docker-push/src/main.rs` is 7 lines and follows a fixed pattern: declare the module, import the tool struct, define an async `main` that calls `mcp_tool_harness::serve_stdio_tool`.

## Requirements
1. Declare `mod register_agent;` to pull in the sibling module.
2. Import `register_agent::RegisterAgentTool`.
3. Define `#[tokio::main(flavor = "current_thread")]` async `main` returning `Result<(), Box<dyn std::error::Error>>`.
4. Body calls `mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await`.
5. File must be under 10 lines total.

## Implementation Details
The file should be an exact structural mirror of `tools/docker-push/src/main.rs`:

```rust
mod register_agent;
use register_agent::RegisterAgentTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(RegisterAgentTool::new(), "register-agent").await
}
```

Key decisions:
- Use `flavor = "current_thread"` on the tokio runtime, matching the established convention.
- The tool name string passed to `serve_stdio_tool` is `"register-agent"` (kebab-case), matching the crate/directory name.
- No additional imports, logging setup, or error handling beyond what the reference provides.

## Dependencies
- **Blocked by**: "Implement RegisterAgentTool struct and handler" -- the `register_agent` module containing `RegisterAgentTool` must exist before this file can compile.
- **Crate dependencies**: `tokio`, `mcp_tool_harness` (both already used by other tools in the workspace).

## Risks & Edge Cases
- If `RegisterAgentTool` is named differently in the sibling module, the import will fail. The struct name must match exactly.
- If the `register_agent` module file is named something other than `register_agent.rs` (e.g., `mod.rs` in a subdirectory), the `mod register_agent;` declaration still works but the file layout must be consistent.

## Verification
1. `cargo check -p register-agent` compiles without errors (requires the `register_agent` module to exist).
2. File is under 10 lines: `wc -l tools/register-agent/src/main.rs` outputs a number less than or equal to 10.
3. Structural diff against `tools/docker-push/src/main.rs` shows only the expected name substitutions (`docker_push` -> `register_agent`, `DockerPushTool` -> `RegisterAgentTool`, `"docker-push"` -> `"register-agent"`).
