# Spec: Implement frontmatter extraction function

> From: .claude/tasks/issue-5.md

## Objective

Implement a `pub(crate)` function `extract_frontmatter` in `crates/skill-loader/src/frontmatter.rs` that splits the raw content of a markdown skill file into two parts: the YAML frontmatter string and the markdown body string. This function is the first stage of the skill-loading pipeline -- it performs structural parsing (delimiter detection) without interpreting the YAML or body content. Downstream code (the `SkillLoader::load` method) will deserialize the YAML portion into a `SkillFrontmatter` struct and use the body as the `preamble` field of a `SkillManifest`.

## Current State

- The `skill-loader` crate (`crates/skill-loader/`) is a scaffold containing only a placeholder `add()` function and its test in `src/lib.rs`. No other source files exist.
- `Cargo.toml` has no dependencies beyond the implicit `std`.
- The `SkillError` enum does not exist yet -- it will be created in `crates/skill-loader/src/error.rs` by a sibling task ("Define SkillError enum"). That task defines a `ParseError { path: PathBuf, source: String }` variant that this function must return when delimiters are missing.
- The error pattern used in the workspace (see `crates/agent-sdk/src/agent_error.rs`) is a hand-written enum with manual `Display` and `Error` impls -- no `thiserror`.
- The `SkillManifest` struct (in `agent-sdk`) has a `preamble: String` field that will be populated from the body portion returned by this function.

## Requirements

1. **Function signature:** `pub(crate) fn extract_frontmatter(content: &str) -> Result<(&str, &str), SkillError>` in `crates/skill-loader/src/frontmatter.rs`.

2. **Opening delimiter detection:** The content must start with `---` on its own line. Leading whitespace (spaces, tabs) and/or a Unicode BOM (`\u{FEFF}`) before the `---` are permitted and must be stripped before checking. If no opening `---` is found after stripping, return `SkillError::ParseError` with a descriptive `source` message (e.g., `"missing opening frontmatter delimiter '---'"`) and a placeholder `path` (empty `PathBuf` or a sentinel -- the caller will replace it with the actual file path if needed).

3. **Closing delimiter detection:** After the opening `---`, scan for the next line that is exactly `---` (with no leading whitespace required, but the line must contain only `---` and optional trailing whitespace). If no closing delimiter is found, return `SkillError::ParseError` with a descriptive message (e.g., `"missing closing frontmatter delimiter '---'"`).

4. **Return value on success:** Return `Ok((yaml, body))` where:
   - `yaml` is the `&str` slice between the opening and closing `---` delimiters (exclusive of the delimiter lines themselves). It is not trimmed -- the YAML deserializer handles leading/trailing whitespace.
   - `body` is the `&str` slice of everything after the closing `---` delimiter line, trimmed of leading and trailing whitespace (using `.trim()`).

5. **No YAML parsing:** This function performs only structural splitting. It does not validate that the YAML portion is well-formed YAML. That responsibility belongs to the `SkillLoader::load` method.

6. **No file I/O:** The function operates on an in-memory `&str`. File reading is handled by the caller (`SkillLoader::load`).

7. **PathBuf in ParseError:** Since this function does not know the file path (it receives only content), it must use `PathBuf::from("<unknown>")` (or equivalent sentinel) for the `path` field of `ParseError`. The `SkillLoader::load` method can map/replace this if a more informative error is desired, or the `extract_frontmatter` function signature could be revised to accept a path parameter. Given the task description specifies only `content: &str`, use a sentinel path.

## Implementation Details

### File: `crates/skill-loader/src/frontmatter.rs` (new)

- **Imports:** `use std::path::PathBuf;` and `use crate::error::SkillError;`.
- **Function:** `pub(crate) fn extract_frontmatter(content: &str) -> Result<(&str, &str), SkillError>`

**Algorithm (step by step):**

1. Strip any leading Unicode BOM (`\u{FEFF}`) from `content`. Then trim leading whitespace (spaces, tabs, newlines) from the result.
2. Check that the trimmed content starts with `---`. If not, return `Err(SkillError::ParseError { ... })`.
3. Find the end of the opening delimiter line (the first newline after the initial `---`). The YAML content begins immediately after this newline.
4. Starting from the character after the opening delimiter line, scan line by line for a line whose trimmed content equals `---`.
5. If no such closing line is found, return `Err(SkillError::ParseError { ... })`.
6. Extract the YAML slice: everything between the end of the opening delimiter line and the start of the closing delimiter line.
7. Extract the body slice: everything after the end of the closing delimiter line, then call `.trim()` on it.
8. Return `Ok((yaml, body))`.

**Key detail on borrowing:** Both returned `&str` values must borrow from the original `content` parameter. The BOM/whitespace stripping in step 1 should produce a sub-slice of `content` (using `trim_start_matches` and `trim_start`), not a new allocation, so that all subsequent indices are valid slices of `content`.

### File: `crates/skill-loader/src/lib.rs` (modified -- but NOT by this task)

This task does not modify `lib.rs`. The downstream task "Implement SkillLoader struct and load method" will add:
```rust
mod error;
mod frontmatter;
```

