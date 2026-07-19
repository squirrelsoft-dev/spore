# Spec: Add `"tools/register-agent"` to workspace `Cargo.toml`

> From: .claude/tasks/issue-50.md

## Objective
Add the `"tools/register-agent"` crate to the workspace members list in the root `Cargo.toml` so that Cargo recognizes it as part of the workspace and includes it in builds, tests, and other workspace-wide commands.

## Current State
The root `Cargo.toml` defines a workspace with 13 members across two directories:
- `crates/` (7 members): `agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, `mcp-tool-harness`, `orchestrator`, `mcp-test-utils`
- `tools/` (6 members): `echo-tool`, `read-file`, `write-file`, `validate-skill`, `cargo-build`, `docker-push`, `docker-build`

The `"tools/register-agent"` entry is not yet present.

## Requirements
1. Add `"tools/register-agent"` to the `members` array in `[workspace]` of the root `Cargo.toml`.
2. Place it immediately after the `"tools/docker-push"` entry (line 25), before `"tools/docker-build"`.
3. Maintain the existing formatting: 4-space indentation, quoted string, trailing comma.

## Implementation Details
- **File**: `Cargo.toml` (root, `/workspaces/spore/Cargo.toml`)
- **Change**: Insert the line `    "tools/register-agent",` after line 25 (`    "tools/docker-push",`).
- The resulting members list in the `tools/` section should read:
  ```
      "tools/echo-tool",
      "tools/read-file",
      "tools/write-file",
      "tools/validate-skill",
      "tools/cargo-build",
      "tools/docker-push",
      "tools/register-agent",
      "tools/docker-build",
  ```

## Dependencies
- The `tools/register-agent` directory and its own `Cargo.toml` must exist before `cargo build` or `cargo check` will succeed. This spec only covers adding the workspace member entry; creation of the crate itself is handled by a separate task.

## Risks & Edge Cases
- **Missing crate directory**: If `tools/register-agent/Cargo.toml` does not yet exist when this change lands, any workspace-wide Cargo command (`cargo build`, `cargo check`, `cargo test`) will fail with a "can't find" error. Coordinate with the crate-creation task to land both together or ensure the crate exists first.
- **Duplicate entry**: Verify the entry does not already exist before adding it to avoid a Cargo workspace error.

## Verification
1. Run `cargo check` (or `cargo build`) -- confirm no errors related to workspace member resolution (assuming the `tools/register-agent` crate exists).
2. Visually inspect the root `Cargo.toml` to confirm correct placement and formatting.
3. Run `cargo test` to ensure no regressions across the workspace.
