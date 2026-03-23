# Spec: Add register-agent to workspace members

> From: .claude/tasks/issue-50.md

## Objective
Add `"tools/register-agent"` to the `[workspace] members` array in the root `Cargo.toml` so that Cargo recognizes the new tool crate as part of the workspace. This is a prerequisite for all subsequent register-agent implementation tasks.

## Current State
The root `Cargo.toml` defines a workspace with 13 members across `crates/` and `tools/` directories. The current members list ends with `"tools/docker-build"`. The `tools/register-agent` directory does not yet exist but will be created by the sibling scaffolding task ("Create register-agent Cargo.toml").

## Requirements
- The string `"tools/register-agent"` must appear in the `[workspace] members` array in `/workspaces/spore/Cargo.toml`
- The entry must be added after the last existing `tools/` entry (`"tools/docker-build"`) to maintain alphabetical/logical grouping
- No other lines in `Cargo.toml` should be modified
- The trailing comma convention used by existing entries must be preserved

## Implementation Details
- **File to modify:** `Cargo.toml` (root)
- **Change:** Add `"tools/register-agent",` as a new line after `"tools/docker-build",` inside the `members` array
- The resulting members array should end with:
  ```toml
      "tools/docker-build",
      "tools/register-agent",
  ]
  ```

## Dependencies
- Blocked by: none (Group 1)
- Blocking: "Implement register_agent tool logic", "Create main.rs entry point", "Write integration tests"

## Risks & Edge Cases
- If this change lands before the `tools/register-agent/Cargo.toml` exists, `cargo` commands will fail with a missing-manifest error. Ensure the sibling task ("Create register-agent Cargo.toml") is merged together with or before this change.
- Merge conflicts are possible if other PRs add new workspace members concurrently. Resolution is straightforward: include all entries.

## Verification
- `grep -q '"tools/register-agent"' Cargo.toml` exits 0
- Once the sibling `Cargo.toml` scaffolding task is also complete, `cargo metadata --no-deps` lists `register-agent` as a workspace member
- `cargo check` passes (requires both scaffolding tasks to be complete)
