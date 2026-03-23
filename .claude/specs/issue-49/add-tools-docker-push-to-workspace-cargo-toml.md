# Spec: Add `"tools/docker-push"` to workspace `Cargo.toml`

> From: .claude/tasks/issue-49.md

## Objective

Add `"tools/docker-push"` to the `members` list in the root `Cargo.toml` workspace definition. This registers the `docker-push` crate with the Cargo workspace so that `cargo build -p docker-push`, `cargo test -p docker-push`, and `cargo clippy -p docker-push` work correctly, following the pattern established by the existing tool crates (`echo-tool`, `cargo-build`, etc.).

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

The `tools/cargo-build` entry is the last member in the list. There is currently no `tools/docker-push` directory in the repository — that crate must be created by a separate task ("Create `tools/docker-push/Cargo.toml`") before this workspace entry becomes meaningful, but adding the entry here is safe to do independently and unblocks the verification suite.

## Requirements

- The string `"tools/docker-push"` must appear in the `members` list of the `[workspace]` section in the root `Cargo.toml`.
- The entry must be placed immediately after `"tools/cargo-build"` to maintain the existing ordering convention (crates grouped first, then tools in insertion order).
- No other changes to `Cargo.toml` are permitted — no new dependencies, no profile changes, no feature changes.
- After the change, `cargo check` must succeed at the workspace level (assuming `tools/docker-push` exists with a valid `Cargo.toml`).

## Implementation Details

- **File to modify:** `/Users/sbeardsley/Developer/squirrelsoft-dev/spore/Cargo.toml`
- **Change:** Insert `"tools/docker-push",` on a new line directly after the `"tools/cargo-build",` line within the `members` array.
- **No new files** are created by this task — only `Cargo.toml` is touched.
- The resulting `members` block should look like:

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
    "tools/docker-push",
]
```

## Dependencies

- Blocked by: None — this change is safe to make before the `tools/docker-push` crate itself exists (Cargo will report a missing-manifest error only when a workspace-level build is attempted while the directory is absent, not on edit alone).
- Blocking: "Run verification suite" — the verification suite depends on `cargo build -p docker-push` and `cargo test -p docker-push` being resolvable, which requires this workspace registration.

## Risks & Edge Cases

- **Missing crate directory:** If `tools/docker-push/` does not yet exist when `cargo build` or `cargo check` is run at the workspace level, Cargo will emit an error about a missing `Cargo.toml`. This is expected and harmless until the crate scaffold task ("Create `tools/docker-push/Cargo.toml`") is complete.
- **Ordering drift:** The members list is not alphabetically sorted; it follows insertion order with crates grouped before tools. The new entry should go at the end of the tools group (after `cargo-build`) to stay consistent.
- **Resolver compatibility:** The workspace already uses `resolver = "2"`, which is compatible with the edition 2024 used by existing tool crates. The new `docker-push` member must also declare `edition = "2024"` in its own `Cargo.toml` (out of scope for this task).

## Verification

1. Open `Cargo.toml` and confirm `"tools/docker-push"` appears in the `members` list immediately after `"tools/cargo-build"`.
2. Confirm no other lines in `Cargo.toml` were changed (no dependency additions, no profile changes, no feature changes).
3. Once `tools/docker-push/` exists with a valid `Cargo.toml` (separate task), run:
   - `cargo check -p docker-push` — must exit 0.
   - `cargo build -p docker-push` — must exit 0.
   - `cargo test -p docker-push` — must exit 0.
   - `cargo clippy -p docker-push` — must exit 0.
4. Run `cargo check` at the workspace root to confirm no other member is broken by the addition.
