# Spec: Add `"tools/list-agents"` to workspace `Cargo.toml`
> From: .claude/tasks/issue-51.md

## Objective
Add `"tools/list-agents"` as a new member entry in the root workspace `Cargo.toml` so the `list-agents` tool crate is included in the Cargo workspace and participates in builds, tests, and other workspace-wide commands.

## Current State
The root `Cargo.toml` defines a `[workspace]` with `resolver = "2"` and the following members:
- `crates/agent-sdk`
- `crates/skill-loader`
- `crates/tool-registry`
- `crates/agent-runtime`
- `crates/mcp-tool-harness`
- `crates/orchestrator`
- `crates/mcp-test-utils`
- `tools/echo-tool`
- `tools/read-file`
- `tools/register-agent`
- `tools/write-file`
- `tools/validate-skill`
- `tools/cargo-build`
- `tools/docker-push`
- `tools/docker-build`

There is no `tools/list-agents` directory on disk yet. This spec only covers the `Cargo.toml` change; the actual crate scaffolding is handled by a separate task.

## Requirements
1. Append `"tools/list-agents"` to the `members` array in the `[workspace]` section of the root `Cargo.toml`.
2. Place it in alphabetical order among the `tools/*` entries (after `tools/echo-tool`, before `tools/read-file`).
3. Do not modify any other part of the file.

## Implementation Details
- File to edit: `/workspaces/spore/Cargo.toml`
- Add the line `    "tools/list-agents",` between `"tools/echo-tool"` and `"tools/read-file"` in the `members` list.
- The resulting members list for tools should read:
  ```
  "tools/echo-tool",
  "tools/list-agents",
  "tools/read-file",
  "tools/register-agent",
  ...
  ```

## Dependencies
- Blocked by: none (Group 1)
- Blocking: "Run verification suite"

## Risks & Edge Cases
- Adding the member before the `tools/list-agents` crate directory exists will cause `cargo` commands to fail with a missing-manifest error. Ensure the crate scaffolding task runs before (or alongside) this change so the workspace resolves cleanly.
- If additional tool crates are added concurrently, merge conflicts in the `members` list are possible. Resolve by keeping entries in alphabetical order within each prefix group (`crates/`, `tools/`).

## Verification
1. Run `cargo check` (or `cargo build`) and confirm no workspace resolution errors related to the new member.
2. Run `cargo test` and confirm the full suite passes.
3. Visually inspect `Cargo.toml` to confirm `"tools/list-agents"` appears exactly once and in alphabetical order among `tools/*` entries.
