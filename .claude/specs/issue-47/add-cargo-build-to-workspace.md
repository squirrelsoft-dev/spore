# Spec: Add `"tools/cargo-build"` to workspace Cargo.toml

> From: .claude/tasks/issue-47.md

## Objective
Register the new `tools/cargo-build` crate as a workspace member so that it participates in `cargo build`, `cargo test`, and `cargo clippy` runs across the workspace.

## Current State
The root `Cargo.toml` contains a `[workspace]` section with `members` listing seven crates and four tools:
```
"tools/echo-tool",
"tools/read-file",
"tools/write-file",
"tools/validate-skill",
```
`tools/cargo-build` is not yet listed.

## Requirements
1. Add `"tools/cargo-build"` to the `members` array in the root `Cargo.toml`.
2. Place it after `"tools/validate-skill"` to maintain the current append-to-end ordering pattern used by the existing tool entries.
3. No other changes to `Cargo.toml`.

## Implementation Details
- Insert `"tools/cargo-build",` as a new line after the `"tools/validate-skill",` entry (line 23) and before the closing `]` (line 24).
- Preserve the existing 4-space indentation and trailing comma style.

## Dependencies
- Blocked by: None (the `tools/cargo-build` crate directory and its own `Cargo.toml` must exist before this change will compile, but that is handled by a sibling task)
- Blocking: "Run verification suite"

## Risks & Edge Cases
- If the `tools/cargo-build` directory or its `Cargo.toml` does not yet exist when this change is applied, `cargo` commands will fail with a missing-member error. Ensure the crate scaffolding task runs first or concurrently.
- A typo in the member path (e.g., `tools/cargo_build` vs `tools/cargo-build`) will cause the same failure; use the hyphenated form matching Cargo convention.

## Verification
1. `cargo check` — workspace resolves without errors.
2. `cargo build` — all members, including `tools/cargo-build`, compile.
3. `cargo test` — all workspace tests pass.
4. Confirm `cargo metadata --no-deps` includes `cargo-build` in its package list.
