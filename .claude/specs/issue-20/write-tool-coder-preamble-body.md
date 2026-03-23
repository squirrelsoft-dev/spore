# Spec: Write tool-coder preamble body

> From: .claude/tasks/issue-20.md

## Objective

Write the markdown body (preamble) for `skills/tool-coder.md` after the closing `---` frontmatter delimiter. This preamble is the behavioral instruction set that teaches an LLM how to generate working Rust MCP tool server binaries. It must encode the complete echo-tool reference pattern in enough detail that the LLM can produce compilable Rust crates without seeing the actual echo-tool source code. The tool-coder is the second seed agent in Spore's self-bootstrapping factory, partnered with skill-writer (issue #19).

## Current State

### Existing skill preamble pattern (skill-writer.md)

The `skills/skill-writer.md` preamble follows this structure:
- Opening paragraph establishing the agent's role and relationship to the factory
- A **Skill File Format Specification** section with detailed field tables
- A **Validation Rules** section with numbered rules
- A **Process** section with numbered steps
- An **Output** section describing the structured JSON response

The tool-coder preamble should mirror this depth and structure, but focused on Rust MCP tool generation instead of skill file generation.

### Echo-tool reference pattern (the pattern the preamble must encode)

**File structure**: Each tool lives in `tools/<tool-name>/` with:
- `Cargo.toml` - package definition and dependencies
- `src/main.rs` - entrypoint with logging and stdio transport
- `src/<tool_name>.rs` - tool module with struct, router, handler
- `README.md` - usage documentation
- Optional: `#[cfg(test)] mod tests` within the tool module

**Cargo.toml dependencies**:
```toml
[dependencies]
rmcp = { version = "1", features = ["transport-io", "server", "macros"] }
tokio = { version = "1", features = ["macros", "rt", "io-std"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }
rmcp = { version = "1", features = ["client", "transport-child-process"] }
serde_json = "1"
```

**Key rmcp imports**:
```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
```

**Tool struct pattern**:
- Define a request struct deriving `Debug, serde::Deserialize, schemars::JsonSchema`
- Define a tool struct with a `tool_router: ToolRouter<Self>` field
- Constructor calls `Self::tool_router()` to initialize the router
- `#[tool_router]` on the impl block; `#[tool(description = "...")]` on each method
- Methods accept `Parameters<RequestType>` and return `String`

**ServerHandler pattern**:
- `#[tool_handler]` on `impl ServerHandler for ToolStruct`
- `get_info()` returns `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`

**main.rs pattern**:
- `use rmcp::ServiceExt;`
- `#[tokio::main(flavor = "current_thread")]`
- Initialize `tracing_subscriber` writing to stderr with ansi disabled
- Create tool, call `.serve(rmcp::transport::stdio()).await`
- Call `service.waiting().await`

**Workspace integration**: New crate path must be added to root `Cargo.toml` workspace `members` list.

### Root workspace Cargo.toml

Current workspace members:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/orchestrator`
- `tools/echo-tool`

## Requirements

1. The preamble must begin with an introductory paragraph establishing the tool-coder as the second seed agent in Spore's self-bootstrapping factory, partnered with skill-writer.
2. The preamble must include a **MCP Tool Implementation Pattern** section that documents:
   - The file structure: `tools/<tool-name>/` with `Cargo.toml`, `src/main.rs`, tool module, README
   - The `rmcp` crate imports and macros: `#[tool_router]`, `#[tool_handler]`, `#[tool(...)]`
   - The `ServerHandler` trait implementation with `get_info()` and `ServerCapabilities`
   - The `ToolRouter<Self>` field pattern and `Parameters<T>` wrapper
   - The stdio transport via `rmcp::transport::stdio()` and `ServiceExt`
   - The `Cargo.toml` dependency pattern with exact feature flags
   - The `main.rs` boilerplate with tracing to stderr
   - The request struct pattern with `serde::Deserialize` and `schemars::JsonSchema` derives
3. The preamble must include a **Process** section with numbered steps: parse skill file for `tools: Vec<String>`, query tool-registry for missing tools, infer input/output schemas, generate Rust crate per tool, write files to `tools/<tool-name>/`, run `cargo build`, return results.
4. The preamble must include a **Workspace Integration** note about adding new crates to root `Cargo.toml` members.
5. The preamble must include an **Output** section describing the structured JSON response with fields: `tools_generated` (list of tool names), `compilation_result` (build success/failure), `implementation_paths` (file paths for each generated tool).
6. The preamble must NOT contain any standalone `---` lines (use `----` for horizontal rules if needed).
7. Code examples in the preamble must be accurate reproductions of the echo-tool patterns, not approximations.
8. The preamble must be detailed enough that an LLM can generate a compilable Rust MCP tool without access to the echo-tool source.

## Implementation Details

### File to modify

- `/workspaces/spore/skills/tool-coder.md` -- append the preamble body after the closing `---` of the YAML frontmatter (the frontmatter itself is created by the sibling task "Create skills/tool-coder.md with YAML frontmatter").

### Preamble structure (sections in order)

1. **Introduction paragraph**: Establish the tool-coder as the second seed agent, partnered with skill-writer, responsible for generating Rust MCP tool server binaries from skill file tool requirements.

