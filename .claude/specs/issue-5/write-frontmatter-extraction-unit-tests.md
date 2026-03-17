# Spec: Write frontmatter extraction unit tests

> From: .claude/tasks/issue-5.md

## Objective

Add inline unit tests for the `extract_frontmatter` function in `crates/skill-loader/src/frontmatter.rs`. These tests validate that YAML frontmatter is correctly split from the markdown body, and that malformed inputs produce the expected `SkillError::ParseError` variants. Frontmatter extraction is the first parsing step in the skill-loading pipeline, so correctness here prevents cascading failures in YAML deserialization and `SkillManifest` construction downstream.

## Current State

- **`crates/skill-loader/src/frontmatter.rs`** does not exist yet. It will be created by two preceding tasks: "Define SkillFrontmatter struct" and "Implement frontmatter extraction function."
- **`extract_frontmatter` signature** (from the task breakdown): `pub(crate) fn extract_frontmatter(content: &str) -> Result<(&str, &str), SkillError>`. It returns `Ok((yaml_str, body_str))` on success, where `yaml_str` is the text between the two `---` delimiters and `body_str` is everything after the closing delimiter (trimmed). It returns `Err(SkillError::ParseError { .. })` when delimiters are missing.
- **`SkillError`** is defined in `crates/skill-loader/src/error.rs` (preceding task). It has a `ParseError { path: PathBuf, source: String }` variant. Since `extract_frontmatter` operates on string content (not files), the `path` field will likely be set to a placeholder or empty `PathBuf` within the function. Tests should assert on the error variant discriminant, not on the exact `path` value, to avoid coupling to that implementation detail.
- **`SkillFrontmatter`** is a private struct in the same file, with fields mirroring `SkillManifest` minus `preamble`. The unit tests in this task do NOT test `SkillFrontmatter` deserialization directly; they only test the string-splitting logic of `extract_frontmatter`.
- **Existing test patterns** in the repo (see `crates/agent-sdk/tests/`) use: `#[test]` attribute, descriptive snake_case names, `assert_eq!` for value comparisons, `assert!(result.is_err())` for error cases, and inline string literals for test data.

## Requirements

### 1. Test: valid frontmatter with body

Create a test named `extract_valid_frontmatter_with_body` that:
- Provides input with `---` on the first line, YAML content, a closing `---`, and markdown body text.
- Calls `extract_frontmatter` and asserts `Ok`.
- Asserts the YAML portion contains the expected key-value content (e.g., `name: test`).
- Asserts the body portion contains the expected markdown text.
- Verifies that the YAML portion does NOT include the `---` delimiters themselves.
- Verifies that the body is trimmed of leading/trailing whitespace.

Example input:
```
---
name: test
version: "1.0"
---
# Hello World

This is the body.
```

### 2. Test: valid frontmatter with empty body

Create a test named `extract_valid_frontmatter_with_empty_body` that:
- Provides input with valid frontmatter delimiters but nothing after the closing `---` (or only whitespace).
- Calls `extract_frontmatter` and asserts `Ok`.
- Asserts the YAML portion is non-empty and correct.
- Asserts the body portion is an empty string (after trimming).

Example input:
```
---
name: minimal
version: "1.0"
---
```

### 3. Test: missing opening delimiter

Create a test named `extract_missing_opening_delimiter` that:
- Provides input that does NOT start with `---` (no frontmatter at all, just markdown).
- Calls `extract_frontmatter` and asserts `Err`.
- Asserts the error is `SkillError::ParseError`.
- Asserts the error's `source` string mentions the missing opening delimiter (use a substring check like `contains("---")` or `contains("delimiter")` or `contains("frontmatter")`).

Example input:
```
name: test
version: "1.0"
---
# Body text
```

### 4. Test: missing closing delimiter

Create a test named `extract_missing_closing_delimiter` that:
- Provides input that starts with `---` but never has a second `---` line.
- Calls `extract_frontmatter` and asserts `Err`.
- Asserts the error is `SkillError::ParseError`.

Example input:
```
---
name: test
version: "1.0"
# This never closes
```

### 5. Test: body containing `---` horizontal rules

Create a test named `extract_body_with_horizontal_rules` that:
- Provides input with valid frontmatter, followed by a body that contains `---` as a markdown horizontal rule (thematic break).
- Calls `extract_frontmatter` and asserts `Ok`.
- Asserts the YAML portion contains only the frontmatter content (not the horizontal rule).
- Asserts the body portion contains the `---` horizontal rule as part of the body text.
- This test verifies that `extract_frontmatter` finds only the FIRST closing delimiter after the opening one, and does not greedily consume body content.

Example input:
```
---
name: test
version: "1.0"
---
# Section One

Some text.

---

# Section Two

More text.
```

### 6. Test: frontmatter with leading whitespace

Create a test named `extract_frontmatter_with_leading_whitespace` that:
- Provides input where the opening `---` is preceded by whitespace (spaces, tabs, or a BOM character).
- Calls `extract_frontmatter` and asserts `Ok`.
- Asserts the YAML portion and body are correctly extracted.
- This verifies the function's tolerance for leading whitespace/BOM as specified in the task breakdown.

