# Spec: Write `README.md`

> From: .claude/tasks/issue-44.md

## Objective

Create `tools/read-file/README.md` so that developers can quickly understand how to build, run, and test the `read-file` MCP tool server. It mirrors the structure of `tools/echo-tool/README.md` and stays under 40 lines.

## Current State

`tools/echo-tool/README.md` exists and serves as the reference template. It contains sections for Build, Run, Test with MCP Inspector, Creating a New Tool, and Tool Registry. The `read-file` crate will follow the same stdio transport pattern as `echo-tool`.

The `tools/read-file/` directory is being scaffolded by earlier tasks in this issue. By the time this task runs, the integration test in `tools/read-file/tests/read_file_server_test.rs` must already pass, confirming the server works correctly over stdio.

## Requirements

- File path is exactly `tools/read-file/README.md`.
- File is under 40 lines total.
- Contains a Build section with command `cargo build -p read-file`.
- Contains a Run section with command `cargo run -p read-file`.
- Describes the stdio transport: reads MCP messages from stdin, writes responses to stdout, logs to stderr.
- Contains a "Test with MCP Inspector" section with command `npx @modelcontextprotocol/inspector cargo run -p read-file`.
- Describes the tool's input (`path` parameter) and output (file contents string on success, descriptive error string on failure).
- Does not include the "Creating a New Tool" or "Tool Registry" sections from `echo-tool/README.md`; those are specific to the reference template.

## Implementation Details

- File to create: `tools/read-file/README.md`
- Structure (sections in order):
  1. Title and one-sentence description of what the tool does.
  2. **Build** — `cargo build -p read-file` code block.
  3. **Run** — `cargo run -p read-file` code block, followed by the stdio transport explanation.
  4. **Test with MCP Inspector** — `npx @modelcontextprotocol/inspector cargo run -p read-file` code block, brief explanation of what the inspector does.
  5. **Tool** — documents the single exposed MCP tool: its name (`read_file`), the `path` input parameter (string, absolute or relative path), and the output (file contents as a string, or an error string beginning with `"Error"` if the file cannot be read).

## Dependencies

- Blocked by: "Write integration test in `tests/read_file_server_test.rs`" — the server must be fully implemented and tested before documenting it.
- Blocking: "Run verification suite" — the verification task expects all files, including the README, to be in place.

## Risks & Edge Cases

- Keeping the file under 40 lines requires omitting the boilerplate "Creating a New Tool" walkthrough present in `echo-tool/README.md`; that content belongs only in the reference template.
- The `path` parameter accepts both absolute and relative paths; the README should note this without implying symlink or size-limit behavior that is not yet implemented.

## Verification

- `wc -l tools/read-file/README.md` reports fewer than 40 lines.
- The file contains the strings `cargo build -p read-file`, `cargo run -p read-file`, and `npx @modelcontextprotocol/inspector cargo run -p read-file`.
- The file mentions `path` as the input parameter.
- The file mentions both success output (file contents) and error output.
- Visual inspection confirms the structure matches the style of `tools/echo-tool/README.md`.
