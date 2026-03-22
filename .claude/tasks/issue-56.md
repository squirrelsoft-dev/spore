# Task Breakdown: Extract shared test utilities and tool boilerplate

> Eliminate duplicated main.rs boilerplate, test helpers, and fixtures across the 4 MCP tools by extracting them into shared crates (`crates/mcp-tool-harness` and `crates/mcp-test-utils`).

## Group 1 — Scaffold shared crates

_Tasks in this group can be done in parallel._

- [x] **Create `crates/mcp-tool-harness` crate with `serve_stdio_tool` function** `[M]`
      Create a new workspace member crate at `crates/mcp-tool-harness` with `publish = false`. Implement a single public async function `serve_stdio_tool<T: ServerHandler>(tool: T, tool_name: &str) -> Result<(), Box<dyn std::error::Error>>` that: (1) initializes tracing to stderr with `EnvFilter` and `.with_ansi(false)`, (2) logs `"Starting {tool_name} MCP server"`, (3) calls `tool.serve(rmcp::transport::stdio()).await`, (4) calls `service.waiting().await`. Dependencies: `rmcp` (with `transport-io`, `server`), `tokio`, `tracing`, `tracing-subscriber` (with `env-filter`). Add the crate to the root `Cargo.toml` workspace members list.
      Files: `crates/mcp-tool-harness/Cargo.toml`, `crates/mcp-tool-harness/src/lib.rs`, `Cargo.toml`
      Blocking: "Migrate echo-tool main.rs", "Migrate read-file main.rs", "Migrate write-file main.rs", "Migrate validate-skill main.rs"

- [x] **Create `crates/mcp-test-utils` crate with `spawn_mcp_client!` macro** `[M]`
      Create a new workspace member crate at `crates/mcp-test-utils` with `publish = false`. Implement a `spawn_mcp_client!` declarative macro that accepts a binary path expression (from `env!("CARGO_BIN_EXE_...")`) and returns `RunningService<RoleClient, ()>`. The macro body should: create a `TokioChildProcess` from a `Command::new(path)`, `.expect("failed to spawn")`, then `().serve(transport).await.expect("failed to connect")`. This must be a macro (not a function) because `env!("CARGO_BIN_EXE_...")` must resolve at compile time in the calling crate. Dependencies: `rmcp` (with `client`, `transport-child-process`), `tokio`. Add the crate to the root `Cargo.toml` workspace members list.
      Files: `crates/mcp-test-utils/Cargo.toml`, `crates/mcp-test-utils/src/lib.rs`, `Cargo.toml`
      Blocking: "Add `assert_single_tool` helper", "Add `unique_temp_dir` helper", "Add shared skill fixture", "Migrate echo-tool tests", "Migrate read-file tests", "Migrate write-file tests", "Migrate validate-skill tests"

## Group 2 — Add test utility functions and fixtures

_Depends on: Group 1_

_Tasks in this group can be done in parallel._

- [x] **Add `assert_single_tool` helper to `mcp-test-utils`** `[S]`
      Add a public async function `assert_single_tool(client: &RunningService<RoleClient, ()>, expected_name: &str, description_contains: &str, expected_params: &[&str])` that calls `list_tools`, asserts exactly 1 tool, asserts name matches, asserts description contains the substring, and asserts each expected param exists in `input_schema.properties`. This consolidates the 3 repeated `tools_list_*` tests per tool into a single call.
      Files: `crates/mcp-test-utils/src/lib.rs`
      Blocked by: "Create `crates/mcp-test-utils` crate"
      Blocking: "Migrate echo-tool tests", "Migrate read-file tests", "Migrate write-file tests", "Migrate validate-skill tests"

- [x] **Add `unique_temp_dir` helper to `mcp-test-utils`** `[S]`
      Move the `unique_temp_dir(test_name: &str) -> PathBuf` function from `tools/write-file/src/write_file.rs` into `crates/mcp-test-utils/src/lib.rs` as a public function. It creates `env::temp_dir().join("spore_tests").join(test_name).join(format!("{}", std::process::id()))`, removes any prior contents, and creates the directory. Use the prefix `spore_tests` instead of `write_file_tests` for generality.
      Files: `crates/mcp-test-utils/src/lib.rs`
      Blocked by: "Create `crates/mcp-test-utils` crate"
      Blocking: "Migrate write-file tests"

