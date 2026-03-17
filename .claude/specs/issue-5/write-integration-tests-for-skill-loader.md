# Spec: Write integration tests for SkillLoader

> From: .claude/tasks/issue-5.md

## Objective

Create an integration test file that exercises the `SkillLoader` end-to-end: reading a `.md` skill file from disk, parsing YAML frontmatter into a typed `SkillManifest`, and extracting the markdown body as the `preamble`. The tests must also verify that `SkillLoader` returns the correct error variants (`IoError`, `ParseError`) for failure cases. This is the integration-level acceptance gate for the skill-loader crate -- if these tests pass, the loader correctly handles real filesystem reads, frontmatter extraction, YAML deserialization, and `SkillManifest` construction.

## Current State

- The `skill-loader` crate (`crates/skill-loader/`) currently contains only a placeholder `add()` function in `src/lib.rs` and has no dependencies beyond the default.
- The task breakdown (issue-5.md) specifies that before this test file can be written, the following must exist:
  - `SkillError` enum in `crates/skill-loader/src/error.rs` with variants `IoError { path, source }`, `ParseError { path, source }`, and `ValidationError { skill_name, reasons }`.
  - `SkillFrontmatter` struct in `crates/skill-loader/src/frontmatter.rs` (private, `Deserialize`-only, same fields as `SkillManifest` minus `preamble`).
  - `extract_frontmatter()` function in `crates/skill-loader/src/frontmatter.rs`.
  - `SkillLoader` struct in `crates/skill-loader/src/lib.rs` with fields `skill_dir: PathBuf` and `tool_registry: Arc<ToolRegistry>`, a `new()` constructor, and an `async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>` method.
  - Dependencies added to `crates/skill-loader/Cargo.toml`: `serde`, `serde_yaml`, `agent-sdk`, `tool-registry`, `tokio` (with `fs` feature); dev-dependencies: `tokio` (with `macros`, `rt`), `tempfile`.
- The `agent-sdk` crate exports `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema`, and other types. All relevant structs derive `PartialEq`, enabling `assert_eq!` in tests.
- The `tool-registry` crate currently has a placeholder `ToolRegistry` (just the `add()` function in `lib.rs`). The `SkillLoader` takes `Arc<ToolRegistry>` but the load path does not depend on registry behavior, so an empty/default `ToolRegistry` suffices for these tests.
- The canonical YAML for skill frontmatter is established in `crates/agent-sdk/tests/skill_manifest_test.rs` and the README's skill file example. The key difference for skill files is that the YAML lives between `---` delimiters and the preamble is the markdown body below the frontmatter, not a YAML field.

## Requirements

### 1. Test: valid load with full frontmatter and markdown body

Create an async test named `load_valid_skill_with_full_frontmatter` that:
- Creates a `tempfile::tempdir()`.
- Writes a file named `summarize.md` into the temp directory with full YAML frontmatter (between `---` delimiters) containing all `SkillManifest` fields except `preamble`: `name`, `version`, `description`, `model` (with `provider`, `name`, `temperature`), `tools` (non-empty list), `constraints` (with `max_turns`, `confidence_threshold`, `escalate_to`, `allowed_actions`), and `output` (with `format` and `schema`).
- Below the closing `---`, includes a multi-line markdown body that will become the `preamble`.
- Constructs a `SkillLoader` with `skill_dir` pointing to the temp directory and a default `Arc<ToolRegistry>`.
- Calls `loader.load("summarize").await`.
- Asserts the result is `Ok`.
- Asserts every field of the returned `SkillManifest` matches the expected values, including that `preamble` equals the trimmed markdown body.

Use YAML content modeled after the README example (lines 19-53) for realistic test data:
```yaml
name: summarize
version: "1.0"
description: Summarize input text
model:
  provider: anthropic
  name: claude-3-haiku
  temperature: 0.3
tools:
  - web_search
  - file_read
constraints:
  max_turns: 5
  confidence_threshold: 0.85
  escalate_to: human_reviewer
  allowed_actions:
    - summarize
    - clarify
output:
  format: json
  schema:
    summary: string
    confidence: number
```

### 2. Test: IoError for missing file

Create an async test named `load_missing_file_returns_io_error` that:
- Creates a `tempfile::tempdir()`.
- Constructs a `SkillLoader` pointing to the temp directory.
- Calls `loader.load("nonexistent").await`.
- Asserts the result is `Err`.
- Pattern-matches the error to confirm it is `SkillError::IoError` with a `path` that contains `"nonexistent.md"`.

### 3. Test: ParseError for malformed YAML

Create an async test named `load_malformed_yaml_returns_parse_error` that:
- Creates a `tempfile::tempdir()`.
- Writes a file `bad-yaml.md` with valid `---` delimiters but invalid YAML between them (e.g., `name: [unterminated` or a bare `{{{` or indentation errors).
- Constructs a `SkillLoader` and calls `loader.load("bad-yaml").await`.
- Asserts the result is `Err`.
- Pattern-matches to confirm it is `SkillError::ParseError` with a `path` containing `"bad-yaml.md"`.

