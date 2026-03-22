# Spec: Expose a public `parse_content` function in skill-loader
> From: .claude/tasks/issue-46.md

## Objective

Add a public free function `parse_content` to the `skill_loader` crate that parses a skill file's raw string content into a `SkillManifest` without performing any filesystem I/O. This lets external crates (e.g., a future `ValidateSkillTool`) parse and inspect skill content that has already been loaded into memory, eliminating the need to duplicate frontmatter extraction and deserialization logic.

## Current State

- `SkillLoader::load` is the only way to obtain a `SkillManifest`. It reads a file from disk, extracts YAML frontmatter via `frontmatter::extract_frontmatter`, deserializes it into `SkillFrontmatter`, and assembles a `SkillManifest`.
- Both `SkillFrontmatter` and `extract_frontmatter` are `pub(crate)`, so no external crate can call them directly.
- `SkillError::ParseError` currently requires a `PathBuf` field (`path`), even for content that did not originate from a file. The existing frontmatter helpers already use `PathBuf::from("<unknown>")` as a sentinel when no real path is available, so this convention is already established.

## Requirements

1. Add a public function with this signature in `crates/skill-loader/src/lib.rs`:
   ```rust
   pub fn parse_content(content: &str) -> Result<SkillManifest, SkillError>
   ```
2. The function must:
   - Call `frontmatter::extract_frontmatter` to split YAML from body.
   - Deserialize the YAML into `frontmatter::SkillFrontmatter`.
   - Build and return a `SkillManifest` with the same field mapping used in `SkillLoader::load`.
   - Use `PathBuf::from("<content>")` (or similar sentinel) for the `path` field inside any `SkillError::ParseError` it produces, since there is no file path.
3. The function must **not** run validation (`validate`). Validation requires a `ToolExists` implementation, and the caller should be free to validate separately with whatever tool checker it has. This keeps `parse_content` focused on parsing only.
4. Refactor `SkillLoader::load` to call `parse_content` internally for the extraction + deserialization + manifest-building steps, then layer on its own path-aware error mapping and validation call. This eliminates code duplication.
5. Add unit tests covering at minimum:
   - Valid content with all required frontmatter fields produces the expected `SkillManifest`.
   - Content missing the opening `---` delimiter returns a `SkillError::ParseError`.
   - Content missing the closing `---` delimiter returns a `SkillError::ParseError`.
   - Content with valid delimiters but invalid/missing YAML fields returns a `SkillError::ParseError`.
   - Body text (preamble) is trimmed and correctly assigned.
6. Re-export `parse_content` from the crate root so it is accessible as `skill_loader::parse_content`.

## Implementation Details

1. **New function** `parse_content` in `lib.rs` (module-level free function, not an `impl` method):
   - Extract frontmatter: `let (yaml, body) = frontmatter::extract_frontmatter(content)?;`
   - Deserialize: `let fm: SkillFrontmatter = serde_yaml::from_str(yaml).map_err(|err| SkillError::ParseError { path: PathBuf::from("<content>"), source: err.to_string() })?;`
   - Build manifest from `fm` fields and `body.trim().to_string()` for `preamble`, identical to the current mapping in `SkillLoader::load`.
   - Return `Ok(manifest)`.

2. **Refactor `SkillLoader::load`** to call `parse_content(&content)`, then:
   - Map any returned `SkillError::ParseError` to replace its sentinel `path` with the real file path.
   - Call `validate(&manifest, &*self.tool_checker)?` on the result.
   - This keeps `load` as the filesystem-aware, validation-aware wrapper.

3. **No visibility changes** to `frontmatter` module or `SkillFrontmatter`. The new public function is the intended public API surface; internals stay `pub(crate)`.

4. **Tests** go in a `#[cfg(test)] mod tests` block at the bottom of `lib.rs`. Construct content strings with embedded YAML frontmatter matching the `SkillFrontmatter` fields (`name`, `version`, `description`, `model`, `tools`, `constraints`, `output`).

## Dependencies

- None. This task uses only existing internal modules and types (`frontmatter`, `SkillFrontmatter`, `SkillManifest`, `SkillError`).
- **Blocks**: "Implement `ValidateSkillTool` struct and handler" (which needs to parse skill content from a string).

## Risks & Edge Cases

- **Error path sentinel**: Using `PathBuf::from("<content>")` is a cosmetic choice. If `SkillError::ParseError` is ever displayed to end-users, the `<content>` placeholder must be understandable. This matches the existing `<unknown>` convention used by `frontmatter.rs`.
- **Refactoring `load`**: Replacing inline logic in `load` with a call to `parse_content` must preserve the existing behavior of re-mapping the `path` field in `ParseError` variants to the actual file path. A test confirming `load` still produces path-aware errors would be ideal but is covered by existing integration/e2e tests.
- **No validation in `parse_content`**: Callers must remember to call `validate` separately if they need it. This is an intentional design choice documented in the function's doc comment.

## Verification

1. `cargo check -p skill-loader` compiles without errors.
2. `cargo test -p skill-loader` passes all existing and new unit tests.
3. `cargo clippy -p skill-loader` reports no warnings.
4. Confirm `parse_content` is visible from an external crate perspective by verifying it appears in `cargo doc -p skill-loader` output.
5. Confirm `SkillLoader::load` still passes its existing tests (no behavioral regression from the refactor).
