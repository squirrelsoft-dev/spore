# Task Breakdown: Skill Loader with Markdown Frontmatter Parsing

> Implement the `skill-loader` crate so it reads markdown skill files (`.md` with YAML frontmatter), parses them into typed `SkillManifest` structs, and surfaces clear errors for IO, parse, and validation failures.

## Group 1 — Dependencies and error type

_Tasks in this group can be done in parallel._

- [x] **Add dependencies to skill-loader Cargo.toml** `[S]`
      Add the following dependencies: `serde = { version = "1", features = ["derive"] }`, `serde_yaml = "0.9"`, `agent-sdk = { path = "../agent-sdk" }`, `tool-registry = { path = "../tool-registry" }`, `tokio = { version = "1", features = ["fs"] }`. Add dev-dependencies: `tokio = { version = "1", features = ["macros", "rt"] }` (or merge features if already present), `tempfile = "3"`. The `serde_yaml` version matches agent-sdk's dev-dep; `tokio` with `fs` feature is needed for async file reads; `tempfile` is for tests that write fixture files to disk.
      Files: `crates/skill-loader/Cargo.toml`
      Blocking: All tasks in Group 2

- [x] **Define SkillError enum** `[S]`
      Create a `SkillError` enum in a new file `crates/skill-loader/src/error.rs` with three variants: `IoError { path: PathBuf, source: String }` for file-not-found or read failures, `ParseError { path: PathBuf, source: String }` for missing frontmatter delimiters or YAML deserialization failures, and `ValidationError { skill_name: String, reasons: Vec<String> }` for semantic validation (to be fleshed out by issue #6). Follow the manual `Display + Error` impl pattern from `crates/agent-sdk/src/agent_error.rs` — no `thiserror`, implement `std::fmt::Display` and `std::error::Error` by hand. Keep each `Display` arm under 3 lines.
      Files: `crates/skill-loader/src/error.rs`
      Blocking: "Implement frontmatter extraction function", "Implement SkillLoader struct and load method"

## Group 2 — Core implementation

_Depends on: Group 1. Tasks in this group can be done in parallel._

- [x] **Define SkillFrontmatter struct** `[S]`
      Create a private (non-pub) `SkillFrontmatter` struct in `crates/skill-loader/src/frontmatter.rs`. This struct has the same fields as `SkillManifest` minus `preamble`: `name: String`, `version: String`, `description: String`, `model: ModelConfig`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`. Derive `Deserialize` only. This struct exists solely to deserialize YAML frontmatter which does not contain the preamble field.
      Files: `crates/skill-loader/src/frontmatter.rs`
      Blocking: "Implement SkillLoader struct and load method"

- [x] **Implement frontmatter extraction function** `[S]`
      In `crates/skill-loader/src/frontmatter.rs`, implement a function `pub(crate) fn extract_frontmatter(content: &str) -> Result<(&str, &str), SkillError>` that splits file content into a YAML portion and a markdown body portion. The file must start with `---` (optionally preceded by whitespace/BOM). Find the second `---` delimiter. Return the text between the two delimiters as the YAML string, and everything after the second delimiter as the body string (trimmed). Return `ParseError` if the opening or closing delimiter is missing.
      Files: `crates/skill-loader/src/frontmatter.rs`
      Blocked by: "Define SkillError enum"
      Blocking: "Implement SkillLoader struct and load method"

## Group 3 — SkillLoader struct and load method

_Depends on: Group 2._

- [x] **Implement SkillLoader struct and load method** `[M]`
      In `crates/skill-loader/src/lib.rs`, remove the placeholder `add()` function and its test. Define the `SkillLoader` struct with fields `skill_dir: PathBuf` and `tool_registry: Arc<ToolRegistry>`. Implement a `new()` constructor. Implement `pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>`: (1) construct path as `self.skill_dir.join(format!("{skill_name}.md"))`, (2) read file with `tokio::fs::read_to_string`, (3) call `extract_frontmatter`, (4) deserialize YAML into `SkillFrontmatter`, (5) construct and return `SkillManifest` with body as `preamble`. Add module declarations and re-exports.
      Files: `crates/skill-loader/src/lib.rs`
      Blocked by: "Define SkillFrontmatter struct", "Implement frontmatter extraction function", "Define SkillError enum", "Add dependencies to skill-loader Cargo.toml"
      Blocking: All Group 4 tasks

## Group 4 — Tests and verification

_Depends on: Group 3. Tests can be done in parallel; verification runs last._

- [x] **Write frontmatter extraction unit tests** `[S]`
      Add `#[cfg(test)] mod tests` in `crates/skill-loader/src/frontmatter.rs` with tests for: valid frontmatter with body, valid frontmatter with empty body, missing opening delimiter, missing closing delimiter, body containing `---` horizontal rules, frontmatter with leading whitespace.
      Files: `crates/skill-loader/src/frontmatter.rs`
      Blocked by: "Implement SkillLoader struct and load method"
      Blocking: "Run verification suite"

- [x] **Write integration tests for SkillLoader** `[M]`
      Create `crates/skill-loader/tests/skill_loader_test.rs`. Use `tempfile::tempdir()` for fixture files. Tests: valid load with full frontmatter and markdown body, IoError for missing file, ParseError for malformed YAML, ParseError for missing delimiters, empty body, markdown body with headings/code blocks/horizontal rules. Use `#[tokio::test]`.
      Files: `crates/skill-loader/tests/skill_loader_test.rs`
      Blocked by: "Implement SkillLoader struct and load method"
      Blocking: "Run verification suite"

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the workspace. Ensure no warnings, all tests pass, skill-loader compiles cleanly.
      Blocked by: "Write frontmatter extraction unit tests", "Write integration tests for SkillLoader"