Example input (with leading spaces or `\t` before the first `---`):
```
   ---
name: test
version: "1.0"
---
Body text here.
```

## Implementation Details

### File to modify

```
crates/skill-loader/src/frontmatter.rs
```

Append a `#[cfg(test)] mod tests { ... }` block at the end of the file. This is an inline unit test module, not a separate integration test file, because `extract_frontmatter` is `pub(crate)` and not accessible from external test files.

### Imports within the test module

```rust
#[cfg(test)]
mod tests {
    use super::*;

    // tests here
}
```

The `use super::*` import brings `extract_frontmatter` and `SkillError` into scope (both are defined in the same file or re-exported from sibling modules within the crate).

### Error matching pattern

For tests that assert an error variant, use `matches!` macro or pattern matching:

```rust
let err = result.unwrap_err();
assert!(matches!(err, SkillError::ParseError { .. }));
```

Optionally, to check the `source` field contains a useful message:

```rust
if let SkillError::ParseError { source, .. } = &err {
    assert!(source.contains("delimiter"), "expected mention of delimiter in: {source}");
} else {
    panic!("expected ParseError, got: {err:?}");
}
```

### Test naming convention

Follow the project pattern: snake_case descriptive names prefixed with the function under test (e.g., `extract_valid_frontmatter_with_body`). Each test should be a standalone `#[test]` function -- no `#[tokio::test]` needed since `extract_frontmatter` is synchronous.

### Test data approach

Use inline `&str` literals (raw strings with `r#"..."#` or regular string literals with `\n`) for test inputs. Keep test data minimal -- only include the YAML keys needed to demonstrate the parsing behavior (e.g., `name` and `version`), not full `SkillManifest`-equivalent YAML. The tests validate string splitting, not YAML schema compliance.

## Dependencies

- **Blocked by**:
  - "Define SkillError enum" -- the `SkillError::ParseError` variant must exist for error assertions.
  - "Define SkillFrontmatter struct" -- the struct definition lives in the same file.
  - "Implement frontmatter extraction function" -- the `extract_frontmatter` function must exist.
  - "Implement SkillLoader struct and load method" -- per the task breakdown, this test task is explicitly blocked by the SkillLoader implementation (likely because `lib.rs` module declarations must be in place for the crate to compile).

- **Blocking**:
  - "Run verification suite" -- all tests must pass before the issue can be closed.

## Risks & Edge Cases

1. **`SkillError` constructor ergonomics**: `extract_frontmatter` operates on `&str` content without a file path. The `ParseError` variant requires a `path: PathBuf` field. The function implementation may use a placeholder like `PathBuf::from("<frontmatter>")` or `PathBuf::new()`. Tests should NOT assert on the exact `path` value -- only on the variant discriminant and optionally the `source` message. This avoids coupling tests to an implementation detail.

2. **Leading whitespace definition**: The task says the opening `---` may be "preceded by whitespace/BOM." The exact whitespace tolerance (spaces only? tabs? newlines?) depends on the implementation. The test should use a straightforward case (a few leading spaces) and document that BOM handling is also expected. If the implementation uses `.trim_start()` or similar, spaces and tabs will both work.

3. **Closing delimiter detection**: The `---` closing delimiter could appear at the start of a line within the YAML content itself (e.g., in a YAML multi-line string). The task breakdown does not mention this case, and YAML frontmatter conventions assume `---` on its own line acts as a delimiter. The tests in this spec do not cover embedded `---` inside YAML values. If this becomes a real concern, it should be addressed in a follow-up issue.

4. **Trimming behavior**: The task breakdown says the body should be "trimmed." Tests should verify that leading and trailing whitespace on the body is removed. The YAML portion's trimming behavior is less specified -- tests should verify the YAML portion does not include the `---` delimiters but should not over-specify whitespace handling of the YAML content itself (the YAML parser is tolerant of surrounding whitespace).

5. **Empty YAML between delimiters**: A case like `---\n---\nBody` (no YAML content) is not explicitly listed in the required tests but could be added as a bonus. It is not part of this spec's requirements.

6. **Windows line endings (`\r\n`)**: The test inputs use `\n`. If the implementation splits on `\n` only, `\r\n` inputs may behave differently. This edge case is not in scope for this task but is worth noting for future hardening.

## Verification

After implementation, run the following commands:

```bash
cargo check -p skill-loader    # Crate compiles
cargo clippy -p skill-loader   # No warnings
cargo test -p skill-loader     # All tests pass
```

Specifically, confirm these six tests exist and pass:

- `frontmatter::tests::extract_valid_frontmatter_with_body`
- `frontmatter::tests::extract_valid_frontmatter_with_empty_body`
- `frontmatter::tests::extract_missing_opening_delimiter`
- `frontmatter::tests::extract_missing_closing_delimiter`
- `frontmatter::tests::extract_body_with_horizontal_rules`
- `frontmatter::tests::extract_frontmatter_with_leading_whitespace`
