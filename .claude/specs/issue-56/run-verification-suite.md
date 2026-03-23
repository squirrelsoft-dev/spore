# Spec: Run verification suite

> From: .claude/tasks/issue-56.md

## Objective
Run the full build, test, lint, and type-check suite across the entire workspace and verify the five acceptance criteria for the shared-utilities extraction performed in issue-56: no duplicated spawn helpers, slim main.rs files, single-location temp dir helper, all tests passing, and clean clippy output.

## Current State
By the time this task runs, all preceding issue-56 tasks will have:
1. Created `crates/mcp-tool-harness` with `serve_stdio_tool` function
2. Created `crates/mcp-test-utils` with `spawn_mcp_client!` macro, `assert_single_tool` helper, `unique_temp_dir` helper, and `valid_skill_content` fixture
3. Migrated all four tool `main.rs` files (`echo-tool`, `read-file`, `write-file`, `validate-skill`) to use `mcp_tool_harness::serve_stdio_tool`
4. Migrated all four tool integration test files to use `spawn_mcp_client!` and `assert_single_tool` from `mcp-test-utils`
5. Moved `unique_temp_dir` out of `write-file` into `mcp-test-utils`
6. Consolidated skill fixture into `mcp_test_utils::valid_skill_content()` and updated `validate-skill` and `skill-loader` tests
7. Added both new crates to the root `Cargo.toml` workspace members list

