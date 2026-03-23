# Spec: Add `"tools/read-file"` to workspace `Cargo.toml`

> From: .claude/tasks/issue-44.md

## Objective

Add `"tools/read-file"` to the `members` list in the root `Cargo.toml` workspace definition. This registers the `read-file` crate with the Cargo workspace so that `cargo build -p read-file` and `cargo test -p read-file` work correctly, matching the pattern already established by `"tools/echo-tool"`.

## Current State

The root `Cargo.toml` at `/Users/sbeardsley/Developer/squirrelsoft-dev/spore/Cargo.toml` defines a Cargo workspace with the following members:

```toml
[workspace]
resolver = "2"
members = [
    "crates/agent-sdk",
    "crates/skill-loader",
    "crates/tool-registry",
    "crates/agent-runtime",
    "crates/orchestrator",
    "tools/echo-tool",
]
```

The `tools/echo-tool` entry is the established pattern for tool crates living under the `tools/` directory. There is currently no `tools/read-file` directory in the repository — that crate must be created by a separate task before this workspace entry becomes meaningful, but adding the entry here unblocks that work and is safe to do independently.

## Requirements

- The string `"tools/read-file"` must appear in the `members` list of the `[workspace]` section in the root `Cargo.toml`.
- The entry must follow immediately after `"tools/echo-tool"` to maintain consistent ordering (tools grouped together after crates).
- No other changes to `Cargo.toml` are permitted — no new dependencies, no profile changes, no feature changes.
- After the change, `cargo check` must succeed at the workspace level (assuming `tools/read-file` exists with a valid `Cargo.toml`).

## Implementation Details

- **File to modify:** `/Users/sbeardsley/Developer/squirrelsoft-dev/spore/Cargo.toml`
- **Change:** Insert `"tools/read-file",` on a new line directly after the `"tools/echo-tool",` line within the `members` array.
- **No new files** are created by this task — only `Cargo.toml` is touched.
- The resulting `members` block should look like:

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

- Blocked by: None — this change is safe to make before the `tools/read-file` crate itself exists (Cargo will report a missing-manifest error only when the directory is absent, not on edit alone).
- Blocking: "Run verification suite" — the verification suite depends on `cargo build -p read-file` and `cargo test -p read-file` being resolvable, which requires this workspace registration.

## Risks & Edge Cases

- **Missing crate directory:** If `tools/read-file/` does not yet exist when `cargo build` or `cargo check` is run at the workspace level, Cargo will emit an error about a missing `Cargo.toml`. This is expected and harmless until the crate scaffold task is complete.
- **Ordering drift:** Future workspace members added between `echo-tool` and `read-file` would disturb alphabetical ordering. The list is not currently alphabetical, so follow insertion order (tools grouped after crates) rather than enforcing alphabetical ordering.
- **Resolver compatibility:** The workspace already uses `resolver = "2"`, which is compatible with the edition 2024 used by `echo-tool`. The new member must also declare `edition = "2024"` in its own `Cargo.toml` (out of scope for this task).

## Verification

1. Open `Cargo.toml` and confirm `"tools/read-file"` appears in the `members` list after `"tools/echo-tool"`.
2. Once `tools/read-file/` exists with a valid `Cargo.toml` (separate task), run:
   - `cargo check -p read-file` — must exit 0.
   - `cargo build -p read-file` — must exit 0.
   - `cargo test -p read-file` — must exit 0.
3. Run `cargo check` at the workspace root to confirm no other member is broken by the addition.
