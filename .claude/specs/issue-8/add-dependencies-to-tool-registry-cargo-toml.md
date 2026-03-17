# Spec: Add dependencies to tool-registry Cargo.toml

> From: .claude/tasks/issue-8.md

## Objective

Add the required dependency declarations to `crates/tool-registry/Cargo.toml` so that subsequent tasks (Group 2 and beyond) can use `serde`, `schemars`, `serde_json`, and `agent-sdk` types and derives when implementing `ToolEntry`, `RegistryError`, and `ToolRegistry`.

## Current State

`crates/tool-registry/Cargo.toml` currently has an empty `[dependencies]` section:

```toml
[package]
name = "tool-registry"
version = "0.1.0"
edition = "2024"

[dependencies]
```

The crate's `src/lib.rs` contains only a placeholder unit struct (`pub struct ToolRegistry;`). No dependencies are used yet.

The sibling crate `crates/agent-sdk/Cargo.toml` already uses the same dependency versions and feature sets that this task adds, establishing the workspace conventions:

- `serde = { version = "1", features = ["derive"] }`
- `schemars = { version = "0.8", features = ["derive", "uuid1"] }`
- `serde_json = "1"`

All three external crates are already resolved in the workspace lockfile. `agent-sdk` is an internal path dependency at `../agent-sdk`.

## Requirements

- Add exactly four dependencies to the `[dependencies]` section of `crates/tool-registry/Cargo.toml`:
  1. `serde = { version = "1", features = ["derive"] }` -- needed for `Serialize`/`Deserialize` derives on `ToolEntry`
  2. `schemars = { version = "0.8", features = ["derive", "uuid1"] }` -- needed for `JsonSchema` derive on `ToolEntry`
  3. `serde_json = "1"` -- needed for JSON serialization in tests and future registry operations
  4. `agent-sdk = { path = "../agent-sdk" }` -- needed for importing `SkillManifest` in `resolve_for_skill`
- Do NOT add `tokio`, `async-trait`, or `rmcp` -- those are deferred to issue #9
- Do NOT add any `[dev-dependencies]` section in this task
- Do NOT change the `[package]` section (name, version, edition must remain as-is)
- The dependency versions and feature flags must exactly match the patterns in `crates/agent-sdk/Cargo.toml`
- No new third-party crates are introduced to the workspace; all external dependencies already exist in the lockfile

## Implementation Details

- **File to modify:** `crates/tool-registry/Cargo.toml`
- **Changes:** Add four lines under the existing empty `[dependencies]` section
- The resulting file should be:

```toml
[package]
name = "tool-registry"
version = "0.1.0"
edition = "2024"

[dependencies]
serde = { version = "1", features = ["derive"] }
schemars = { version = "0.8", features = ["derive", "uuid1"] }
serde_json = "1"
agent-sdk = { path = "../agent-sdk" }
```

- No code changes to `src/lib.rs` are required in this task. The placeholder `pub struct ToolRegistry;` remains until the "Wire up lib.rs" task in Group 3.

## Dependencies

- **Blocked by:** None (this is a Group 1 task with no prerequisites)
- **Blocking:** All tasks in Group 2, specifically "Implement `ToolRegistry` struct and methods" which needs all four dependencies to compile

## Risks & Edge Cases

- **Path dependency resolution:** The relative path `../agent-sdk` must resolve correctly from `crates/tool-registry/`. This is the standard pattern for sibling crate dependencies within this workspace and is already used by other crates (e.g., `skill-loader` depends on `tool-registry` via a similar path).
- **Edition 2024 compatibility:** The `tool-registry` crate uses `edition = "2024"`. All specified dependency versions (`serde 1`, `schemars 0.8`, `serde_json 1`) are compatible with Rust edition 2024.
- **Unused dependency warnings:** After adding these dependencies, `cargo check` on the `tool-registry` crate alone may emit "unused dependency" warnings since `src/lib.rs` does not yet import them. This is expected and acceptable because the dependencies will be consumed by the immediately subsequent Group 1/2 tasks. If clippy is run on the workspace, this should not block.

## Verification

1. `cargo check -p tool-registry` compiles without errors (unused dependency warnings are acceptable at this stage)
2. `cargo check` (full workspace) compiles without errors
3. `cargo test` (full workspace) passes -- existing tests in other crates are unaffected
4. The `[dependencies]` section contains exactly the four specified entries with correct versions and features
5. No `[dev-dependencies]` section was added
6. The `[package]` section is unchanged