- [x] **Add shared skill fixture constants to `mcp-test-utils`** `[S]`
      Add a public function `valid_skill_content() -> String` that returns the canonical valid skill YAML frontmatter fixture (matching the one in `tools/validate-skill/src/validate_skill.rs` and `tools/validate-skill/tests/validate_skill_server_test.rs`). This eliminates 3 near-identical copies. Use `output.format: json` as the canonical value.
      Files: `crates/mcp-test-utils/src/lib.rs`
      Blocked by: "Create `crates/mcp-test-utils` crate"
      Blocking: "Migrate validate-skill tests", "Migrate skill-loader tests to use shared fixture"

## Group 3 — Migrate tool main.rs files

_Depends on: Group 1_

_Tasks in this group can be done in parallel._

- [x] **Migrate echo-tool main.rs to use `serve_stdio_tool`** `[S]`
      Replace the contents of `tools/echo-tool/src/main.rs` with: `mod echo; use echo::EchoTool;` then call `mcp_tool_harness::serve_stdio_tool(EchoTool::new(), "echo-tool").await`. Add `mcp-tool-harness = { path = "../../crates/mcp-tool-harness" }` to `tools/echo-tool/Cargo.toml` dependencies. Remove the now-unnecessary direct dependencies on `tracing`, `tracing-subscriber` from `Cargo.toml` if they are not used elsewhere in the tool's source.
      Files: `tools/echo-tool/src/main.rs`, `tools/echo-tool/Cargo.toml`
      Blocked by: "Create `crates/mcp-tool-harness` crate"
      Blocking: "Run verification suite"

- [x] **Migrate read-file main.rs to use `serve_stdio_tool`** `[S]`
      Same pattern as echo-tool. Replace `tools/read-file/src/main.rs` to call `mcp_tool_harness::serve_stdio_tool(ReadFileTool::new(), "read-file").await`. Add harness dependency.
      Files: `tools/read-file/src/main.rs`, `tools/read-file/Cargo.toml`
      Blocked by: "Create `crates/mcp-tool-harness` crate"
      Blocking: "Run verification suite"

- [x] **Migrate write-file main.rs to use `serve_stdio_tool`** `[S]`
      Same pattern as echo-tool. Replace `tools/write-file/src/main.rs` to call `mcp_tool_harness::serve_stdio_tool(WriteFileTool::new(), "write-file").await`. Add harness dependency.
      Files: `tools/write-file/src/main.rs`, `tools/write-file/Cargo.toml`
      Blocked by: "Create `crates/mcp-tool-harness` crate"
      Blocking: "Run verification suite"

- [x] **Migrate validate-skill main.rs to use `serve_stdio_tool`** `[S]`
      Same pattern as echo-tool. Replace `tools/validate-skill/src/main.rs` to call `mcp_tool_harness::serve_stdio_tool(ValidateSkillTool::new(), "validate-skill").await`. Add harness dependency.
      Files: `tools/validate-skill/src/main.rs`, `tools/validate-skill/Cargo.toml`
      Blocked by: "Create `crates/mcp-tool-harness` crate"
      Blocking: "Run verification suite"

## Group 4 — Migrate integration tests

_Depends on: Groups 2 and 3_

_Tasks in this group can be done in parallel._

- [x] **Migrate echo-tool integration tests** `[M]`
      In `tools/echo-tool/tests/echo_server_test.rs`: (1) Replace `spawn_echo_client()` with `mcp_test_utils::spawn_mcp_client!(env!("CARGO_BIN_EXE_echo-tool"))`. (2) Replace the 3 `tools_list_*` tests with a single test that calls `mcp_test_utils::assert_single_tool(&client, "echo", "Returns the input message unchanged", &["message"]).await`. (3) Keep tool-specific call tests as-is. Add `mcp-test-utils` to dev-dependencies.
      Files: `tools/echo-tool/tests/echo_server_test.rs`, `tools/echo-tool/Cargo.toml`
      Blocked by: "Add `assert_single_tool` helper"
      Blocking: "Run verification suite"

