# Spec: Write unit tests

> From: .claude/tasks/issue-45.md

## Objective

Add a `#[cfg(test)] mod tests` block to `tools/write-file/src/write_file.rs` covering the core behaviors of `WriteFileTool`: file creation, parent directory creation, error handling, byte-count reporting, overwrite semantics, and unicode round-tripping.

## Current State

The file `tools/write-file/src/write_file.rs` does not yet exist. It will be created by the blocking task "Implement `WriteFileTool` struct and handler". No tests exist for the write-file tool.

## Requirements

Six test cases must be added inside a `#[cfg(test)] mod tests` block:

1. **`write_file_creates_file_with_content`** — Call the tool's handler with a path inside a temporary directory and known content. Read the file back with `std::fs::read_to_string` and assert the content matches exactly.
2. **`write_file_creates_parent_directories`** — Call the handler with a deeply nested path (e.g., `tmpdir/a/b/c/file.txt`) that does not yet exist. Assert the file is created and its content is correct.
3. **`write_file_empty_path`** — Call the handler with an empty string as the path. Assert that the result is an error with a descriptive message (not a panic or generic OS error).
4. **`write_file_returns_byte_count`** — Write content of a known length. Assert the confirmation message string contains the correct byte count.
5. **`write_file_overwrites_existing`** — Write content to a path, then write different content to the same path. Read back and assert only the second content is present.
6. **`write_file_preserves_unicode`** — Write content containing multi-byte unicode characters (emoji, accented characters, CJK). Read back and assert exact round-trip equality.

## Implementation Details

- Follow the test module pattern established in `tools/echo-tool/src/echo.rs`:
  - `use super::*;` to import the tool struct and request type.
  - A small helper function (e.g., `call_write_file`) that constructs the request struct, wraps it in `Parameters(...)`, and calls the handler method directly.
  - Each test is annotated with `#[tokio::test]` since the handler may be async.
- Use `std::env::temp_dir()` combined with a unique subdirectory per test (e.g., `temp_dir().join("write_file_tests/<test_name>")`) to isolate test state. Do **not** add the `tempfile` crate as a dependency.
- Each test must clean up its temporary files/directories in a scope guard or explicit `std::fs::remove_dir_all` at the end.
- No new dependencies should be introduced.

## Dependencies

- **Blocked by**: "Implement `WriteFileTool` struct and handler" — the struct, request type, and handler method must exist before tests can compile.
- **Blocking**: "Write integration tests" — integration tests assume unit-level correctness.

## Risks & Edge Cases

- Tests that share the same temp directory name could collide when run in parallel. Mitigate by using a unique subdirectory per test function.
- If cleanup fails (e.g., test panics before the cleanup line), stale temp files remain. Consider using `std::panic::catch_unwind` or placing cleanup in a `Drop` guard if this becomes a problem.
- The byte count assertion depends on the exact format of the confirmation message returned by the handler; the test must match whatever format the implementation uses.
- On some platforms, deeply nested path creation may behave differently; the `write_file_creates_parent_directories` test validates this cross-platform.

## Verification

- `cargo test -p write-file` passes with all six tests green.
- `cargo clippy -p write-file` reports no warnings in the test module.
- Each test is independent and can pass when run in isolation (`cargo test -p write-file <test_name>`).
