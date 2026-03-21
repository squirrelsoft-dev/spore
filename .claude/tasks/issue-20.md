# Task Breakdown: Create tool-coder seed agent

> Create `skills/tool-coder.md`, the second seed agent that reads skill files, identifies missing tools, and implements them as Rust MCP server binaries following the echo-tool reference pattern.

## Group 1 — Create skill file with frontmatter and preamble

_Tasks in this group can be done in parallel._

- [x] **Create `skills/tool-coder.md` with YAML frontmatter** `[S]`
      Create the file with frontmatter matching the SkillManifest schema. Fields: `name: tool-coder`, `version: "0.1"`, `description` summarizing the agent's purpose, `model` with `provider: anthropic`, `name: claude-sonnet-4-6`, `temperature: 0.1`, `tools: [read_file, write_file, cargo_build]`, `constraints` with `max_turns: 15`, `confidence_threshold: 0.85`, `escalate_to: human_reviewer`, `allowed_actions: [read, write, execute]`, `output` with `format: structured_json` and schema keys `tools_generated`, `compilation_result`, `implementation_paths`. Follow the exact format of existing skills like `skills/skill-writer.md` and `skills/orchestrator.md`. Ensure `version` is quoted to prevent YAML float coercion.
      Files: `skills/tool-coder.md`
      Blocking: "Write tool-coder preamble body"

- [x] **Write tool-coder preamble body** `[M]`
      Write the markdown body (preamble) after the closing `---` delimiter. This is the core of the skill — it instructs the LLM on how to generate working Rust MCP tools. The preamble must include:
      (1) An introduction establishing this as the second seed agent in Spore's self-bootstrapping factory, partnered with skill-writer (#19).
      (2) A **MCP Tool Implementation Pattern** section documenting the reference pattern from `tools/echo-tool/`: each tool is a standalone Rust binary crate in `tools/<tool-name>/` with `Cargo.toml`, `src/main.rs`, a tool module file, a README, and optional tests. Document the `rmcp` crate usage: `#[tool_router]` and `#[tool_handler]` macros, `ServerHandler` trait, `ToolRouter`, `Parameters<T>` wrapper, `ServerCapabilities::builder().enable_tools().build()`, and stdio transport via `rmcp::transport::stdio()`. Document the `Cargo.toml` dependency pattern (rmcp with features `transport-io`, `server`, `macros`; tokio; serde; tracing).
      (3) A **Process** section with numbered steps: parse input skill file to extract `tools: Vec<String>`, query tool-registry to identify missing tools, infer tool input/output schema from context, generate Rust crate for each missing tool, write files to `tools/<tool-name>/`, run `cargo build` to verify compilation, return results.
      (4) A **Workspace Integration** note about adding the new crate to the root `Cargo.toml` workspace members list.
      (5) An **Output** section describing the structured JSON response with `tools_generated`, `compilation_result`, and `implementation_paths` fields.
      (6) Avoid any standalone `---` lines in the body (use `----` for horizontal rules if needed).
      Reference files for pattern accuracy: `tools/echo-tool/src/echo.rs` (tool struct pattern), `tools/echo-tool/src/main.rs` (main entrypoint pattern), `tools/echo-tool/Cargo.toml` (dependency pattern), `tools/echo-tool/README.md` (creating new tools guide).
      Files: `skills/tool-coder.md`
      Blocking: "Add integration test for tool-coder skill"

## Group 2 — Integration test

_Depends on: Group 1._

- [x] **Add integration test for tool-coder skill** `[S]`
      Add a `load_tool_coder_skill` test to `crates/skill-loader/tests/example_skills_test.rs` following the exact pattern of existing tests (e.g., `load_skill_writer_skill`, `load_orchestrator_skill`). The test should: call `loader.load("tool-coder").await.unwrap()`, assert all frontmatter fields match expected values (`name`, `version`, `description`, `model.*`, `tools`, `constraints.*`, `output.*`), assert the preamble is non-empty, and assert keyword presence in the preamble (e.g., contains "MCP" or "mcp", contains "Rust" or "rust", contains "cargo" or "build", contains "tool-registry" or "missing tool"). Use the same `make_loader` and `skills_dir` helpers already defined in the test file.
      Files: `crates/skill-loader/tests/example_skills_test.rs`
      Blocked by: "Create `skills/tool-coder.md` with YAML frontmatter", "Write tool-coder preamble body"
      Blocking: "Run verification suite"

## Group 3 — Verification

_Depends on: Group 2._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify all existing tests pass plus the new `load_tool_coder_skill` test. Confirm `SkillLoader::load("tool-coder")` succeeds with `AllToolsExist` and the preamble contains MCP/Rust tool implementation guidance.
      Files: (none — command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **Do not modify frontmatter of other skills**: Only `skills/tool-coder.md` is created. No changes to existing skill files.

2. **Preamble quality is the deliverable**: Like the skill-writer, the tool-coder's effectiveness depends entirely on how accurately its preamble encodes the MCP tool implementation pattern. The preamble must contain enough detail for an LLM to generate compilable Rust code without seeing the actual echo-tool source.

3. **Stub tools are intentional**: `read_file`, `write_file`, and `cargo_build` are declared in frontmatter but do not exist in the tool registry yet. The test uses `AllToolsExist` to bypass this check, matching the approach for skill-writer's `write_file` and `validate_skill`.

4. **`escalate_to: human_reviewer`**: The triage comment specifies this value. It is a non-empty string, so it satisfies the validation rule in `crates/skill-loader/src/validation.rs`.

5. **Reference implementation pattern**: The echo-tool at `tools/echo-tool/` is the canonical example. The preamble must describe this pattern in enough detail that the LLM can replicate it for arbitrary tools.

6. **Workspace `Cargo.toml`**: The preamble should instruct the agent to add new tool crates to the workspace members list in the root `Cargo.toml`.

## Critical Files for Implementation

- `skills/tool-coder.md` — New file to create; the entire deliverable
- `skills/skill-writer.md` — Pattern to follow for frontmatter structure and preamble depth
- `tools/echo-tool/src/echo.rs` — Reference MCP tool implementation the preamble must describe
- `crates/skill-loader/tests/example_skills_test.rs` — Add integration test following existing patterns
- `tools/echo-tool/README.md` — "Creating a New Tool" guide that the preamble should encode
