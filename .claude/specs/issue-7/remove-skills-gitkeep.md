# Spec: Remove `skills/.gitkeep`

> From: .claude/tasks/issue-7.md

## Objective

Remove the `skills/.gitkeep` placeholder file now that the directory contains real skill files (`cogs-analyst.md`, `echo.md`, `skill-writer.md`). The `.gitkeep` convention exists solely to force Git to track an otherwise-empty directory; once the directory has actual content, the placeholder is unnecessary. The `tools/.gitkeep` file must remain untouched because `tools/` is still empty.

## Current State

- **`skills/.gitkeep`** exists at `/workspaces/spore/skills/.gitkeep`. It is a 0-byte file tracked by Git since commit `4396416`.
- **`tools/.gitkeep`** exists at `/workspaces/spore/tools/.gitkeep`. It is a 0-byte file, also tracked. The `tools/` directory has no other files and will remain empty after issue-7.

## Requirements

- Delete `skills/.gitkeep` from the working tree and from Git tracking.
- Do NOT delete, modify, or touch `tools/.gitkeep`.
- Only perform after all three Group 1 skill files have been committed.
- After removal, `skills/` must still contain at least the three skill files.

## Implementation Details

- Run `git rm skills/.gitkeep` to delete the file and stage the deletion.
- No code changes, no configuration changes, no dependency changes.

## Dependencies

- **Blocked by:** All Group 1 tasks (skill files must exist first, otherwise `skills/` becomes empty and untracked)
- **Blocking:** None

## Risks & Edge Cases

1. **Premature removal:** If removed before skill files are committed, Git won't track `skills/`. Mitigation: enforce Group 1 dependency.
2. **Accidental `tools/.gitkeep` removal:** Never use a wildcard like `rm */.gitkeep`. Use the explicit path `skills/.gitkeep` only.
3. **Merge conflicts:** Low-risk given the file is 0 bytes and the operation is a simple deletion.

## Verification

- `test ! -f skills/.gitkeep` exits 0
- `git ls-files skills/.gitkeep` produces no output
- `test -f tools/.gitkeep` exits 0
- `ls skills/` lists `cogs-analyst.md`, `echo.md`, `skill-writer.md`
- `cargo test` passes (no regressions)