### 4. Test: ParseError for missing delimiters

Create an async test named `load_missing_delimiters_returns_parse_error` that:
- Creates a `tempfile::tempdir()`.
- Writes a file `no-frontmatter.md` containing plain markdown with no `---` delimiters at all.
- Constructs a `SkillLoader` and calls `loader.load("no-frontmatter").await`.
- Asserts the result is `Err`.
- Pattern-matches to confirm it is `SkillError::ParseError`.

### 5. Test: empty body

Create an async test named `load_skill_with_empty_body` that:
- Creates a `tempfile::tempdir()`.
- Writes a file `minimal.md` with valid frontmatter but nothing after the closing `---` (or only whitespace).
- Constructs a `SkillLoader` and calls `loader.load("minimal").await`.
- Asserts the result is `Ok`.
- Asserts that the returned `SkillManifest.preamble` is an empty string (or contains only whitespace, depending on trimming behavior -- assert it is empty after trimming).

### 6. Test: markdown body with headings, code blocks, and horizontal rules

Create an async test named `load_skill_with_rich_markdown_body` that:
- Creates a `tempfile::tempdir()`.
- Writes a file `rich.md` with valid frontmatter followed by a markdown body that contains:
  - A heading (`# Instructions`)
  - A paragraph of text
  - A code block (triple backticks with content inside)
  - A horizontal rule (`---`) embedded in the body -- this is the critical edge case, verifying that `---` inside the body is not confused with a frontmatter delimiter
  - Another paragraph after the horizontal rule
- Constructs a `SkillLoader` and calls `loader.load("rich").await`.
- Asserts the result is `Ok`.
- Asserts that `preamble` contains all the markdown content including the `---` horizontal rule, the code block, and the headings.

## Implementation Details

### File to create

```
crates/skill-loader/tests/skill_loader_test.rs
```

This is a Rust integration test file (lives in `tests/`, not `src/`). It imports from the crate's public API.

### Imports

```rust
use std::sync::Arc;

use agent_sdk::SkillManifest;
use skill_loader::{SkillLoader, SkillError};
use tool_registry::ToolRegistry;
use tempfile::tempdir;
use tokio::fs;
```

The `agent_sdk`, `tool_registry`, and `tempfile` crates must be available to integration tests. `agent_sdk` and `tool_registry` are regular dependencies of `skill-loader`, so they are available transitively. `tempfile` must be listed in `[dev-dependencies]`. `tokio` with `macros` and `rt` features must be in `[dev-dependencies]` for `#[tokio::test]`.

### Helper function

Define a helper to reduce boilerplate across tests:

```rust
fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let registry = Arc::new(ToolRegistry::default());
    SkillLoader::new(dir.to_path_buf(), registry)
}
```

This assumes `ToolRegistry` implements `Default` or has a `new()` constructor. If `ToolRegistry` does not implement `Default`, construct it via whatever constructor is available (currently the crate only has a placeholder `add()` function, so the implementation task will need to provide a constructor).

### Canonical frontmatter YAML for tests

Use a `const` string or inline string for the full frontmatter YAML, modeled after `CANONICAL_YAML` in `skill_manifest_test.rs`. The critical difference from `SkillManifest` YAML is that the frontmatter does NOT contain a `preamble` field -- the preamble comes from the markdown body.

### Writing fixture files

Use `tokio::fs::write` (async) or `std::fs::write` (sync, acceptable in tests) to create `.md` files inside the `tempdir`. Each test creates its own `tempdir()` for isolation.

### Error matching pattern

Since `SkillError` does not derive `PartialEq` (it contains `PathBuf` and `String` fields that would make equality checks brittle), use `matches!()` macro or `match` statements to verify the error variant:

```rust
let err = result.unwrap_err();
assert!(matches!(err, SkillError::IoError { .. }));
```

For more specific assertions, destructure and check individual fields:

```rust
match err {
    SkillError::IoError { path, .. } => {
        assert!(path.to_string_lossy().contains("nonexistent.md"));
    }
    other => panic!("expected IoError, got: {:?}", other),
}
```

### Test naming convention

Follow the pattern established in `skill_manifest_test.rs` and `micro_agent_test.rs`: snake_case descriptive names explaining what is being verified.

### Async test pattern

Follow the pattern from `micro_agent_test.rs`:
```rust
#[tokio::test]
async fn test_name() {
    // test body
}
```

### SkillManifest field assertions

For the valid-load test, assert every field individually rather than relying on struct-level `PartialEq` (even though `SkillManifest` derives it). This provides better error messages on failure:

