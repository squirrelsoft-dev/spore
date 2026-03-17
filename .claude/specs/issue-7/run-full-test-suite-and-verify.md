# Spec: Run full test suite and verify

> From: .claude/tasks/issue-7.md

## Objective

Gate the issue-7 deliverable by confirming that every artifact produced by the prior tasks results in a clean build, zero test failures, and zero clippy warnings. This is a pure verification step with no code changes.

## Current State

The workspace has five crates (`agent-sdk`, `skill-loader`, `tool-registry`, `agent-runtime`, `orchestrator`).

**Baseline test counts on `main` (before issue-7):**
- 11 test binaries, 74 total tests
- `cargo clippy` exits cleanly

**Artifacts expected from prior tasks:**
1. `skills/cogs-analyst.md` — full-featured finance domain skill
2. `skills/echo.md` — minimal edge-case skill
3. `skills/skill-writer.md` — bootstrap seed agent stub
4. `crates/skill-loader/tests/example_skills_test.rs` — new integration test
5. Updated `README.md`
6. `skills/.gitkeep` removed

## Requirements

- `cargo test` exits with status 0, all tests pass, including the new `example_skills_test`
- `cargo clippy` exits with status 0 with no warnings
- No existing tests regress (74 baseline tests still pass)
- No compilation errors across the workspace

## Implementation Details

No files created or modified. Run commands and inspect output only.

### Step 1: Run full test suite

```
cargo test
```

Check:
- Every test binary shows `ok` in summary
- Zero failures
- `example_skills_test` binary appears and passes
- Total test count = 74 + new integration tests

### Step 2: Run clippy

```
cargo clippy
```

Check:
- Exit status 0
- No warnings or errors

### Step 3: Verify file system state

- `skills/cogs-analyst.md`, `skills/echo.md`, `skills/skill-writer.md` exist and are non-empty
- `skills/.gitkeep` does NOT exist
- `crates/skill-loader/tests/example_skills_test.rs` exists

## Dependencies

- **Blocked by:** All previous tasks in issue-7
- **Blocking:** None (terminal task)

## Risks & Edge Cases

1. **Integration test path resolution:** Test must locate `skills/` relative to workspace root via `env!("CARGO_MANIFEST_DIR")/../../skills/`. Incorrect path causes `SkillError::IoError`.
2. **`escalate_to` handling:** Echo and skill-writer must omit `escalate_to` (defaulting to `None`), not set it to empty string (which fails validation).
3. **Clippy version sensitivity:** Different Rust toolchains may flag different lints. New code must pass the installed version.

## Verification

- `cargo test` exits 0 with 0 failures
- `cargo clippy` exits 0 with no warnings
- `example_skills_test` binary appears in test output with all tests passing
- All 74 baseline tests still pass
- File system state confirmed as expected