- [x] **Migrate read-file integration tests** `[M]`
      Same pattern: replace `spawn_read_file_client()` with the macro, replace the 3 list tests with `assert_single_tool`, keep call-specific tests. Add `mcp-test-utils` dev-dependency.
      Files: `tools/read-file/tests/read_file_server_test.rs`, `tools/read-file/Cargo.toml`
      Blocked by: "Add `assert_single_tool` helper"
      Blocking: "Run verification suite"

- [x] **Migrate write-file integration tests** `[M]`
      Same pattern: replace `spawn_write_file_client()` with the macro, replace the 3 list tests with `assert_single_tool`. Also replace the local `unique_temp_dir` in `write_file.rs` unit tests with `mcp_test_utils::unique_temp_dir`. Add `mcp-test-utils` to dev-dependencies.
      Files: `tools/write-file/tests/write_file_server_test.rs`, `tools/write-file/src/write_file.rs`, `tools/write-file/Cargo.toml`
      Blocked by: "Add `assert_single_tool` helper", "Add `unique_temp_dir` helper"
      Blocking: "Run verification suite"

- [x] **Migrate validate-skill integration tests** `[M]`
      Same pattern: replace `spawn_validate_skill_client()` with the macro, replace the 3 list tests with `assert_single_tool`. Replace `valid_skill_content()` in the integration test with `mcp_test_utils::valid_skill_content()`. Also replace `valid_content()` in the unit tests with the shared fixture. Add `mcp-test-utils` dev-dependency.
      Files: `tools/validate-skill/tests/validate_skill_server_test.rs`, `tools/validate-skill/src/validate_skill.rs`, `tools/validate-skill/Cargo.toml`
      Blocked by: "Add `assert_single_tool` helper", "Add shared skill fixture"
      Blocking: "Run verification suite"

- [x] **Migrate skill-loader tests to use shared fixture** `[S]`
      Replace `valid_frontmatter()` in `crates/skill-loader/src/lib.rs` tests with `mcp_test_utils::valid_skill_content()`. Add `mcp-test-utils` as a dev-dependency. Note: the skill-loader fixture uses `output.format: markdown` while the shared one uses `json` — update the test to use `json` or parameterize the fixture.
      Files: `crates/skill-loader/src/lib.rs`, `crates/skill-loader/Cargo.toml`
      Blocked by: "Add shared skill fixture"
      Blocking: "Run verification suite"

## Group 5 — Verification

_Depends on: Groups 1–4_

- [ ] **Run verification suite** `[S]`
      Run `cargo build`, `cargo test`, `cargo clippy`, and `cargo check` across the full workspace. Verify acceptance criteria: (1) no `spawn_*_client` function appears in more than one file, (2) no `main.rs` exceeds 5 lines excluding imports, (3) `unique_temp_dir` exists in exactly one place, (4) all tests pass, (5) `cargo clippy` is clean.
      Files: (none — command-line verification only)
      Blocked by: All previous tasks
      Blocking: None

## Implementation Notes

1. **Macro vs function for spawn**: `spawn_mcp_client!` must be a macro because `env!("CARGO_BIN_EXE_...")` resolves at compile time in the calling crate.
2. **No new external dependencies**: Both new crates only use dependencies already in the workspace.
3. **`serve_stdio_tool` generic constraint**: Needs `T: ServerHandler` from rmcp. All 4 tools already satisfy this.
4. **Skill fixture differences**: `valid_frontmatter()` in skill-loader uses `format: markdown`, while validate-skill uses `format: json`. Use `json` as the canonical value and update skill-loader tests accordingly.
5. **`unique_temp_dir` for unit tests**: Add `mcp-test-utils` as dev-dependency (works for both unit and integration tests in Rust).

## Critical Files

- `tools/echo-tool/src/main.rs` — Reference for main.rs boilerplate being replaced
- `tools/echo-tool/tests/echo_server_test.rs` — Reference for integration test pattern being consolidated
- `tools/write-file/src/write_file.rs` — Contains `unique_temp_dir` to extract
- `crates/skill-loader/src/lib.rs` — Contains `valid_frontmatter()` fixture to consolidate
- `Cargo.toml` — Workspace members list
