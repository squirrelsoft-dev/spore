# Spec: Run verification suite

> From: .claude/tasks/issue-20.md

## Objective

Run the full Rust workspace verification suite (`cargo check`, `cargo clippy`, `cargo test`) to confirm that the new `skills/tool-coder.md` skill file and its corresponding integration test are correct and that no regressions were introduced. This is the final gate before the issue can be closed.

## Current State

The workspace contains six crates defined in the root `Cargo.toml`:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/orchestrator`
- `tools/echo-tool`

Existing integration tests in `crates/skill-loader/tests/example_skills_test.rs` cover four skills:
- `load_cogs_analyst_skill`
- `load_echo_skill`
- `load_skill_writer_skill`
- `load_orchestrator_skill`

After the preceding tasks in issue-20 are complete, there will be:
- A new skill file at `skills/tool-coder.md` with YAML frontmatter and a preamble body.
- A new test `load_tool_coder_skill` appended to `crates/skill-loader/tests/example_skills_test.rs`.

## Requirements

- `cargo check` must exit 0 with no errors across the entire workspace.
- `cargo clippy` must exit 0 with no warnings treated as errors across the entire workspace.
- `cargo test` must exit 0 with all tests passing across the entire workspace.
- The new `load_tool_coder_skill` test must appear in the test output and pass.
- All four pre-existing `example_skills_test` tests must continue to pass (no regressions).
- `SkillLoader::load("tool-coder")` (exercised by the new test) must succeed when using `AllToolsExist` as the tool checker, confirming the YAML frontmatter parses correctly and all validation rules pass.
- The test must confirm the preamble contains keywords related to MCP/Rust tool implementation guidance (e.g., "MCP" or "mcp", "Rust" or "rust", "cargo" or "build").

## Implementation Details

### Commands to run (in order)

1. **Type check:**
   ```
   cargo check --workspace
   ```
   Expected: exits 0 with no errors.

2. **Lint:**
   ```
   cargo clippy --workspace -- -D warnings
   ```
   Expected: exits 0 with no warnings. The `-D warnings` flag ensures any clippy warning is treated as a hard failure.

3. **Test:**
   ```
   cargo test --workspace
   ```
   Expected: exits 0 with all tests passing. Look for the line:
   ```
   test load_tool_coder_skill ... ok
   ```
   in the `example_skills_test` test binary output.

### What constitutes a pass

- All three commands exit with code 0.
- The `cargo test` output includes `load_tool_coder_skill ... ok`.
- The `cargo test` output includes all four existing example skill tests as `ok`.
- No `FAILED` lines appear in any output.

### What constitutes a fail

- Any command exits with a non-zero code.
- Any test shows `FAILED`.
- The `load_tool_coder_skill` test is missing from output (meaning it was not added or not compiled).
- Clippy produces warnings (with `-D warnings`).

## Dependencies

- Blocked by: "Create `skills/tool-coder.md` with YAML frontmatter", "Write tool-coder preamble body", "Add integration test for tool-coder skill"
- Blocking: None (this is the final task)

## Risks & Edge Cases

- **YAML parsing edge cases:** If `version` in `tool-coder.md` is not quoted (e.g., `0.1` instead of `"0.1"`), YAML may parse it as a float, which could cause the version string assertion to fail. The frontmatter task spec requires quoting.
- **Preamble contains bare `---`:** If the preamble body contains a standalone `---` on a line by itself, the YAML frontmatter parser may interpret it as a second delimiter, truncating the preamble. The preamble task spec explicitly prohibits standalone `---` lines.
- **Clippy lint regressions in unrelated crates:** A new Rust toolchain update could introduce new clippy lints that fail on existing code. If this happens, the issue is unrelated to the tool-coder changes and should be fixed separately.
- **Stub tools (`read_file`, `write_file`, `cargo_build`):** These tools are declared in the tool-coder frontmatter but do not exist in the tool registry. The test uses `AllToolsExist` to bypass tool existence checks. If the test were mistakenly written with a real registry check, it would fail.
- **Test name collision:** Ensure the new test function is named exactly `load_tool_coder_skill` (with underscores, not hyphens) to match Rust naming conventions and the expected test output.

## Verification

- All three commands (`cargo check --workspace`, `cargo clippy --workspace -- -D warnings`, `cargo test --workspace`) exit 0.
- `cargo test --workspace 2>&1 | grep "load_tool_coder_skill"` returns `test load_tool_coder_skill ... ok`.
- `cargo test --workspace 2>&1 | grep -c "FAILED"` returns `0`.
- The five example skill tests all show as `ok`: `load_cogs_analyst_skill`, `load_echo_skill`, `load_skill_writer_skill`, `load_orchestrator_skill`, `load_tool_coder_skill`.
