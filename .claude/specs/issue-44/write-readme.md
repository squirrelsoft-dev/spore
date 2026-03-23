# Spec: Write README

> From: .claude/tasks/issue-44.md

## Objective

Create a README for the `tools/read-file/` crate that documents the tool's purpose, how to build, run, and test it, and the stdio transport model. Follow the established pattern from `tools/echo-tool/README.md`. This is a documentation-only task with no code changes.

## Current State

- The `tools/read-file/` crate will be implemented by earlier tasks in this issue (scaffold, core implementation, unit tests).
- The `tools/echo-tool/README.md` exists and serves as the reference template for all tool READMEs.
- The tool uses `rmcp` with stdio transport, `tokio` for async, and `std::fs` for file operations.
- The tool reads file contents from disk and returns them as a string, with descriptive error handling and a 10 MB size guard.

## Requirements

- The README must contain the following sections:
  1. **Purpose** -- describe the read-file tool as an MCP tool server that reads file contents from disk and returns them as text, with validation and descriptive error handling.
  2. **Build** -- `cargo build -p read-file`.
  3. **Run** -- `cargo run -p read-file` with a note about stdio transport.
  4. **Test** -- `cargo test -p read-file`.
  5. **Test with MCP Inspector** -- `npx @modelcontextprotocol/inspector cargo run -p read-file`.
  6. **Stdio transport note** -- explain that the server reads MCP messages from stdin and writes responses to stdout, with all logging directed to stderr.
- The README must not include a "Creating a New Tool" section (that belongs only in the echo-tool reference README).
- The README must not contain speculative information about APIs or features not yet implemented.
- The document must use standard markdown formatting consistent with the project style (heading hierarchy, fenced code blocks for commands).

## Implementation Details

### File to create

- **`tools/read-file/README.md`**

### Document structure

```
# read-file

<One-paragraph description: MCP tool server that reads file contents from disk
and returns them as text. Includes path validation, existence checks, and a
size guard (10 MB) to prevent reading excessively large files.>

## Build

<cargo build command in a fenced code block>

## Run

<cargo run command in a fenced code block, followed by a paragraph explaining
stdio transport: stdin/stdout for MCP messages, stderr for logging>

## Test

<cargo test command in a fenced code block>

## Test with MCP Inspector

<npx inspector command in a fenced code block, with a brief explanation of
what the inspector does and how to use it to verify tool behavior>
```

### Key content details

- The purpose paragraph should mention that the tool accepts a file path, validates it, checks existence and size, reads the contents via `std::fs::read_to_string`, and returns the text or a descriptive error.
- The "Run" section must note that the tool communicates over stdin/stdout using the MCP protocol, and that all logging goes to stderr to avoid corrupting the transport channel.
- The "Test with MCP Inspector" section should briefly explain that the MCP Inspector provides a web UI for sending tool calls and inspecting responses interactively.
- The build command must be exactly `cargo build -p read-file`.
- The run command must be exactly `cargo run -p read-file`.
- The test command must be exactly `cargo test -p read-file`.
- The inspector command must be exactly `npx @modelcontextprotocol/inspector cargo run -p read-file`.

## Dependencies

- Blocked by: "Write unit tests" -- the README documents the implemented tool's behavior and test commands; writing it before tests exist risks documenting behavior that changes during test-driven fixes.
- Blocking: "Run verification suite" -- the verification suite confirms all artifacts including documentation are in place.

## Risks & Edge Cases

1. **API drift:** If the read-file tool implementation changes after the README is written (e.g., method names, error messages, size limit), the README will become inaccurate. Mitigate by keeping descriptions at the behavioral level rather than pinning to exact internal names.
2. **MCP Inspector availability:** The `npx @modelcontextprotocol/inspector` command depends on the inspector package being published to npm. This is an external dependency outside project control.
3. **No "Creating a New Tool" section:** Unlike the echo-tool README, this README should not include a template guide. The echo-tool README is the canonical source for that guidance.

## Verification

1. The file `tools/read-file/README.md` exists and is valid markdown.
2. All required sections are present: purpose, build, run, test, and MCP Inspector.
3. The build command is exactly `cargo build -p read-file`.
4. The run command is exactly `cargo run -p read-file`.
5. The test command is exactly `cargo test -p read-file`.
6. The inspector command is exactly `npx @modelcontextprotocol/inspector cargo run -p read-file`.
7. The stdio transport note is present in the "Run" section.
8. No speculative or incorrect API documentation is present.
9. `cargo test` and `cargo clippy` pass (no code changes, but confirm nothing is broken).
