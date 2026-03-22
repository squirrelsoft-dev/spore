# Spec: Add `tools/validate-skill` to workspace members

> From: .claude/tasks/issue-46.md

## Objective
Register the `tools/validate-skill` crate in the Cargo workspace so that standard workspace commands (`cargo build -p validate-skill`, `cargo test -p validate-skill`, `cargo clippy -p validate-skill`) discover and operate on it.

## Current State
The root `Cargo.toml` defines a workspace with `resolver = "2"` and nine members:

```toml
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/orchestrator",
    "tools/echo-tool",
    "tools/read-file",
    "tools/write-file",
]
```

The `tools/` directory already contains `echo-tool`, `read-file`, and `write-file`. No `tools/validate-skill` path is referenced anywhere in the workspace yet.

## Requirements
- The string `"tools/validate-skill"` must appear in the `members` array of `[workspace]` in the root `Cargo.toml`.
- The entry must be placed immediately after `"tools/write-file"` to maintain logical grouping of tool crates.
- The entry must be syntactically valid TOML and maintain the existing formatting style (one entry per line, trailing comma, 4-space indentation).
- After the change, `cargo metadata` must list `tools/validate-skill` as a workspace member path (this will only fully resolve once the companion `tools/validate-skill/Cargo.toml` exists).

## Implementation Details
- **File to modify:** `/workspaces/spore/Cargo.toml`
- **Change:** Append `"tools/validate-skill",` as a new line inside the `members` array, after the existing `"tools/write-file"` entry.
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
    "tools/read-file",
    "tools/write-file",
    "tools/validate-skill",
]
```

## Dependencies
- Blocking: "Run verification suite"

## Risks & Edge Cases
- **Missing crate directory:** If `tools/validate-skill/Cargo.toml` does not yet exist, `cargo check` at the workspace level will fail with a "can't find" error. This is expected and resolved once the companion scaffolding task is completed. Both tasks should land together before running workspace-wide commands.
- **Path ordering:** Placing the new entry after `tools/write-file` keeps the logical grouping clean. All `tools/` entries follow the `crates/` entries. If additional tools are added later, they should follow the same `tools/` prefix pattern.

## Verification
- Open `/workspaces/spore/Cargo.toml` and confirm `"tools/validate-skill"` is present in the `members` array, positioned after `"tools/write-file"`.
- After the companion scaffolding task is also complete, run `cargo metadata --no-deps --format-version 1 | grep validate-skill` and confirm the crate appears in the workspace member list.
- Run `cargo check -p validate-skill` (requires the companion `Cargo.toml` to exist) and confirm it resolves the package without "not a member" errors.
