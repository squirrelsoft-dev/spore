# Task Breakdown: Implement validate_skill MCP tool

> Implement `validate_skill` as a standalone Rust MCP server binary that validates skill file YAML frontmatter against the SkillManifest schema, following the echo-tool reference pattern.

## Group 1 — Expose parsing API from skill-loader

_Tasks in this group can be done in parallel._

- [x] **Expose a public `parse_content` function in skill-loader** `[M]`
      The `frontmatter` module and `SkillFrontmatter` struct are `pub(crate)`, so external crates cannot parse skill file content without filesystem I/O. Add a public function (e.g., `pub fn parse_content(content: &str) -> Result<SkillManifest, SkillError>`) to `crates/skill-loader/src/lib.rs` that extracts frontmatter, deserializes it into `SkillFrontmatter`, and builds a `SkillManifest` — reusing the same logic currently in `SkillLoader::load` but without the filesystem read. This avoids code duplication in the validate-skill tool. Add unit tests for the new function with valid and invalid content strings.
      Files: `crates/skill-loader/src/lib.rs`
      Blocking: "Implement `ValidateSkillTool` struct and handler"

## Group 2 — Scaffold the crate

_Depends on: Group 1_

_Tasks in this group can be done in parallel._

- [x] **Create `tools/validate-skill/Cargo.toml`** `[S]`
      Copy and adapt `tools/echo-tool/Cargo.toml`. Change `name = "validate-skill"`. Keep the same base dependencies: `rmcp` with `transport-io`, `server`, `macros` features; `tokio` with `macros`, `rt`, `io-std`; `serde` with `derive`; `serde_json`; `tracing`; `tracing-subscriber` with `env-filter`. Add `skill-loader = { path = "../../crates/skill-loader" }` and `agent-sdk = { path = "../../crates/agent-sdk" }` as dependencies (needed for `parse_content`, `validate`, `AllToolsExist`, and `SkillManifest` serialization). Add the same `[dev-dependencies]` block as echo-tool for integration tests.
      Files: `tools/validate-skill/Cargo.toml`
      Blocking: "Implement `ValidateSkillTool` struct and handler", "Write `main.rs`", "Write integration test"

- [x] **Add `"tools/validate-skill"` to workspace `Cargo.toml`** `[S]`
      Add `"tools/validate-skill"` to the `members` list in the root `Cargo.toml`, following the pattern of other tool entries.
      Files: `Cargo.toml`
      Blocking: "Run verification suite"

## Group 3 — Core implementation

_Depends on: Groups 1 and 2_

- [x] **Implement `ValidateSkillTool` struct and handler** `[M]`
      Create `tools/validate-skill/src/validate_skill.rs`. Define `ValidateSkillRequest { content: String }` with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]` and a doc comment `/// The full skill file content (markdown with YAML frontmatter)`. Define `ValidateSkillTool { tool_router: ToolRouter<Self> }` with `new()` calling `Self::tool_router()`. Annotate the impl block with `#[tool_router]` and add a `validate_skill` method with `#[tool(description = "Validate a skill file's YAML frontmatter against the SkillManifest schema and validation rules")]`. The method should: (1) call `skill_loader::parse_content(&request.content)` to parse the frontmatter into a `SkillManifest`; (2) if parsing fails, return `serde_json::to_string` of `{ "valid": false, "errors": [error_message] }`; (3) if parsing succeeds, call `skill_loader::validate(&manifest, &AllToolsExist)` to run validation rules; (4) if validation fails, return `{ "valid": false, "errors": [reasons] }`; (5) if everything passes, return `{ "valid": true, "errors": [], "manifest": manifest }` using `serde_json::json!` macro. The tool uses `AllToolsExist` as the tool checker since its purpose is structural validation, not tool existence verification. Annotate `impl ServerHandler for ValidateSkillTool` with `#[tool_handler]`. Add inline unit tests covering: valid skill content returns `valid: true` with parsed manifest fields; missing frontmatter returns `valid: false`; invalid YAML returns `valid: false`; validation failures (empty name, bad confidence threshold, etc.) return `valid: false` with specific error messages.
      Files: `tools/validate-skill/src/validate_skill.rs`
      Blocked by: "Create `tools/validate-skill/Cargo.toml`", "Expose a public `parse_content` function in skill-loader"
      Blocking: "Write `main.rs`", "Write integration test"

