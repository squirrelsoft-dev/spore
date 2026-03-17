# Spec: Write tool README

> From: .claude/tasks/issue-10.md

## Objective

Create a README for the `tools/echo-tool/` crate that serves as both user documentation for the echo tool and a template guide for building new MCP tools in the Spore project. This is a documentation-only task with no code changes.

## Current State

- The `tools/echo-tool/` directory does not yet exist; it will be created by the "Implement echo tool server" task (Group 2).
- The `tool-registry` crate is a stub (`pub struct ToolRegistry;`), so registry integration docs must be deferred until issue #8 is complete.
- The project uses `rmcp` for MCP tool servers with stdio transport, `tokio` for async, and Rust edition 2024.
- No README or documentation exists yet under `tools/`.

## Requirements

- The README must contain all five sections specified in the task:
  1. **What the tool does** -- describe the echo tool as a reference MCP tool server that returns input messages unchanged, serving as the template for all future tool implementations.
  2. **How to build** -- `cargo build -p echo-tool`.
  3. **How to run** -- `cargo run -p echo-tool` with a note that it uses stdio transport (stdin/stdout for MCP messages, stderr for logging).
  4. **How to test with MCP inspector** -- `npx @modelcontextprotocol/inspector cargo run -p echo-tool`.
  5. **How to create a new tool using this as a template** -- step-by-step guide covering: creating the directory under `tools/`, copying and modifying `Cargo.toml`, defining a tool struct with `#[tool_router]`, implementing `ServerHandler`, adding the crate to the workspace `members` array, and running tests.
- The README must include a note that tool-registry registration will be documented once issue #8 is complete.
- The README must not contain incorrect or speculative information about APIs that have not been implemented yet.
- The document must use standard markdown formatting consistent with the project's `README.md` style (heading hierarchy, fenced code blocks for commands).

## Implementation Details

### File to create

- **`tools/echo-tool/README.md`**

### Document structure

```
# echo-tool

<One-paragraph description of what it does and its role as a reference implementation>

## Build

<cargo build command>

## Run

<cargo run command with explanation of stdio transport>

## Test with MCP Inspector

<npx inspector command with brief explanation of what the inspector does>

## Creating a New Tool

<Numbered steps to create a new tool using echo-tool as a template>

## Tool Registry

<Note that registration docs are pending issue #8>
```

### Key content details

- The "Run" section should mention that the tool communicates over stdin/stdout using the MCP protocol, and that all logging goes to stderr to avoid corrupting the transport channel.
- The "Test with MCP Inspector" section should briefly explain that the MCP Inspector provides a web UI for sending tool calls and inspecting responses interactively.
- The "Creating a New Tool" section should cover these steps:
  1. Create a new directory under `tools/` (e.g., `tools/my-tool/`).
  2. Copy `tools/echo-tool/Cargo.toml` and update the package name.
  3. Define a tool struct and apply `#[tool_router]` with tool methods annotated with `#[tool(description = "...")]`.
  4. Implement `ServerHandler` with `get_info()` returning server capabilities with tools enabled.
  5. Write `main()` to initialize logging (to stderr), create the tool, and serve over stdio.
  6. Add the new crate path to the workspace `members` array in the root `Cargo.toml`.
  7. Run `cargo build -p <tool-name>`, `cargo test -p <tool-name>`, and `cargo clippy -p <tool-name>`.
- No functions, types, or interfaces are added by this task.

## Dependencies

- Blocked by: "Implement echo tool server" -- the README documents the implemented tool's behavior, build commands, and source structure; writing it before the tool exists risks documenting APIs that change during implementation.
- Blocking: None

## Risks & Edge Cases

1. **API drift:** If the echo tool implementation changes after the README is written (e.g., struct names, method signatures), the README will become inaccurate. Mitigate by keeping code references generic (e.g., "define a tool struct") rather than pinning to exact type names, except where specificity aids clarity.
2. **MCP Inspector availability:** The `npx @modelcontextprotocol/inspector` command depends on the inspector package being published to npm. If it is renamed or removed, the command will break. This is an external dependency outside project control.
3. **Premature registry docs:** The task explicitly states that tool-registry registration should be noted as pending. Do not document registry APIs that do not yet exist.

## Verification

1. The file `tools/echo-tool/README.md` exists and is valid markdown.
2. All five required sections are present: what, build, run, MCP inspector, and new-tool template guide.
3. The build command is exactly `cargo build -p echo-tool`.
4. The run command is exactly `cargo run -p echo-tool`.
5. The inspector command is exactly `npx @modelcontextprotocol/inspector cargo run -p echo-tool`.
6. A note about tool-registry registration being pending (issue #8) is included.
7. No speculative or incorrect API documentation is present.
8. `cargo test` and `cargo clippy` pass (no code changes, but confirm nothing is broken).