```rust
assert_eq!(manifest.name, "summarize");
assert_eq!(manifest.version, "1.0");
assert_eq!(manifest.description, "Summarize input text");
assert_eq!(manifest.model.provider, "anthropic");
assert_eq!(manifest.model.name, "claude-3-haiku");
assert!((manifest.model.temperature - 0.3).abs() < f64::EPSILON);
assert_eq!(manifest.preamble, "You are a summarization assistant.");
// ... etc
```

This mirrors the assertion style in `skill_manifest_test.rs` (lines 36-59).

### Type signatures reference (from issue-5 task breakdown)

- `SkillLoader { skill_dir: PathBuf, tool_registry: Arc<ToolRegistry> }`
- `SkillLoader::new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self`
- `SkillLoader::load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>` (async)
- `SkillError::IoError { path: PathBuf, source: String }`
- `SkillError::ParseError { path: PathBuf, source: String }`
- `SkillError::ValidationError { skill_name: String, reasons: Vec<String> }`

## Dependencies

- **Blocked by:**
  - "Add dependencies to skill-loader Cargo.toml" (Group 1) -- `tempfile`, `tokio`, `agent-sdk`, `tool-registry` must be in dependencies for imports to resolve.
  - "Define SkillError enum" (Group 1) -- `SkillError` must exist for error assertions.
  - "Define SkillFrontmatter struct" (Group 2) -- internal type used by `SkillLoader::load`.
  - "Implement frontmatter extraction function" (Group 2) -- called by `SkillLoader::load`.
  - "Implement SkillLoader struct and load method" (Group 3) -- the primary type under test must exist with `new()` and `load()`.

- **Blocking:**
  - "Run verification suite" (Group 4) -- all tests must pass before the issue can be closed.

## Risks & Edge Cases

1. **`ToolRegistry` constructor availability**: The `tool-registry` crate currently only has a placeholder `add()` function. The `SkillLoader` takes `Arc<ToolRegistry>`, but the tests need to construct one. If `ToolRegistry` does not implement `Default` or have a `new()` constructor, the test helper will need adjustment. Mitigation: the implementation task for `SkillLoader` will need to ensure `ToolRegistry` is constructible; worst case, add `#[derive(Default)]` to `ToolRegistry` or provide a `ToolRegistry::new()`.

2. **Horizontal rule `---` in markdown body**: The most important edge case. After the frontmatter's closing `---`, the markdown body may contain its own `---` as a horizontal rule. The `extract_frontmatter` function must only split on the first two `---` delimiters. Test 6 specifically verifies this. If the parser naively splits on all `---` occurrences, this test will fail.

3. **Body trimming behavior**: The task breakdown says the body is "trimmed" after extraction. Tests should account for whether leading/trailing whitespace is stripped. Test 5 (empty body) checks this explicitly. Test 1 and 6 should assert against the trimmed body content.

4. **File extension convention**: `SkillLoader::load` constructs the path as `skill_dir.join(format!("{skill_name}.md"))`. Tests must use the skill name without the `.md` extension when calling `load()`. The fixture files must be named `{name}.md`.

5. **Tempdir cleanup**: `tempfile::tempdir()` returns a `TempDir` that deletes its contents on drop. The `TempDir` must be held in a variable for the duration of each test (not just its path), otherwise the directory is deleted before the loader reads the file.

6. **Cross-platform path handling**: `SkillError::IoError.path` is a `PathBuf`. Path assertions should use `path.to_string_lossy().contains(...)` rather than exact string equality to handle OS-specific path separators.

7. **YAML `preamble` field absence**: The frontmatter YAML must NOT contain a `preamble` field. The `SkillFrontmatter` struct intentionally omits it. If a test accidentally includes `preamble:` in the YAML, `serde_yaml` will either ignore it (if `deny_unknown_fields` is not set) or error. Tests should be careful to match the `SkillFrontmatter` struct shape.

8. **`serde_yaml` strict vs. lenient parsing**: If the `SkillFrontmatter` uses `#[serde(deny_unknown_fields)]`, any extra YAML keys will cause a `ParseError`. Tests should only include fields that `SkillFrontmatter` expects. If it does not use `deny_unknown_fields`, extra keys are silently ignored.

## Verification

After implementation, run the following commands (per CLAUDE.md):

```bash
cargo check -p skill-loader    # Ensure the test file compiles
cargo clippy -p skill-loader   # No warnings
cargo test -p skill-loader     # All tests pass
```

Specifically, confirm these six tests exist and pass:

- `skill_loader_test::load_valid_skill_with_full_frontmatter`
- `skill_loader_test::load_missing_file_returns_io_error`
- `skill_loader_test::load_malformed_yaml_returns_parse_error`
- `skill_loader_test::load_missing_delimiters_returns_parse_error`
- `skill_loader_test::load_skill_with_empty_body`
- `skill_loader_test::load_skill_with_rich_markdown_body`

Additionally, verify that existing tests across the workspace continue to pass (no regressions):

```bash
cargo test    # Full workspace test suite
```
