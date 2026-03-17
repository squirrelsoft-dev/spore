# Spec: Add dependencies to skill-loader Cargo.toml

> From: .claude/tasks/issue-5.md

## Objective

Add the required dependencies and dev-dependencies to the `skill-loader` crate so that subsequent tasks (frontmatter parsing, YAML deserialization, async file I/O, and test fixtures) have the libraries they need. This is a prerequisite for all implementation work in Group 2 and beyond.

## Current State

`crates/skill-loader/Cargo.toml` is a minimal stub:

```toml
[package]
name = "skill-loader"
version = "0.1.0"
edition = "2024"

[dependencies]
```

There are no dependencies or dev-dependencies defined. The workspace root (`Cargo.toml`) already lists `skill-loader` as a member. Sibling crates `agent-sdk` and `tool-registry` exist at `../agent-sdk` and `../tool-registry` respectively and are also workspace members.

Version references from sibling crates:
- `agent-sdk` uses `serde = { version = "1", features = ["derive"] }` and `serde_yaml = "0.9"` (dev-dep), `tokio = { version = "1", features = ["macros", "rt"] }` (dev-dep).
- `tool-registry` has no dependencies yet.

## Requirements

- Add the following entries under `[dependencies]`:
  - `serde = { version = "1", features = ["derive"] }` -- needed for `Deserialize` derive on `SkillFrontmatter`
  - `serde_yaml = "0.9"` -- needed to deserialize YAML frontmatter; version matches `agent-sdk`'s dev-dep
  - `agent-sdk = { path = "../agent-sdk" }` -- needed for `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema` types
  - `tool-registry = { path = "../tool-registry" }` -- needed for `ToolRegistry` used in `SkillLoader` struct
  - `tokio = { version = "1", features = ["fs"] }` -- needed for `tokio::fs::read_to_string` in async file reads
- Add the following entries under `[dev-dependencies]`:
  - `tokio = { version = "1", features = ["macros", "rt"] }` -- needed for `#[tokio::test]` in integration tests
  - `tempfile = "3"` -- needed for creating temporary fixture directories/files in tests
- The `[package]` section (`name`, `version`, `edition`) must remain unchanged.
- No other dependencies should be added.

## Implementation Details

- **File to modify:** `crates/skill-loader/Cargo.toml`
- The only change is appending dependency lines. No Rust source files are created or modified in this task.
- `tokio` appears in both `[dependencies]` (with `fs` feature) and `[dev-dependencies]` (with `macros`, `rt` features). Cargo merges features across these sections when running tests, so this is the correct pattern -- production code gets only `fs`, while test code additionally gets `macros` and `rt`.

The resulting file should be:

```toml
[package]
name = "skill-loader"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
serde_yaml = "0.9"
agent-sdk = { path = "../agent-sdk" }
tool-registry = { path = "../tool-registry" }
tokio = { version = "1", features = ["fs"] }

[dev-dependencies]
tokio = { version = "1", features = ["macros", "rt"] }
tempfile = "3"
```

## Dependencies

- Blocked by: Nothing (Group 1 task, no predecessors)
- Blocking: All tasks in Group 2 ("Define SkillFrontmatter struct", "Implement frontmatter extraction function") and transitively all tasks in Groups 3 and 4

## Risks & Edge Cases

- **Version drift:** `serde_yaml = "0.9"` must match the version used by `agent-sdk` to avoid pulling two incompatible versions. Currently both use `0.9`.
- **Path correctness:** The relative paths `../agent-sdk` and `../tool-registry` assume the standard `crates/<name>` layout. This is confirmed by the workspace root `Cargo.toml`.
- **Edition 2024:** The crate uses Rust edition 2024. All specified dependency versions are compatible with this edition.
- **tokio feature split:** Having `tokio` in both `[dependencies]` and `[dev-dependencies]` with different feature sets is intentional and idiomatic. Cargo unions the feature sets when building tests. If a future maintainer consolidates them into `[dependencies]` only, they must include all three features (`fs`, `macros`, `rt`), which would unnecessarily bloat the production dependency.

## Verification

- `cargo check -p skill-loader` succeeds with no errors (confirms dependency resolution and path references are valid).
- `cargo build -p skill-loader` succeeds.
- The `[package]` section is unchanged (name = "skill-loader", version = "0.1.0", edition = "2024").
- All five dependencies and two dev-dependencies are present with the exact versions and features specified above.
