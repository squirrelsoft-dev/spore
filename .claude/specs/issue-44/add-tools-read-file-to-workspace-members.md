# Spec: Add `tools/read-file` to workspace members

> From: .claude/tasks/issue-44.md

## Objective
Register the `tools/read-file` crate in the Cargo workspace so that standard workspace commands (`cargo build -p read-file`, `cargo test -p read-file`, `cargo clippy -p read-file`) discover and operate on it. This is a prerequisite for all subsequent read-file implementation and testing tasks.

## Current State
The root `Cargo.toml` defines a workspace with `resolver = "2"` and six members:

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

The `tools/` directory already contains `echo-tool`. No `tools/read-file` path is referenced anywhere in the workspace yet.

## Requirements
- The string `"tools/read-file"` must appear in the `members` array of `[workspace]` in the root `Cargo.toml`.
- The entry must be placed immediately after `"tools/echo-tool"` to maintain logical grouping of tool crates.
- The entry must be syntactically valid TOML and maintain the existing formatting style (one entry per line, trailing comma, 4-space indentation).
- After the change, `cargo metadata` must list `tools/read-file` as a workspace member path (this will only fully resolve once the companion task "Create tools/read-file/Cargo.toml" is also complete).

## Implementation Details
- **File to modify:** `/workspaces/spore/Cargo.toml`
- **Change:** Append `"tools/read-file",` as a new line inside the `members` array, after the existing `"tools/echo-tool"` entry.
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
]
```

## Dependencies
- Blocked by: "Create tools/read-file/Cargo.toml"
- Blocking: "Implement ReadFileTool struct and handler", "Create main.rs entrypoint"

## Risks & Edge Cases
- **Missing crate directory:** If `tools/read-file/Cargo.toml` does not yet exist, `cargo check` at the workspace level will fail with a "can't find" error. This is expected and resolved once the companion scaffolding task ("Create tools/read-file/Cargo.toml") is completed. Both tasks should land together before running workspace-wide commands.
- **Path ordering:** Placing the new entry after `tools/echo-tool` keeps the logical grouping clean. All `tools/` entries follow the `crates/` entries. If additional tools are added later, they should follow the same `tools/` prefix pattern.

## Verification
- Open `/workspaces/spore/Cargo.toml` and confirm `"tools/read-file"` is present in the `members` array, positioned after `"tools/echo-tool"`.
- After the companion scaffolding task is also complete, run `cargo metadata --no-deps --format-version 1 | grep read-file` and confirm the crate appears in the workspace member list.
- Run `cargo check -p read-file` (requires the companion `Cargo.toml` to exist) and confirm it resolves the package without "not a member" errors.
