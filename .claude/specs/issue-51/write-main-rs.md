# Spec: Write `src/main.rs`
> From: .claude/tasks/issue-51.md

## Objective
Create the entrypoint file `tools/list-agents/src/main.rs` that wires up the `ListAgentsTool` struct to the MCP stdio harness, following the identical pattern used by every other tool binary in the workspace.

## Current State
The file `tools/list-agents/src/main.rs` does not exist yet. The sibling tools (`echo-tool`, `register-agent`, etc.) each have a ~7-line `main.rs` that declares the tool module, imports the tool struct, and calls `mcp_tool_harness::serve_stdio_tool`.

## Requirements
1. Declare `mod list_agents;` to pull in the sibling module file.
2. Import `ListAgentsTool` from that module.
3. Define `#[tokio::main(flavor = "current_thread")]` async main returning `Result<(), Box<dyn std::error::Error>>`.
4. Call `mcp_tool_harness::serve_stdio_tool(ListAgentsTool::new(), "list-agents").await` and return its result.
5. File must be under 10 lines total.

## Implementation Details
The file should mirror `tools/echo-tool/src/main.rs` exactly in structure:

```
mod list_agents;
use list_agents::ListAgentsTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(ListAgentsTool::new(), "list-agents").await
}
```

Key conventions carried over from existing tools:
- `flavor = "current_thread"` (single-threaded Tokio runtime, consistent across all tool binaries).
- The tool name string passed to `serve_stdio_tool` matches the crate/binary name (`"list-agents"`).
- No additional imports, logging setup, or configuration — the harness handles all of that.

## Dependencies
- **Blocked by**: "Implement `ListAgentsTool` struct and handler" — the `list_agents` module (and its `ListAgentsTool` struct with `new()`) must exist before this file compiles.
- **Blocking**: "Write integration tests" — tests will invoke this binary.
- **Crate dependencies**: `tokio`, `mcp_tool_harness` (already declared in the workspace `Cargo.toml` for other tools).

## Risks & Edge Cases
- **Module naming**: The module is `list_agents` (underscores), matching the filename `list_agents.rs`. The binary/tool name is `list-agents` (hyphens). This follows Rust convention and is consistent with `register-agent` / `register_agent`.
- **No standalone risk**: This file is pure boilerplate with no logic of its own; all risk lives in the `ListAgentsTool` implementation and the harness.

## Verification
1. `cargo check -p list-agents` compiles without errors (requires the `list_agents` module to exist).
2. `cargo build -p list-agents` produces a binary.
3. The binary, when run, starts an MCP stdio server that advertises the `list-agents` tool (verified by integration tests in a later task).
