# Spec: Add `tools/echo-tool` to workspace members

> From: .claude/tasks/issue-10.md

## Objective
Register the `tools/echo-tool` crate in the Cargo workspace so that standard workspace commands (`cargo build -p echo-tool`, `cargo test -p echo-tool`, `cargo clippy -p echo-tool`) discover and operate on it. This is a prerequisite for all subsequent echo-tool implementation and testing tasks.

## Current State
The root `Cargo.toml` defines a workspace with `resolver = "2"` and five members:

```toml
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/orchestrator",
]
```

All existing members live under `crates/`. The `tools/` directory exists but is empty (contains only `.gitkeep`). No `tools/echo-tool` path is referenced anywhere in the workspace yet.

## Requirements
- The string `"tools/echo-tool"` must appear in the `members` array of `[workspace]` in the root `Cargo.toml`.
- The entry must be syntactically valid TOML and maintain the existing formatting style (one entry per line, trailing comma, 4-space indentation).
- After the change, `cargo metadata` must list `tools/echo-tool` as a workspace member path (this will only fully resolve once the companion task "Create `tools/echo-tool/` crate with Cargo.toml" is also complete).

## Implementation Details
- **File to modify:** `/workspaces/spore/Cargo.toml`
- **Change:** Append `"tools/echo-tool",` as a new line inside the `members` array, after the last existing entry (`"crates/orchestrator"`).
- No new files are created by this task.
- No new functions, types, or interfaces are involved.

The resulting `members` array should look like:

```toml
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/orchestrator",
    "tools/echo-tool",
]
```

## Dependencies
- Blocked by: None
- Blocking: "Implement echo tool server", "Write unit tests", "Write integration test"

## Risks & Edge Cases
- **Missing crate directory:** If `tools/echo-tool/Cargo.toml` does not yet exist, `cargo check` at the workspace level will fail with a "can't find" error. This is expected and resolved once the companion scaffolding task ("Create `tools/echo-tool/` crate with Cargo.toml") is completed. Both Group 1 tasks should land together before running workspace-wide commands.
- **Path ordering:** Placing the new entry at the end of the array (after all `crates/` entries) keeps the logical grouping clean. If additional tools are added later, they should follow the same `tools/` prefix pattern.

## Verification
- Open `/workspaces/spore/Cargo.toml` and confirm `"tools/echo-tool"` is present in the `members` array.
- After the companion scaffolding task is also complete, run `cargo metadata --no-deps --format-version 1 | grep echo-tool` and confirm the crate appears in the workspace member list.
- Run `cargo check -p echo-tool` (requires the companion `Cargo.toml` to exist) and confirm it resolves the package without "not a member" errors.