However, for this file to compile in isolation during development, the implementer may temporarily add `mod error; mod frontmatter;` to `lib.rs` for local verification, then revert if that collides with the parallel "Define SkillError enum" task.

### Integration points

- **Consumed by:** `SkillLoader::load` in `crates/skill-loader/src/lib.rs`, which calls `extract_frontmatter(&file_content)`, then deserializes the YAML portion into `SkillFrontmatter`, and uses the body as `SkillManifest.preamble`.
- **Depends on:** `SkillError::ParseError` variant from `crates/skill-loader/src/error.rs`.

## Dependencies

- **Blocked by:** "Define SkillError enum" -- the function returns `SkillError::ParseError`, so the error type must exist before this function can compile.
- **Blocking:** "Implement SkillLoader struct and load method" -- the loader calls `extract_frontmatter` as the first step of its pipeline.

## Risks & Edge Cases

1. **BOM handling:** Some editors (notably Windows Notepad and certain YAML tools) prepend a UTF-8 BOM (`\u{FEFF}`) to files. If the BOM is not stripped, the opening `---` check will fail even though the file is structurally valid. The implementation must strip the BOM before delimiter detection.

2. **`---` inside the body:** Markdown uses `---` as a horizontal rule (thematic break). The function must only match the *first* `---` after the opening delimiter as the closing delimiter. Any subsequent `---` lines in the body are part of the body content and must not be misinterpreted. This is inherently handled by the algorithm: once the closing delimiter is found, everything after it is body.

3. **`---` inside the YAML:** YAML itself uses `---` as a document start marker. However, in the frontmatter convention, the closing `---` is always on its own line at the top level. Nested `---` within YAML values (e.g., in a multiline string) would be indented or inside quotes. The line-by-line scan for a line whose trimmed content equals exactly `---` should not produce false positives in well-formed YAML frontmatter. If a malformed YAML block contains a bare `---` line, it will be treated as the closing delimiter -- this is consistent with how all major static site generators (Jekyll, Hugo, Zola) handle frontmatter.

4. **Empty YAML section:** A file with `---\n---\nbody` has an empty YAML section. The function should return `Ok(("", "body"))`. The caller (YAML deserializer) will fail on the empty string, which is the correct behavior -- the error surfaces as a parse failure, not a frontmatter-extraction failure.

5. **Empty body:** A file with `---\nname: foo\n---` (no trailing content) should return `Ok(("name: foo\n", ""))`. An empty body is valid; the `preamble` field of `SkillManifest` will simply be an empty string.

6. **Windows line endings (`\r\n`):** The line-by-line scan should handle `\r\n` line endings. Using `lines()` on a `&str` in Rust automatically strips `\r`, so `"---\r".trim() == "---"` holds. However, the YAML slice extraction must account for `\r\n` when computing offsets if not using `lines()`. The safest approach is to use `str::find` or manual index arithmetic on the raw content rather than relying on `lines()` for offset tracking, since `lines()` does not preserve byte offsets.

7. **Sentinel path value:** Using `PathBuf::from("<unknown>")` as the `path` in `ParseError` is a pragmatic choice. An alternative is to accept a `path: &Path` parameter, but the task description specifies `content: &str` as the only parameter. If the team later prefers to pass the path in, the signature change is backward-compatible at the crate level since the function is `pub(crate)`.

8. **No trailing newline after closing delimiter:** If the closing `---` is the last line with no trailing newline, the body should be `""` (empty after trim). The implementation must not panic or return an error in this case.

## Verification

1. **Compilation:** After both this task and "Define SkillError enum" are complete, add `mod error; mod frontmatter;` to `lib.rs` (if not already present) and run `cargo check -p skill-loader`.
2. **Clippy:** Run `cargo clippy -p skill-loader` and confirm no warnings. Pay attention to potential clippy lints about manual string searching vs. iterator methods.
3. **Existing tests:** Run `cargo test -p skill-loader` to confirm the placeholder test (if still present) or any new tests pass.
4. **Manual spot-check (before unit tests are written by the Group 4 task):** Verify the function handles these inputs correctly by temporarily adding assertions or a `#[test]`:
   - Standard frontmatter: `"---\nname: test\n---\nHello world"` returns `Ok(("name: test\n", "Hello world"))`.
   - BOM prefix: `"\u{FEFF}---\nname: test\n---\nbody"` returns `Ok(...)` (does not error on BOM).
   - Missing opening delimiter: `"no frontmatter here"` returns `Err(ParseError { ... })`.
   - Missing closing delimiter: `"---\nname: test\nno closing"` returns `Err(ParseError { ... })`.
   - Body with `---` horizontal rule: `"---\nname: test\n---\nbody\n---\nmore body"` returns body as `"body\n---\nmore body"`.
   - Empty body: `"---\nname: test\n---"` returns `Ok(("name: test\n", ""))`.
5. **No external dependencies:** Confirm the function uses only `std` types and the crate-local `SkillError`. No `serde_yaml`, `regex`, or other crate imports.