2. **MCP Tool Implementation Pattern** section with these subsections:
   - **File Structure**: describe the `tools/<tool-name>/` layout with `Cargo.toml`, `src/main.rs`, `src/<tool_name>.rs`, and `README.md`
   - **Cargo.toml**: show the exact dependency block (rmcp with features `transport-io`, `server`, `macros`; tokio with `macros`, `rt`, `io-std`; serde with `derive`; serde_json; tracing; tracing-subscriber with `env-filter`). Also show dev-dependencies for testing.
   - **Tool Module**: show the request struct pattern with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]`, tool struct with `tool_router: ToolRouter<Self>` field, `#[tool_router]` impl with `#[tool(description = "...")]` methods accepting `Parameters<T>`, and `#[tool_handler]` on `impl ServerHandler` with `get_info()` returning `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`
   - **Main Entrypoint**: show the `main.rs` pattern with `use rmcp::ServiceExt`, `#[tokio::main(flavor = "current_thread")]`, tracing to stderr with `.with_ansi(false)`, and `.serve(rmcp::transport::stdio()).await` followed by `service.waiting().await`
   - **README**: note that each tool should have a README with build/run/test instructions
   - All code examples should use generic placeholder names (e.g., `MyTool`, `MyRequest`) not echo-specific names

3. **Process** section (numbered steps):
   1. Parse the input skill file to extract the `tools: Vec<String>` field
   2. Query the tool-registry to identify which tools are missing (not yet implemented)
   3. For each missing tool, infer the input/output schema from the skill's context and the tool name
   4. Generate a complete Rust crate for each missing tool following the MCP Tool Implementation Pattern
   5. Write all generated files to `tools/<tool-name>/`
   6. Add the new crate to the root `Cargo.toml` workspace members list
   7. Run `cargo build -p <tool-name>` to verify compilation
   8. Return structured results

4. **Workspace Integration** section:
   - Instruct the agent to add `"tools/<tool-name>"` to the `[workspace] members` array in the root `Cargo.toml`
   - Note that this must happen before `cargo build` will recognize the new crate

5. **Output** section:
   - `tools_generated`: list of tool name strings that were created
   - `compilation_result`: string describing build outcome (success or error details)
   - `implementation_paths`: map of tool name to list of file paths created for that tool

### Key patterns that MUST appear in code examples

- `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` on request structs
- `tool_router: ToolRouter<Self>` field in tool struct
- `Self::tool_router()` call in constructor
- `#[tool_router]` attribute on impl block
- `#[tool(description = "...")]` attribute on methods
- `Parameters(request): Parameters<RequestType>` method signature
- `#[tool_handler]` attribute on `impl ServerHandler`
- `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`
- `rmcp::transport::stdio()` in main
- `#[tokio::main(flavor = "current_thread")]`
- `.with_writer(std::io::stderr)` and `.with_ansi(false)` for tracing

## Dependencies

- Blocked by: None (same file as frontmatter, but logically follows it)
- Blocking: "Add integration test for tool-coder skill"

## Risks & Edge Cases

- **Preamble contains standalone `---`**: The frontmatter parser uses `---` as a delimiter. Any standalone `---` line in the markdown body would break parsing. Mitigation: use `----` for horizontal rules, and review the final output for accidental `---` lines.
- **Code examples drift from actual echo-tool**: If the echo-tool pattern changes, the preamble becomes stale. Mitigation: the preamble references the echo-tool as canonical and the integration test verifies keyword presence.
- **Preamble too vague for code generation**: If the pattern is described abstractly rather than with concrete code, LLMs may produce non-compiling output. Mitigation: include full code blocks with exact import paths, derives, and macro usage.
- **Missing dev-dependencies pattern**: The preamble should mention dev-dependencies for testing (rmcp client features, transport-child-process) so generated tools are testable.
- **Community skills unavailable**: `npx` was not available in the build environment, so community skill search (`npx skills find mcp-server-rust` and `npx skills find rust-tool-generation`) could not be executed. No community skills were added. This should be retried when npx becomes available.

## Verification

1. The file `skills/tool-coder.md` has non-empty markdown content after the closing `---` frontmatter delimiter.
2. The preamble contains the string "second seed agent" or equivalent introduction.
3. The preamble contains a section heading for "MCP Tool Implementation Pattern" (or similar).
4. The preamble contains code blocks showing `#[tool_router]`, `#[tool_handler]`, `Parameters<`, `ServerCapabilities::builder().enable_tools().build()`, and `rmcp::transport::stdio()`.
5. The preamble contains a "Process" section with numbered steps including "tool-registry" and "cargo build".
6. The preamble contains a "Workspace Integration" section referencing root `Cargo.toml` members.
7. The preamble contains an "Output" section listing `tools_generated`, `compilation_result`, and `implementation_paths`.
8. The preamble contains NO standalone `---` lines (grep for `^---$` in the body should return zero matches, excluding the frontmatter delimiters).
9. The integration test `load_tool_coder_skill` (created by the blocking task) passes, confirming the preamble contains expected keywords like "MCP", "Rust", "cargo", and "tool-registry".
10. `cargo test -p skill-loader` passes with the new test included.
