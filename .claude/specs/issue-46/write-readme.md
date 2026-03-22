# Spec: Write README

> From: .claude/tasks/issue-46.md

## Objective

Create a README for the `tools/validate-skill/` crate that documents the tool's purpose, input parameter, JSON output format, and how to build, run, and test it. Follow the established pattern from `tools/echo-tool/README.md`. This is a documentation-only task with no code changes.

## Current State

- The `tools/validate-skill/` crate will be implemented by earlier tasks in this issue (scaffold, `parse_content` API, core implementation).
- The `tools/echo-tool/README.md` exists and serves as the reference template for all tool READMEs.
- The tool uses `rmcp` with stdio transport, `tokio` for async, and depends on `skill-loader` for parsing and validation logic.
- The tool accepts full skill file content (markdown with YAML frontmatter), validates it against the `SkillManifest` schema, and returns structured JSON indicating success or failure.
- Validation uses `AllToolsExist` as the tool checker, meaning it performs structural validation only (it does not verify that referenced tools actually exist in a registry).

## Requirements

- The README must contain the following sections:
  1. **Purpose** -- describe the validate-skill tool as an MCP tool server that validates skill file YAML frontmatter against the `SkillManifest` schema and returns structured JSON results.
  2. **Build** -- `cargo build -p validate-skill`.
  3. **Run** -- `cargo run -p validate-skill` with a note about stdio transport.
  4. **Test** -- `cargo test -p validate-skill`.
  5. **Test with MCP Inspector** -- `npx @modelcontextprotocol/inspector cargo run -p validate-skill`.
  6. **Input** -- describe the `content` parameter: the full skill file content (markdown with YAML frontmatter delimited by `---`).
  7. **Output** -- describe the JSON output format with three fields: `valid` (boolean), `errors` (array of strings, empty on success), and `manifest` (the parsed `SkillManifest` object, present only when `valid` is `true`).
  8. **Validation note** -- explain that the tool uses `AllToolsExist` as the tool checker, meaning it performs structural validation only (schema conformance, required fields, allowed values) and does not verify whether referenced tools exist in a live registry.
- The README must not include a "Creating a New Tool" section (that belongs only in the echo-tool reference README).
- The README must not contain speculative information about APIs or features not yet implemented.
- The document must use standard markdown formatting consistent with the project style (heading hierarchy, fenced code blocks for commands).

## Implementation Details

### File to create

- **`tools/validate-skill/README.md`**

### Document structure

```
# validate-skill

<One-paragraph description: MCP tool server that validates skill file YAML
frontmatter against the SkillManifest schema and returns structured JSON
indicating whether the content is valid, any errors found, and the parsed
manifest on success.>

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

## Input

<Description of the `content` parameter: the full text of a skill file
including YAML frontmatter between `---` delimiters and the markdown body.>

## Output

<Description of the JSON output format with examples of both success and
failure cases:
- Success: { "valid": true, "errors": [], "manifest": { ... } }
- Failure: { "valid": false, "errors": ["error description", ...] }
>

## Validation Scope

<Note that the tool uses `AllToolsExist` as the tool checker, which means
all tool references are treated as valid. The tool performs structural
validation only: schema conformance, required fields, allowed values for
enums like output_format, and constraint bounds like confidence_threshold.
It does not check whether referenced tools exist in a live tool registry.>
```

### Key content details

- The purpose paragraph should mention that the tool accepts a `content` string containing a full skill file (markdown with YAML frontmatter), parses the frontmatter into a `SkillManifest`, runs validation rules, and returns structured JSON.
- The "Run" section must note that the tool communicates over stdin/stdout using the MCP protocol, and that all logging goes to stderr to avoid corrupting the transport channel.
- The "Test with MCP Inspector" section should briefly explain that the MCP Inspector provides a web UI for sending tool calls and inspecting responses interactively.
- The "Input" section should explain that `content` is the sole input parameter and must contain the full skill file text with YAML frontmatter between `---` delimiters.
- The "Output" section should describe both the success case (`valid: true` with `errors: []` and the parsed `manifest` object) and the failure case (`valid: false` with `errors` containing one or more descriptive strings).
- The "Validation Scope" section should clarify that `AllToolsExist` is a structural-only checker -- it treats all tool references as valid, so the tool validates schema shape and field constraints but not tool existence.
- The build command must be exactly `cargo build -p validate-skill`.
- The run command must be exactly `cargo run -p validate-skill`.
- The test command must be exactly `cargo test -p validate-skill`.
- The inspector command must be exactly `npx @modelcontextprotocol/inspector cargo run -p validate-skill`.
- No "Creating a New Tool" section -- the echo-tool README is the canonical source for that guidance.

## Dependencies

- Blocked by: "Implement `ValidateSkillTool` struct and handler" -- the README documents the implemented tool's behavior, input/output format, and validation semantics; writing it before the tool exists risks documenting behavior that changes during implementation.
- Blocking: None

## Risks & Edge Cases

1. **API drift:** If the validate-skill tool implementation changes after the README is written (e.g., output JSON field names, error message format, validation rules), the README will become inaccurate. Mitigate by describing behavior at a functional level rather than pinning to exact error strings.
2. **MCP Inspector availability:** The `npx @modelcontextprotocol/inspector` command depends on the inspector package being published to npm. This is an external dependency outside project control.
3. **No "Creating a New Tool" section:** Unlike the echo-tool README, this README should not include a template guide.
4. **Output format stability:** The JSON output structure (`valid`, `errors`, `manifest`) is defined by the task spec. If the implementation uses different field names, the README will need updating. Mitigate by verifying field names against the implemented code before finalizing.

## Verification

1. The file `tools/validate-skill/README.md` exists and is valid markdown.
2. All required sections are present: purpose, build, run, test, MCP Inspector, input, output, and validation scope.
3. The build command is exactly `cargo build -p validate-skill`.
4. The run command is exactly `cargo run -p validate-skill`.
5. The test command is exactly `cargo test -p validate-skill`.
6. The inspector command is exactly `npx @modelcontextprotocol/inspector cargo run -p validate-skill`.
7. The stdio transport note is present in the "Run" section.
8. The `content` input parameter is documented.
9. The JSON output format (`valid`, `errors`, `manifest`) is documented with both success and failure examples.
10. The `AllToolsExist` structural-validation-only note is present.
11. No speculative or incorrect API documentation is present.
12. `cargo test` and `cargo clippy` pass (no code changes, but confirm nothing is broken).
