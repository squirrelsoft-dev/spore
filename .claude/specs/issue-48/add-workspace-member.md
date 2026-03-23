# Spec: Add `"tools/docker-build"` to workspace `Cargo.toml`

> From: .claude/tasks/issue-48.md

## Objective
Register the new `tools/docker-build` crate as a workspace member so that `cargo build`, `cargo test`, and other workspace-wide commands include it. This is a prerequisite for the verification suite to pass once the crate exists.

## Current State
The root `Cargo.toml` defines a workspace with 11 members. The last entry in the `members` list is `"tools/cargo-build"` (line 24). The `tools/docker-build` directory is not yet referenced.

```toml
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/mcp-tool-harness",
    "crates/orchestrator",
    "crates/mcp-test-utils",
    "tools/echo-tool",
    "tools/read-file",
    "tools/write-file",
    "tools/validate-skill",
    "tools/cargo-build",
]
```

## Requirements
- The string `"tools/docker-build"` must appear in the `members` array of the `[workspace]` section.
- It must be placed immediately after `"tools/cargo-build"` to maintain alphabetical/logical ordering of tool entries.
- The trailing comma style must be preserved (each entry on its own line, trailing comma after the last entry).
- No other lines in `Cargo.toml` should be modified.

## Implementation Details
- **File to modify:** `Cargo.toml` (root workspace manifest)
- **Change:** Add the line `    "tools/docker-build",` after the `    "tools/cargo-build",` line inside the `members` list.

## Dependencies
- Blocked by: None
- Blocking: "Run verification suite"

## Risks & Edge Cases
- If the `tools/docker-build` directory or its own `Cargo.toml` does not exist yet when this change is applied, `cargo` commands will fail with a missing-manifest error. Ensure the crate scaffold is created before (or in the same PR as) this workspace registration.
- Duplicate entry: confirm `"tools/docker-build"` does not already appear in the list before adding.

## Verification
- `grep '"tools/docker-build"' Cargo.toml` returns exactly one match.
- The `members` list is syntactically valid TOML (no missing commas, no duplicate entries).
- Once the `tools/docker-build` crate exists on disk, `cargo metadata --no-deps` includes `docker-build` in its package list.
