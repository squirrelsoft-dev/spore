# Spec: Define SkillError enum

> From: .claude/tasks/issue-5.md

## Objective

Create a `SkillError` enum in the `skill-loader` crate that represents every failure mode the loader can encounter: filesystem I/O failures, frontmatter parsing failures, and semantic validation failures. This type will be the `Err` variant in every `Result` returned by the skill-loader's public API, giving callers structured, matchable error information. The enum follows the manual `Display + Error` pattern established by `AgentError` in the `agent-sdk` crate, without relying on `thiserror`.

## Current State

- The `skill-loader` crate (`crates/skill-loader/`) is a skeleton with a placeholder `add()` function and a trivial test in `src/lib.rs`. No error types, no real logic.
- `Cargo.toml` declares edition 2024 and has no dependencies yet (a sibling task will add `serde`, `serde_yaml`, `tokio`, etc.).
- The reference error pattern lives in `crates/agent-sdk/src/agent_error.rs`: a `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]` enum with a manual `impl fmt::Display` (one `write!` per variant, each under 3 lines) and an empty `impl std::error::Error for AgentError {}`.
- `crates/agent-sdk/src/lib.rs` shows the module/re-export convention: a private `mod` declaration plus a `pub use` re-export for each type.
- `SkillManifest` (in `agent-sdk`) has fields `name`, `version`, `description`, `model`, `preamble`, `tools`, `constraints`, and `output`. The `preamble` field is populated from the markdown body after the YAML frontmatter, which means parse errors surface before a `SkillManifest` is ever constructed.

## Requirements

1. **File location:** `crates/skill-loader/src/error.rs` (new file).

2. **Enum definition:** A public enum named `SkillError` with exactly three variants:
   - `IoError { path: PathBuf, source: String }` -- The loader could not read the skill file. `path` is the file it tried to open, `source` is the underlying OS error message (obtained via `std::io::Error::to_string()`).
   - `ParseError { path: PathBuf, source: String }` -- The file was read but its content is structurally invalid. Covers two sub-cases: (a) the file does not contain valid `---` frontmatter delimiters, and (b) the YAML between the delimiters fails `serde_yaml` deserialization. `path` is the file, `source` describes what went wrong.
   - `ValidationError { skill_name: String, reasons: Vec<String> }` -- The frontmatter deserialized successfully but violates semantic rules (e.g., unknown tool names, constraint conflicts). `skill_name` is the skill's declared name, `reasons` is a non-empty list of human-readable validation failure messages. The concrete validation rules are out of scope (issue #6); this variant exists now so that the error type is stable before validators are added.

3. **Derive macros:** `#[derive(Debug, Clone, PartialEq)]`. Do **not** derive `Serialize`, `Deserialize`, or `JsonSchema`. `SkillError` is a crate-internal operational error, not a wire type. Note: although the existing `AgentError` in `agent-sdk` derives `Serialize` and `Deserialize`, that is specific to agent-sdk's wire protocol needs. `SkillError` does not cross process boundaries.

4. **`std::fmt::Display` implementation:** A manual `impl fmt::Display for SkillError` using `match self` with `write!(f, ...)`. Each arm must be at most 3 lines. Suggested formats:
   - `IoError` -- `"IO error reading {path}: {source}"` (where `path` uses `Path::display()`).
   - `ParseError` -- `"parse error in {path}: {source}"` (where `path` uses `Path::display()`).
   - `ValidationError` -- `"validation error for skill '{skill_name}': {reasons}"` (where `reasons` are joined with `"; "`).

   Messages should be lowercase to follow Rust error-message conventions. All field values must appear in the output.

5. **`std::error::Error` implementation:** An `impl std::error::Error for SkillError {}` block. None of the variants wrap a typed `source` error (they store stringified messages instead), so the default `source()` returning `None` is correct.

6. **No additional dependencies.** The file uses only `std::fmt`, `std::path::PathBuf`, and `std::error::Error`. No external crates are imported. This means the file compiles even before the sibling "Add dependencies" task runs.

7. **No `From` conversions in this file.** A `From<std::io::Error>` impl would require knowing the `path`, which is context the caller must supply. Conversion helpers (if any) belong in the modules that perform I/O, not in the error definition file.

