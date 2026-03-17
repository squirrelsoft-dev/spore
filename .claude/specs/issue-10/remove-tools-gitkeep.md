# Spec: Remove `tools/.gitkeep`

> From: .claude/tasks/issue-10.md

## Objective
Remove the `tools/.gitkeep` placeholder file since it will no longer be needed once the `echo-tool/` directory has been added to `tools/`. The `.gitkeep` convention exists solely to track otherwise-empty directories in Git; once the directory has real content, the placeholder should be cleaned up.

## Current State
- `tools/.gitkeep` is a zero-byte placeholder file.
- `tools/` currently contains only `.gitkeep` (the `echo-tool/` subdirectory does not exist yet).

## Requirements
- `tools/.gitkeep` must be deleted from the repository.
- The `tools/` directory must still exist after deletion (guaranteed by the presence of `echo-tool/`).
- No other files or directories are affected.

## Implementation Details
- **File to delete:** `tools/.gitkeep`
  - Remove this file entirely. No other changes are needed.

## Dependencies
- Blocked by: "Implement echo tool server" (the `echo-tool/` crate must exist in `tools/` before `.gitkeep` is removed, otherwise Git will stop tracking the empty directory)
- Blocking: None

## Risks & Edge Cases
- **Risk:** If this task runs before `echo-tool/` is added, `tools/` becomes an empty directory and Git will stop tracking it. **Mitigation:** Enforce the dependency order; only remove `.gitkeep` after confirming `tools/echo-tool/` exists.
- **Risk:** Other tooling or scripts reference `tools/.gitkeep`. **Mitigation:** This is unlikely for a `.gitkeep` file, but a quick search of the codebase should confirm no references exist.

## Verification
- Confirm `tools/.gitkeep` no longer exists: `test ! -f tools/.gitkeep`
- Confirm `tools/` directory still exists and contains `echo-tool/`: `test -d tools/echo-tool`
- Run `cargo build` to ensure no build impact.
- Run `cargo test` to ensure no test impact.