- [x] **Write `main.rs`** `[S]`
      Create `tools/validate-skill/src/main.rs`. Mirror `tools/echo-tool/src/main.rs` exactly, but change module name to `validate_skill`, struct name to `ValidateSkillTool`, and the log line to `"Starting validate-skill MCP server"`.
      Files: `tools/validate-skill/src/main.rs`
      Blocked by: "Implement `ValidateSkillTool` struct and handler"
      Blocking: "Write integration test"

## Group 4 — Integration test and documentation

_Depends on: Group 3_

_Tasks in this group can be done in parallel._

- [x] **Write integration test** `[M]`
      Create `tools/validate-skill/tests/validate_skill_server_test.rs`. Mirror `tools/echo-tool/tests/echo_server_test.rs`. Use `env!("CARGO_BIN_EXE_validate-skill")` to spawn the binary as a child process via `TokioChildProcess`. Write these tests (each `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]`): (1) `tools_list_returns_validate_skill_tool` — assert `tools.len() == 1` and `tools[0].name == "validate_skill"`; (2) `tools_list_has_correct_description` — assert description contains "Validate"; (3) `tools_list_has_content_parameter` — assert `input_schema` properties contain `"content"`; (4) `tools_call_with_valid_skill_returns_valid_true` — call with a well-formed skill file string, parse response JSON, assert `valid == true` and `manifest` object contains expected fields; (5) `tools_call_with_missing_frontmatter_returns_valid_false` — call with content lacking `---` delimiters, assert `valid == false` and `errors` is non-empty; (6) `tools_call_with_invalid_yaml_returns_valid_false` — call with malformed YAML, assert errors.
      Files: `tools/validate-skill/tests/validate_skill_server_test.rs`
      Blocked by: "Write `main.rs`"
      Blocking: "Run verification suite"

- [x] **Write README** `[S]`
      Create `tools/validate-skill/README.md`. Model after `tools/echo-tool/README.md`. Include: build/run/test commands, description of the `content` input parameter, description of the JSON output format (`valid`, `errors`, `manifest`), MCP Inspector test command, and a note that it uses `AllToolsExist` (structural validation only).
      Files: `tools/validate-skill/README.md`
      Blocked by: "Implement `ValidateSkillTool` struct and handler"
      Blocking: None

## Group 5 — Verification

_Depends on: Groups 1–4_

- [x] **Run verification suite** `[S]`
      Run `cargo build -p validate-skill`, then `cargo test -p validate-skill`, then `cargo clippy -p validate-skill`, then `cargo check` (workspace-wide) to confirm no regressions. All five acceptance criteria from the issue must pass: build succeeds, tests pass, returns `{valid: true, errors: [], manifest: {...}}` for well-formed skill files, returns `{valid: false, errors: ["..."]}` for malformed skill files, and tool is named `validate_skill` in `tools/list`.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

## Implementation Notes

1. **New dependency on skill-loader**: The validate-skill tool depends on `skill-loader` and `agent-sdk` crates. No third-party crates beyond what echo-tool already uses need to be added.
2. **Public parsing API required**: The `frontmatter` module in skill-loader is `pub(crate)`. A new public `parse_content(content: &str) -> Result<SkillManifest, SkillError>` function must be added to avoid duplicating the parsing logic.
3. **Error handling as structured data**: Validation errors are returned as structured JSON (`{valid: false, errors: [...]}`), not as MCP tool failures.
4. **`AllToolsExist` stub**: The issue specifies using `AllToolsExist` as the tool checker because this tool validates structure, not tool existence.
5. **Binary name vs tool name**: Package name `validate-skill` produces binary `validate-skill`. The MCP tool name exposed over the protocol will be `validate_skill`.
6. **Registry is environment-driven**: No code changes to the tool-registry are needed.

## Critical Files

- `crates/skill-loader/src/lib.rs` — Must add public `parse_content` function
- `crates/skill-loader/src/frontmatter.rs` — Frontmatter parsing logic that `parse_content` will wrap
- `crates/skill-loader/src/validation.rs` — Contains `validate()` and `AllToolsExist`
- `tools/echo-tool/src/echo.rs` — Reference tool pattern
- `tools/echo-tool/src/main.rs` — Reference main entrypoint
- `tools/echo-tool/Cargo.toml` — Dependency template
- `tools/echo-tool/tests/echo_server_test.rs` — Integration test pattern
- `Cargo.toml` — Workspace members list