8. **No `#[non_exhaustive]` attribute.** The enum is internal to the `spore` workspace, and exhaustive matching is more valuable than future-proofing at this stage.

## Implementation Details

### File: `crates/skill-loader/src/error.rs` (new)

- **Imports:** `use std::fmt;` and `use std::path::PathBuf;`. Only standard library types.
- **Enum declaration:** Three variants as specified above, with `#[derive(Debug, Clone, PartialEq)]`.
- **`Display` impl:** A `match self` block. For `IoError` and `ParseError`, use `path.display()` to render the `PathBuf`. For `ValidationError`, join the `reasons` vec with `"; "` to produce a single-line summary.
- **`Error` impl:** Empty impl block -- the default trait methods are sufficient.

### File: `crates/skill-loader/src/lib.rs` (modified -- but NOT by this task)

This task does **not** modify `lib.rs`. The downstream task "Implement SkillLoader struct and load method" will add:
```rust
mod error;
pub use error::SkillError;
```

### Integration points

- `SkillError::IoError` will be constructed by `SkillLoader::load` when `tokio::fs::read_to_string` fails. The caller converts `std::io::Error` to a `String` and pairs it with the path.
- `SkillError::ParseError` will be constructed by `extract_frontmatter` (missing delimiters) and by the YAML deserialization step in `SkillLoader::load` (invalid YAML).
- `SkillError::ValidationError` will be constructed by future validation logic (issue #6). Until then, no code in the crate produces this variant, but tests will verify it compiles and displays correctly.
- `SkillError` is the single error type for every public fallible function in the `skill-loader` crate.

## Dependencies

- **Blocked by:** Nothing. This task uses only `std` types, so it compiles independently of the "Add dependencies to skill-loader Cargo.toml" task.
- **Parallel with:** "Add dependencies to skill-loader Cargo.toml" (Group 1).
- **Blocking:** "Implement frontmatter extraction function" (Group 2), "Implement SkillLoader struct and load method" (Group 3) -- both return `Result<_, SkillError>`.

## Risks & Edge Cases

1. **`PathBuf` in `PartialEq`:** `PathBuf::eq` is byte-level on Unix and case-insensitive on Windows. Tests should use consistent paths (e.g., constructed from a single `PathBuf::from` call) to avoid platform-dependent equality surprises.

2. **`Vec<String>` in `PartialEq` for `ValidationError`:** Order matters. The `reasons` vector is compared element-by-element, so validation code and tests must agree on the ordering of reasons. This is acceptable because the validation logic (issue #6) will define a deterministic order.

3. **`Display` output stability:** Downstream test tasks will assert on `Display` output (e.g., checking that error messages contain the file path). The format strings should be treated as semi-stable. Any change requires updating the corresponding tests.

4. **No `source()` chain:** By storing `source` as `String` rather than as a boxed `dyn Error`, we lose the ability to call `.source()` on the error to walk the cause chain. This is a deliberate trade-off: it avoids lifetime and `Send + Sync` complexity, and the stringified message retains all diagnostic information. If typed error chaining is needed later, the variants can be extended without breaking the public API shape.

5. **Thread safety:** `SkillError` contains only `PathBuf`, `String`, and `Vec<String>` -- all `Send + Sync`. Safe to use across async task boundaries in `tokio` without wrapping.

6. **Large `reasons` vectors:** The `Display` impl joins all reasons with `"; "`, which could produce very long lines if validation generates many failures. This is acceptable for logging and debug output; structured consumers should match the variant and iterate `reasons` directly.

## Verification

1. After creating the file, run `cargo check -p skill-loader` to confirm the enum and its trait impls compile. This should succeed even before dependencies are added because the file uses only `std`.
2. Run `cargo clippy -p skill-loader` to confirm no lint warnings.
3. Run `cargo test -p skill-loader` to confirm existing (placeholder) tests still pass and the new file does not introduce breakage.
4. Verify that the file contains no `use serde`, `use schemars`, `use thiserror`, or other external crate imports -- only `std::fmt` and `std::path::PathBuf`.
5. Verify the enum has exactly three variants: `IoError`, `ParseError`, `ValidationError`, with the field names and types specified above.
6. Full `Display` output testing and integration testing are handled by Group 4 tasks, not this task.
