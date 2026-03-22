# Spec: Write README

> From: .claude/tasks/issue-45.md

## Objective

Create `tools/write-file/README.md` following the established pattern from `tools/echo-tool/README.md`. The README documents the write-file MCP tool server for developers who need to understand its purpose, build it, run it, and test it.

## Current State

No README exists for the write-file tool. The echo-tool README serves as the canonical template for all tool READMEs in the project, covering sections for build, run, MCP Inspector testing, and general notes on transport.

## Requirements

1. Follow the structure and tone of `tools/echo-tool/README.md`.
2. Include a heading and one-line purpose statement: the tool writes content to files, creating parent directories as needed.
3. Include a **Build** section with `cargo build -p write-file`.
4. Include a **Run** section with `cargo run -p write-file` and a note that the server uses stdio transport (reads MCP messages from stdin, writes responses to stdout, logs to stderr).
5. Include a **Test with MCP Inspector** section with `npx @modelcontextprotocol/inspector cargo run -p write-file`.
6. Include a **Test** section with `cargo test -p write-file`.
7. Document the input parameters in a dedicated **Parameters** section:
   - `path` (string) -- absolute or relative path of the file to write. Parent directories are created if they do not exist.
   - `content` (string) -- the content to write to the file.
8. Do not include the "Creating a New Tool" or "Tool Registry" sections (those belong only in the echo-tool reference README).

## Implementation Details

- File location: `tools/write-file/README.md`
- Use fenced `sh` code blocks for all shell commands, matching the echo-tool style.
- Keep prose concise; one short paragraph per section maximum.
- Parameter descriptions should be in a Markdown table or definition list for quick scanning.

## Dependencies

- **Blocked by:** "Write integration tests" -- the test commands referenced in the README must be valid before the README is finalized.
- **Blocking:** "Run verification suite" -- the verification suite expects the README to exist.

## Risks & Edge Cases

- If the crate is renamed or the binary name differs from `write-file`, the build/run/test commands will be wrong. Confirm the package name in `tools/write-file/Cargo.toml` before writing.
- The parameter names (`path`, `content`) must match the actual struct field names in the tool implementation; verify against the source before finalizing.

## Verification

1. Confirm the README renders correctly in a Markdown previewer (no broken links or formatting).
2. Run each documented command and verify it succeeds:
   - `cargo build -p write-file`
   - `cargo test -p write-file`
   - `cargo clippy -p write-file`
3. Verify the parameter names listed in the README match the tool's input schema by calling the tool via MCP Inspector and comparing advertised parameters.
