# Spec: Migrate skill-loader tests to use shared fixture

> From: .claude/tasks/issue-56.md

## Objective

Replace the local `valid_frontmatter()` helper in the `skill-loader` test module with the shared `mcp_test_utils::valid_skill_content()` function to eliminate a duplicated skill fixture. Update the one assertion that depends on the old fixture's `output.format: markdown` value, since the shared fixture uses `json`.

## Current State

- **`crates/skill-loader/src/lib.rs`** (lines 96-121) defines a `valid_frontmatter()` function inside `#[cfg(test)] mod tests`. It returns a YAML-frontmatter string with `output.format: markdown`.
- Two tests call `valid_frontmatter()`:
  - `parse_content_valid_returns_expected_manifest` (line 124) — parses the fixture and asserts field values, including `assert_eq!(manifest.output.format, "markdown")` on line 135.
  - `parse_content_body_is_trimmed` (line 171) — replaces the preamble body text and re-parses. Does not assert on `output.format`.
- **`mcp_test_utils::valid_skill_content()`** (not yet implemented; created by the predecessor task "Add shared skill fixture") returns an identical fixture except `output.format: json` instead of `markdown`. All other fields (`name`, `version`, `description`, `model`, `tools`, `constraints`, `output.schema`, preamble body) are the same.
- **`crates/skill-loader/Cargo.toml`** currently has no `mcp-test-utils` dependency.

## Requirements

- Remove the `valid_frontmatter()` function from `crates/skill-loader/src/lib.rs`.
- Replace all calls to `valid_frontmatter()` with `mcp_test_utils::valid_skill_content()`.
- Update the assertion on line 135 from `assert_eq!(manifest.output.format, "markdown")` to `assert_eq!(manifest.output.format, "json")`.
- Add `mcp-test-utils` as a dev-dependency in `crates/skill-loader/Cargo.toml`.
- All existing tests in `skill-loader` must continue to pass.
- No other test logic or assertions should change beyond the format field.

## Implementation Details

### Files to modify

1. **`crates/skill-loader/Cargo.toml`**
   - Add under `[dev-dependencies]`:
     ```toml
     mcp-test-utils = { path = "../mcp-test-utils" }
     ```

2. **`crates/skill-loader/src/lib.rs`**
   - In the `#[cfg(test)] mod tests` block, remove the entire `valid_frontmatter()` function (lines 96-121).
   - At the top of the test module, add `use mcp_test_utils::valid_skill_content;` (dev-dependencies are available in `#[cfg(test)]` modules).
   - In `parse_content_valid_returns_expected_manifest`:
     - Change `let content = valid_frontmatter();` to `let content = valid_skill_content();`.
     - Change `assert_eq!(manifest.output.format, "markdown");` to `assert_eq!(manifest.output.format, "json");`.
   - In `parse_content_body_is_trimmed`:
     - Change `let content = valid_frontmatter().replace(...)` to `let content = valid_skill_content().replace(...)`.

### Assertions that do NOT change

- `assert_eq!(manifest.name, "test-skill")` — same in both fixtures.
- `assert_eq!(manifest.version, "1.0.0")` — same.
- `assert_eq!(manifest.description, "A test skill")` — same.
- `assert_eq!(manifest.model.provider, "openai")` — same.
- `assert_eq!(manifest.model.name, "gpt-4")` — same.
- `assert_eq!(manifest.tools, vec!["read_file", "write_file"])` — same.
- `assert_eq!(manifest.preamble, "This is the preamble body.")` — same.
- The three error-path tests (`parse_content_missing_opening_delimiter_returns_parse_error`, `parse_content_missing_closing_delimiter_returns_parse_error`, `parse_content_invalid_yaml_fields_returns_parse_error`) do not use the fixture at all and are unchanged.

## Dependencies

- **Blocked by:** "Add shared skill fixture constants to `mcp-test-utils`" — `valid_skill_content()` must exist before this task can be implemented.
- **Blocking:** "Run verification suite"

## Risks & Edge Cases

- **Format value mismatch:** The only semantic difference between the old fixture (`markdown`) and the shared fixture (`json`) is the `output.format` field. If any downstream logic in `skill-loader` validates or branches on the format value during parsing, the change from `markdown` to `json` could surface unexpected behavior. However, `parse_content` does not validate format values — it just stores the string. The `validate` function checks against allowed formats but is not called in the unit tests that use this fixture.
- **Import path:** `mcp_test_utils` (with underscore) is the Rust crate name derived from `mcp-test-utils` (with hyphen) in `Cargo.toml`. The `use` statement must use the underscore form.
- **No new external dependencies:** `mcp-test-utils` is a workspace-local crate with `publish = false`.

## Verification

1. `cargo check -p skill-loader` passes with no errors.
2. `cargo test -p skill-loader` passes — all 5 tests in the `tests` module and all integration tests continue to pass.
3. `cargo clippy -p skill-loader` produces no warnings.
4. Confirm `valid_frontmatter` no longer appears anywhere in `crates/skill-loader/src/lib.rs`.
5. Confirm `mcp-test-utils` appears in `[dev-dependencies]` in `crates/skill-loader/Cargo.toml`.
