# Spec: Implement SkillLoader struct and load method

> From: .claude/tasks/issue-5.md

## Objective

Create the `SkillLoader` struct and its `load` method in `crates/skill-loader/src/lib.rs`. This is the primary entry point for the skill-loader crate: given a skill name, it reads a Markdown file with YAML frontmatter from disk, parses it, and returns a fully-populated `SkillManifest`. This task also wires up the crate's module declarations and re-exports so that downstream crates can use `skill_loader::SkillLoader` and `skill_loader::SkillError`.

## Current State

- `crates/skill-loader/src/lib.rs` contains only a placeholder `add()` function and a trivial test. No real types or logic exist.
- `crates/skill-loader/Cargo.toml` has no dependencies listed (edition 2024 only).
- The `SkillManifest` type is defined in `crates/agent-sdk/src/skill_manifest.rs` and re-exported from `agent_sdk`. It has fields: `name`, `version`, `description`, `model` (`ModelConfig`), `preamble` (`String`), `tools` (`Vec<String>`), `constraints` (`Constraints`), and `output` (`OutputSchema`).
- `crates/tool-registry/src/lib.rs` is also a placeholder with an `add()` function. `ToolRegistry` does not exist as a type yet; it will be defined in a separate task. For this task, `ToolRegistry` is stored as a field on `SkillLoader` but is not used in the `load` method -- it exists to support future tool-validation functionality (issue #6).
- The `agent-sdk` crate follows a one-struct-per-file pattern with module declarations and `pub use` re-exports in `lib.rs`.
- Error types in the project follow a manual `Display + Error` impl pattern (no `thiserror`), as seen in `crates/agent-sdk/src/agent_error.rs`.
- Sibling tasks in Groups 1 and 2 will create: `SkillError` (in `error.rs`), `SkillFrontmatter` and `extract_frontmatter` (in `frontmatter.rs`), and the required `Cargo.toml` dependencies. This task depends on all of those.

## Requirements

1. Remove the placeholder `add()` function and its `#[cfg(test)] mod tests` block entirely from `crates/skill-loader/src/lib.rs`.
2. Add module declarations for `error` and `frontmatter` submodules:
   - `mod error;`
   - `mod frontmatter;`
3. Add public re-exports so crate consumers can use the key types directly:
   - `pub use error::SkillError;`
   - The `SkillFrontmatter` struct is `pub(crate)` and must NOT be re-exported.
4. Define a public struct `SkillLoader` with two fields:
   - `skill_dir: PathBuf` -- the directory where skill `.md` files are stored.
   - `tool_registry: Arc<ToolRegistry>` -- a shared reference to the tool registry (used by future validation logic, not by `load` itself).
5. Implement `SkillLoader::new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self` as a straightforward constructor that stores both fields.
6. Implement `pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>` with the following sequential steps:
   - (a) Construct the file path: `self.skill_dir.join(format!("{skill_name}.md"))`.
   - (b) Read the file contents using `tokio::fs::read_to_string(&path).await`. On failure, return `SkillError::IoError` with the path and the error's string representation.
   - (c) Call `frontmatter::extract_frontmatter(&content)` to split the content into a YAML string and a body string. Propagate any `SkillError::ParseError` returned.
   - (d) Deserialize the YAML string into a `SkillFrontmatter` using `serde_yaml::from_str`. On failure, return `SkillError::ParseError` with the path and the deserialization error's string representation.
   - (e) Construct a `SkillManifest` by mapping all fields from the deserialized `SkillFrontmatter` and setting `preamble` to the trimmed body string. Return `Ok(manifest)`.
7. The `load` method must not panic. All error paths must return a `Result::Err` with an appropriate `SkillError` variant.
8. No `#[derive]` macros are needed on `SkillLoader` itself -- it is not a data-transfer type.

## Implementation Details

- **File to modify:** `crates/skill-loader/src/lib.rs` (complete rewrite of the file's contents).
- **Imports needed at the top of `lib.rs`:**
  - `use std::path::PathBuf;`
  - `use std::sync::Arc;`
  - `use agent_sdk::SkillManifest;`
  - `use tool_registry::ToolRegistry;` (note: `ToolRegistry` must exist as a public type in the `tool-registry` crate; if it is still a placeholder, this task assumes a prior or parallel task defines at minimum a `pub struct ToolRegistry;` stub)
  - The `frontmatter` module's `extract_frontmatter` function and `SkillFrontmatter` struct are accessed via `frontmatter::extract_frontmatter` and `frontmatter::SkillFrontmatter` (or brought into scope with `use` statements).
- **`SkillLoader` struct definition:**
  ```
  pub struct SkillLoader {
      skill_dir: PathBuf,
      tool_registry: Arc<ToolRegistry>,
  }
  ```
  Fields are private; construction goes through `new()`.
- **`SkillLoader::new` signature:**
  ```
  pub fn new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self
  ```
- **`SkillLoader::load` signature:**
  ```
  pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>
  ```
- **Error mapping in `load`:**
  - IO error: `SkillError::IoError { path: path.clone(), source: err.to_string() }`
  - YAML parse error: `SkillError::ParseError { path: path.clone(), source: err.to_string() }`
  - Frontmatter extraction errors are returned directly from `extract_frontmatter` (it already returns `SkillError`).
- **SkillManifest construction from SkillFrontmatter + body:**
  ```
  SkillManifest {
      name: fm.name,
      version: fm.version,
      description: fm.description,
      model: fm.model,
      preamble: body.trim().to_string(),
      tools: fm.tools,
      constraints: fm.constraints,
      output: fm.output,
  }
  ```
  where `fm` is the deserialized `SkillFrontmatter` and `body` is the body string returned by `extract_frontmatter`.
- **Module declarations and re-exports** appear at the top of `lib.rs`, before the struct definition.
- **No tests in this file.** Unit tests for frontmatter extraction live in `frontmatter.rs`; integration tests for `SkillLoader` live in `tests/skill_loader_test.rs`. Both are handled by separate tasks in Group 4.

## Dependencies

- **Blocked by:**
  - "Add dependencies to skill-loader Cargo.toml" (Group 1) -- `serde`, `serde_yaml`, `agent-sdk`, `tool-registry`, and `tokio` must be in `Cargo.toml` before this code compiles.
  - "Define SkillError enum" (Group 1) -- `crates/skill-loader/src/error.rs` must exist with `SkillError` and its `IoError`, `ParseError`, and `ValidationError` variants.
  - "Define SkillFrontmatter struct" (Group 2) -- `crates/skill-loader/src/frontmatter.rs` must contain the `SkillFrontmatter` struct with `Deserialize` derive.
  - "Implement frontmatter extraction function" (Group 2) -- `extract_frontmatter` must exist in `frontmatter.rs` before `load` can call it.
- **Blocking:**
  - "Write frontmatter extraction unit tests" (Group 4)
  - "Write integration tests for SkillLoader" (Group 4)
  - "Run verification suite" (Group 4)

## Risks & Edge Cases

1. **`ToolRegistry` type does not exist yet.** The `tool-registry` crate currently has only a placeholder `add()` function. This task requires at minimum a `pub struct ToolRegistry;` stub to exist in `crates/tool-registry/src/lib.rs`. If that stub is not created by a parallel task, this task will fail to compile. Mitigation: ensure that a prior or parallel task defines the stub, or define it inline as part of this task's implementation scope.
2. **`skill_dir` does not exist at runtime.** If the directory passed to `new()` does not exist, `load` will return `SkillError::IoError` when `tokio::fs::read_to_string` fails. This is correct behavior -- the loader does not create directories.
3. **Skill name with path traversal characters.** A `skill_name` like `"../../etc/passwd"` would cause `skill_dir.join(...)` to resolve outside the intended directory. This is not addressed in this task (validation is scoped to issue #6). The `ValidationError` variant exists for this purpose. Mitigation for now: document this as a known limitation.
4. **UTF-8 encoding.** `tokio::fs::read_to_string` returns an error for non-UTF-8 files. This is the desired behavior -- skill files must be valid UTF-8.
5. **Large files.** `read_to_string` reads the entire file into memory. Skill Markdown files are expected to be small (kilobytes), so this is acceptable.
6. **Empty preamble.** If the Markdown body after frontmatter is empty or whitespace-only, `preamble` will be an empty string after `trim().to_string()`. This is valid -- some skills may be metadata-only.
7. **`serde_yaml` deprecation.** The `serde_yaml` crate (0.9) is in maintenance mode; the successor is `serde_yml`. However, the task description specifies `serde_yaml = "0.9"`, matching `agent-sdk`'s dev-dependency, so we use it as specified.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p skill-loader
cargo clippy -p skill-loader
```

Both must pass with no errors and no warnings. Additionally verify:

- The file `crates/skill-loader/src/lib.rs` contains no placeholder `add()` function or test.
- `SkillLoader` has exactly two fields: `skill_dir: PathBuf` and `tool_registry: Arc<ToolRegistry>`.
- `SkillLoader::new` accepts `PathBuf` and `Arc<ToolRegistry>` and returns `Self`.
- `SkillLoader::load` is `pub async fn` returning `Result<SkillManifest, SkillError>`.
- `SkillError` is re-exported from the crate root (`use skill_loader::SkillError` works from external crates).
- `SkillFrontmatter` is NOT publicly accessible from outside the crate.
- Full integration-test validation (loading valid files, error cases) is covered by the separate "Write integration tests for SkillLoader" task in Group 4.