The workspace members now include: `agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, `orchestrator`, `mcp-tool-harness`, `mcp-test-utils`, `echo-tool`, `read-file`, `write-file`, `validate-skill`.

## Requirements
1. `cargo build` (full workspace) exits with code 0
2. `cargo test` (full workspace) exits with code 0 with all non-ignored tests passing
3. `cargo clippy -- -D warnings` (full workspace) exits with code 0 with zero warnings
4. `cargo check` (full workspace) exits with code 0
5. No `spawn_*_client` helper function is defined in more than one file across the workspace
6. No `main.rs` in `tools/*/src/main.rs` exceeds 5 lines excluding `use`/`mod` import lines
7. `unique_temp_dir` is defined in exactly one place (`crates/mcp-test-utils/src/lib.rs`)
8. No commented-out code or debug `println!` statements remain in any modified file

## Implementation Details
This task does not create or modify source files. It is a verification-only task. Run each step in order; if any step fails, diagnose the root cause and fix the relevant file before proceeding.

### Step 1: Full workspace build
```bash
cargo build
```
Confirms all workspace members compile, including the two new crates (`mcp-tool-harness`, `mcp-test-utils`) and the migrated tools. Likely failure causes:
- Missing dependency in `Cargo.toml` for a new or migrated crate
- Import path errors after migration (e.g., `mcp_tool_harness::serve_stdio_tool` not found)
- Macro export issues with `spawn_mcp_client!`

### Step 2: Full workspace tests
```bash
cargo test
```
Runs all unit and integration tests across every workspace member. Tests marked `#[ignore]` are skipped and do not count as failures. Verify the test summary shows 0 failures. Likely failure causes:
- `spawn_mcp_client!` macro not producing a valid client (transport or connection error)
- `assert_single_tool` assertion mismatches after migration (wrong tool name, description, or parameter names)
- `unique_temp_dir` path prefix change from `write_file_tests` to `spore_tests` causing test assumptions to break
- `valid_skill_content()` fixture using `json` format where a test expected `markdown`

### Step 3: Clippy lint check
```bash
cargo clippy -- -D warnings
```
The `-D warnings` flag promotes all warnings to errors. Verify clean exit with no output beyond "Finished". Likely failure causes:
- Unused imports left behind after removing inlined helpers
- Unused dependencies in `Cargo.toml` after moving `tracing`/`tracing-subscriber` to the harness crate
- Dead code warnings for functions that were extracted but not fully removed from the original location

### Step 4: Type check
```bash
cargo check
```
Full workspace type-check. This is faster than `cargo build` and catches type errors without linking. Serves as a final cross-crate consistency check.

### Step 5: Structural acceptance checks
Run the following verification checks from the workspace root:

**5a. No duplicated `spawn_*_client` helpers:**
Search the entire workspace for function definitions matching `fn spawn_*_client`. Each pattern (e.g., `spawn_echo_client`, `spawn_read_file_client`) must appear in zero files (fully replaced by the macro) or at most one file. If any `spawn_*_client` function definition appears in more than one file, the migration is incomplete.
```bash
grep -rn "fn spawn_.*_client" tools/ crates/
```
Expected: zero matches, or matches only within `mcp-test-utils` if a generic helper was kept.

**5b. Slim `main.rs` files:**
For each `tools/*/src/main.rs`, count the non-import lines (lines that do not start with `use ` or `mod `). Each must have at most 5 such lines.
```bash
for f in tools/*/src/main.rs; do
  count=$(grep -cvE '^\s*(use |mod |//|$)' "$f")
  echo "$f: $count non-import lines"
  if [ "$count" -gt 5 ]; then
    echo "FAIL: $f exceeds 5 non-import lines"
  fi
done
```

**5c. `unique_temp_dir` in exactly one place:**
Search for the function definition across the workspace. It must appear in exactly one file: `crates/mcp-test-utils/src/lib.rs`.
```bash
grep -rn "fn unique_temp_dir" tools/ crates/
```
Expected: exactly one match in `crates/mcp-test-utils/src/lib.rs`.

If any structural check fails, diagnose which migration task was incomplete and apply the minimal fix.

### Files potentially touched (fixes only, if needed)
- `tools/echo-tool/src/main.rs` -- leftover imports or boilerplate
- `tools/read-file/src/main.rs` -- leftover imports or boilerplate
- `tools/write-file/src/main.rs` -- leftover imports or boilerplate
- `tools/validate-skill/src/main.rs` -- leftover imports or boilerplate
- `tools/write-file/src/write_file.rs` -- leftover `unique_temp_dir` definition
- `tools/validate-skill/src/validate_skill.rs` -- leftover fixture function
- `tools/*/tests/*_server_test.rs` -- leftover `spawn_*_client` functions
- `tools/*/Cargo.toml` -- unused dependency cleanup
- `crates/skill-loader/src/lib.rs` -- fixture migration fix
- `crates/mcp-test-utils/src/lib.rs` -- helper function fixes
- `crates/mcp-tool-harness/src/lib.rs` -- `serve_stdio_tool` signature fixes

## Dependencies
- Blocked by: "Create `crates/mcp-tool-harness` crate", "Create `crates/mcp-test-utils` crate", "Add `assert_single_tool` helper", "Add `unique_temp_dir` helper", "Add shared skill fixture constants", "Migrate echo-tool main.rs", "Migrate read-file main.rs", "Migrate write-file main.rs", "Migrate validate-skill main.rs", "Migrate echo-tool integration tests", "Migrate read-file integration tests", "Migrate write-file integration tests", "Migrate validate-skill integration tests", "Migrate skill-loader tests to use shared fixture"
- Blocking: None (this is the final task for issue-56)

## Risks & Edge Cases
- **Macro hygiene**: The `spawn_mcp_client!` macro must work correctly when invoked from different crate contexts. If the macro relies on items not re-exported or not in scope, tests will fail to compile. Fix by ensuring the macro uses fully qualified paths (e.g., `::rmcp::transport::TokioChildProcess`).
- **Unused dependency warnings**: After moving `tracing` and `tracing-subscriber` from tool crates into `mcp-tool-harness`, clippy or cargo may warn about unused dependencies in the tool `Cargo.toml` files. Remove them if no other source file in that crate references them.
- **Feature gate mismatches**: The `mcp-test-utils` crate needs `rmcp` with `client` and `transport-child-process` features. If these features are not correctly specified, integration tests will fail to compile.
- **Skill fixture format divergence**: The canonical `valid_skill_content()` uses `format: json`. If any test in `skill-loader` was asserting `format: markdown`, that test must be updated to match the new canonical value.
- **Cross-platform temp dir**: `unique_temp_dir` uses `std::env::temp_dir()`. On different platforms this resolves to different paths. Tests should not hardcode path separators or assume a specific temp directory structure.
- **Edition 2024 lints**: The workspace uses `edition = "2024"`, which may surface new lints on the migrated code. Address each individually.

## Verification (Pass/Fail Criteria)
- **PASS**: `cargo build` exits with code 0
- **PASS**: `cargo test` exits with code 0, summary shows 0 failures across all crates
- **PASS**: `cargo clippy -- -D warnings` exits with code 0, no warning or error output
- **PASS**: `cargo check` exits with code 0
- **PASS**: `grep -rn "fn spawn_.*_client" tools/ crates/` returns zero matches or matches in at most one file
- **PASS**: Every `tools/*/src/main.rs` has at most 5 non-import lines
- **PASS**: `grep -rn "fn unique_temp_dir" tools/ crates/` returns exactly one match in `crates/mcp-test-utils/src/lib.rs`
- **FAIL**: Any of the above checks does not meet its criterion
