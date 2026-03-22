# Spec: Add `tools/write-file` to workspace members

> From: .claude/tasks/issue-45.md

## Objective

Add `"tools/write-file"` to the `members` list in the root `Cargo.toml` workspace section so that Cargo recognizes the new crate as part of the workspace and includes it in builds, tests, and other workspace-wide commands.

## Current State

The root `Cargo.toml` defines a `[workspace]` with the following members:

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

`tools/echo-tool` is the only entry under the `tools/` directory. There is no `tools/write-file` entry yet.

## Requirements

1. Add `"tools/write-file"` as a new entry in the `members` array, placed immediately after `"tools/echo-tool"`.
2. Preserve the existing formatting (four-space indentation, trailing comma on every entry).
3. Do not modify any other section of `Cargo.toml`.

## Implementation Details

Insert a single line into `Cargo.toml`:

```toml
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/orchestrator",
    "tools/echo-tool",
    "tools/write-file",
]
```

No other files are changed in this task.

## Dependencies

- **Blocked by:** "Create `tools/write-file/Cargo.toml`" -- the directory and crate manifest must exist before `cargo` can resolve the workspace member.
- **Blocking:** "Implement `WriteFileTool` struct and handler", "Create `main.rs` entrypoint" -- both require the crate to be a recognized workspace member.

## Risks & Edge Cases

- If `tools/write-file/Cargo.toml` does not yet exist on disk when this change lands, `cargo build` (and all other cargo commands) will fail with a "failed to read manifest" error. Ensure the blocked-by task is completed first or land both changes together.
- Duplicate entries in `members` would cause a cargo warning. Confirm the entry does not already exist before adding.

## Verification

1. Run `cargo check` from the workspace root and confirm it exits successfully (requires the `tools/write-file` crate to exist on disk).
2. Run `cargo metadata --no-deps --format-version 1 | jq '.workspace_members'` and confirm `write-file` appears in the list.
3. Inspect the diff to confirm only the `members` array was modified and formatting is consistent with existing entries.
