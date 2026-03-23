# Spec: Add shared skill fixture constants to `mcp-test-utils`

> From: .claude/tasks/issue-56.md

## Objective

Add a public function `valid_skill_content() -> String` to the `mcp-test-utils` crate that returns the canonical valid skill YAML frontmatter fixture. This eliminates three near-identical copies of the same fixture scattered across the codebase.

## Current State

Three files each define their own private helper returning nearly identical skill YAML frontmatter:

1. **`crates/skill-loader/src/lib.rs`** -- `fn valid_frontmatter() -> String` (line 96). Uses `output.format: markdown`.
2. **`tools/validate-skill/src/validate_skill.rs`** -- `fn valid_content() -> String` (line 76). Uses `output.format: json`.
3. **`tools/validate-skill/tests/validate_skill_server_test.rs`** -- `fn valid_skill_content() -> String` (line 20). Uses `output.format: json`.

All three share the same structure: a YAML frontmatter block delimited by `---`, containing fields `name`, `version`, `description`, `model` (with `provider`, `name`, `temperature`), `tools`, `constraints` (with `confidence_threshold`, `max_turns`, `allowed_actions`), `output` (with `format`, `schema`), and a trailing preamble body line. The only difference is the `output.format` value (`markdown` vs `json`).

The `mcp-test-utils` crate is assumed to already exist (created by the "Create `crates/mcp-test-utils` crate" task in Group 1).

## Requirements

- Add a public function `pub fn valid_skill_content() -> String` to `crates/mcp-test-utils/src/lib.rs`.
- The function must return a `String` containing valid skill YAML frontmatter with the following field values:
  - `name: test-skill`
  - `version: "1.0.0"`
  - `description: A test skill`
  - `model.provider: openai`
  - `model.name: gpt-4`
  - `model.temperature: 0.7`
  - `tools: [read_file, write_file]`
  - `constraints.confidence_threshold: 0.8`
  - `constraints.max_turns: 5`
  - `constraints.allowed_actions: [read, write]`
  - `output.format: json` (canonical value -- not `markdown`)
  - `output.schema.result: string`
- The returned string must include the `---` frontmatter delimiters and a trailing preamble body line `This is the preamble body.`, matching the existing fixtures.
- The function must take no arguments and return `String`.
- No new dependencies are required -- this is a pure function returning a string literal.
- Do NOT add any other functions, structs, or modules as part of this task.
- Do NOT modify any consuming crates in this task. Downstream migration of the three existing copies is handled by separate tasks ("Migrate validate-skill tests" and "Migrate skill-loader tests to use shared fixture").

## Implementation Details

### File to modify

**`crates/mcp-test-utils/src/lib.rs`**

- Add the function `pub fn valid_skill_content() -> String` below any existing code in the file.
- Use a raw string literal (`r#"..."#`) to define the YAML content, matching the style used in the existing fixtures.
- Call `.to_string()` on the raw string literal to return an owned `String`.

### Canonical fixture content

The returned string must be exactly:

```
---
name: test-skill
version: "1.0.0"
description: A test skill
model:
  provider: openai
  name: gpt-4
  temperature: 0.7
tools:
  - read_file
  - write_file
constraints:
  confidence_threshold: 0.8
  max_turns: 5
  allowed_actions:
    - read
    - write
output:
  format: json
  schema:
    result: string
---
This is the preamble body.
```

### Why `json` and not `markdown`

Two of the three existing copies use `output.format: json`. The `skill-loader` copy uses `markdown`, but the task description specifies `json` as the canonical value. When the skill-loader tests are migrated (a separate task), they will be updated to use `json` and their assertions adjusted accordingly.

### Integration points

- **`tools/validate-skill/src/validate_skill.rs`** tests will replace local `valid_content()` with `mcp_test_utils::valid_skill_content()` (in the "Migrate validate-skill tests" task).
- **`tools/validate-skill/tests/validate_skill_server_test.rs`** will replace local `valid_skill_content()` with `mcp_test_utils::valid_skill_content()` (in the "Migrate validate-skill tests" task).
- **`crates/skill-loader/src/lib.rs`** tests will replace local `valid_frontmatter()` with `mcp_test_utils::valid_skill_content()` and update assertions for `format: json` (in the "Migrate skill-loader tests" task).

## Dependencies

- Blocked by: "Create `crates/mcp-test-utils` crate" (Group 1)
- Blocking: "Migrate validate-skill tests" (Group 4), "Migrate skill-loader tests to use shared fixture" (Group 4)

## Risks & Edge Cases

- **Fixture drift**: If the `SkillManifest` struct gains new required fields in the future, this fixture will need updating. Since all consumers will share the same function, updates happen in one place -- that is the whole point of this consolidation.
- **`skill-loader` test assertions**: The skill-loader tests currently assert `output.format` equals `"markdown"`. When migrated, those assertions must change to `"json"`. This is expected and documented in the task breakdown (Implementation Note 4). It is NOT part of this task.
- **Function placement in `lib.rs`**: Other Group 2 tasks may also be adding functions to the same `lib.rs` file (e.g., `assert_single_tool`, `unique_temp_dir`). Since they are independent additions, merge conflicts should be minimal -- each adds a distinct function.

## Verification

- Run `cargo check -p mcp-test-utils` and confirm no compiler errors.
- Run `cargo test -p mcp-test-utils` and confirm the crate compiles (no tests are required for this function since it returns a static string, but the crate must compile cleanly).
- Run `cargo clippy -p mcp-test-utils` and confirm no warnings.
- Verify the function is `pub` and accessible from other crates by confirming `cargo check -p mcp-test-utils` succeeds with the function exported.
- Verify the returned string parses correctly by visual inspection against the canonical content above.
