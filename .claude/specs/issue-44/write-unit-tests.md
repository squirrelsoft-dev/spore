# Spec: Write unit tests for read_file tool logic

> From: .claude/tasks/issue-44.md

## Objective

Add inline unit tests for the read_file tool's core handler method to verify that it correctly reads file contents from disk, handles missing and empty paths, preserves unicode, and handles empty files. Tests exercise the tool logic directly (without starting an MCP server), using temporary files for filesystem interactions.

## Current State

- **ReadFileTool does not exist yet.** The `tools/read-file/` crate has not been created. Per the task breakdown (`.claude/tasks/issue-44.md`), `read_file.rs` will define a `ReadFileTool` struct with a `#[tool_router]` impl containing a `read_file` method that accepts a `ReadFileRequest { path: String }` and returns a `String` (file contents on success, descriptive error string on failure).

- **Expected ReadFileTool API (from task breakdown):**
  - `ReadFileRequest` struct with a single field `path: String`, deriving `Deserialize` and `JsonSchema`.
  - `ReadFileTool` struct holding a `ToolRouter<Self>`, with a `new()` constructor (same pattern as `EchoTool`).
  - `read_file(&self, Parameters(request): Parameters<ReadFileRequest>) -> String` method under `#[tool_router]`.
  - Validates path is non-empty, returning a descriptive error string if empty.
  - Returns a descriptive error if the file does not exist.
  - Returns a descriptive error if the file exceeds 10 MB.
  - Returns the file contents as a `String` on success.

- **Reference pattern:** `tools/echo-tool/src/echo.rs` defines `EchoTool` with a helper function `call_echo` in its test module that constructs a `Parameters(EchoRequest { ... })` and calls the tool method directly. The read_file tests follow this same pattern.

- **Workspace test conventions:**
  - Inline tests use `#[cfg(test)] mod tests { use super::*; ... }`.
  - Async tests use `#[tokio::test]` with `tokio` as a dev-dependency.
  - Tests construct structs directly and assert on return values. No mocking frameworks.
  - Assertions use `assert_eq!`, `assert!`, and `.contains()` for error message substring checks.

## Requirements

1. **File location:** The `#[cfg(test)] mod tests` block must be appended to `tools/read-file/src/read_file.rs`, the file that defines `ReadFileTool`.

2. **Direct construction:** Tests must construct `ReadFileTool::new()` directly. No MCP server startup, no transport layer, no client connection.

3. **Direct method call via helper:** Define a helper function `call_read_file(tool: &ReadFileTool, path: &str) -> String` that wraps the `Parameters(ReadFileRequest { ... })` construction, mirroring the `call_echo` helper in echo-tool.

4. **Temp files:** Tests that need real files on disk must use `std::env::temp_dir()` combined with unique file names (e.g., using the test function name as a suffix) to create temporary files. Each test must clean up its temp file after assertions. Alternatively, tests may use a unique subdirectory per test.

5. **Five test cases**, each as a separate `#[tokio::test] async fn`:

   | # | Test name | Setup | Input | Assertion |
   |---|-----------|-------|-------|-----------|
   | 1 | `read_file_returns_contents` | Create a temp file containing `"hello from read_file test"` | Path to the temp file | Result string equals `"hello from read_file test"` |
   | 2 | `read_file_nonexistent_path` | None | `"/tmp/nonexistent_file_spore_test_12345.txt"` (a path that does not exist) | Result string contains a substring indicating the file was not found (e.g., `.contains("not found")` or `.contains("does not exist")` or `.contains("No such file")`, case-insensitive check preferred) |
   | 3 | `read_file_empty_path` | None | `""` (empty string) | Result string contains a substring indicating the path is empty or invalid (e.g., `.contains("empty")` or `.contains("path")`) |
   | 4 | `read_file_preserves_unicode` | Create a temp file containing `"\u{1F600} hello \u{00E9}\u{00E8}\u{00EA} \u{4E16}\u{754C}"` | Path to the temp file | Result string equals the exact unicode input |
   | 5 | `read_file_empty_file` | Create a temp file with empty contents (`""`) | Path to the temp file | Result string equals `""` (empty string, not an error) |

6. **No server, no transport:** Tests must not start an MCP server, spawn a process, or use any transport. They test the pure function logic only.

7. **Imports:** The test module imports from `super::*` (to access `ReadFileTool`, `ReadFileRequest`) and `rmcp::handler::server::wrapper::Parameters`. It also uses `std::fs` and `std::io::Write` for temp file management.

## Implementation Details

### File to modify: `tools/read-file/src/read_file.rs`

Append a `#[cfg(test)]` module at the bottom of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    fn temp_file_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("spore_read_file_test_{}", name))
    }

    fn call_read_file(tool: &ReadFileTool, path: &str) -> String {
        tool.read_file(Parameters(ReadFileRequest {
            path: path.to_string(),
        }))
    }

    #[tokio::test]
    async fn read_file_returns_contents() {
        let path = temp_file_path("returns_contents.txt");
        let content = "hello from read_file test";
        fs::write(&path, content).expect("failed to write temp file");

        let tool = ReadFileTool::new();
        let result = call_read_file(&tool, path.to_str().unwrap());
        assert_eq!(result, content);

        fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn read_file_nonexistent_path() {
        let tool = ReadFileTool::new();
        let result = call_read_file(&tool, "/tmp/nonexistent_file_spore_test_12345.txt");
        let lower = result.to_lowercase();
        assert!(
            lower.contains("not found") || lower.contains("does not exist") || lower.contains("no such file"),
            "expected error about missing file, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn read_file_empty_path() {
        let tool = ReadFileTool::new();
        let result = call_read_file(&tool, "");
        let lower = result.to_lowercase();
        assert!(
            lower.contains("empty") || lower.contains("path"),
            "expected error about empty path, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn read_file_preserves_unicode() {
        let path = temp_file_path("preserves_unicode.txt");
        let content = "\u{1F600} hello \u{00E9}\u{00E8}\u{00EA} \u{4E16}\u{754C}";
        fs::write(&path, content).expect("failed to write temp file");

        let tool = ReadFileTool::new();
        let result = call_read_file(&tool, path.to_str().unwrap());
        assert_eq!(result, content);

        fs::remove_file(&path).ok();
    }

    #[tokio::test]
    async fn read_file_empty_file() {
        let path = temp_file_path("empty_file.txt");
        fs::write(&path, "").expect("failed to write temp file");

        let tool = ReadFileTool::new();
        let result = call_read_file(&tool, path.to_str().unwrap());
        assert_eq!(result, "");

        fs::remove_file(&path).ok();
    }
}
```

**Note on `call_read_file` return type:** The echo-tool's `echo` method returns `String` directly (not `Result`). Per the task breakdown, the `read_file` method also returns `String` (returning descriptive error strings rather than `Result`). If the implementer uses a different return type, the helper and assertions must be adapted accordingly.

**Note on temp file cleanup:** Tests use `fs::remove_file(&path).ok()` to silently ignore cleanup failures. This is acceptable for test code since temp files are ephemeral.

### No other files created or modified

This task only adds the `#[cfg(test)] mod tests` block. It does not modify `Cargo.toml` (dev-dependencies are handled by the scaffolding task), `main.rs`, or any other files.

## Dependencies

- **Blocked by:**
  - "Implement `ReadFileTool` struct and handler" (Group 2, issue #44) -- the `ReadFileTool` struct and its `read_file` method must exist before tests can be written against them.
- **Blocking:**
  - "Write README" (Group 4, issue #44) -- the README task depends on tests existing and passing.

## Risks & Edge Cases

1. **Return type variance:** If the implementer wraps the return in `Result<String, ...>` instead of returning a plain `String`, the helper function and all assertions must unwrap or match on the `Result`. The echo-tool pattern returns `String` directly, so this spec assumes the same.

2. **Error message wording:** Tests use `.contains()` with lowercase comparisons to be resilient against exact wording changes in error messages. The implementer should ensure error messages include at least one of the checked substrings.

3. **Temp file race conditions:** Each test uses a unique file name derived from the test function name, so parallel test execution will not cause collisions.

4. **File path encoding:** `temp_file_path` uses `to_str().unwrap()` which will panic if the temp directory contains non-UTF-8 characters. This is acceptable for test environments on all major platforms.

5. **Nonexistent path test:** The path `/tmp/nonexistent_file_spore_test_12345.txt` is chosen to be extremely unlikely to exist. If it somehow does exist, the test will fail with a clear message.

6. **Empty file vs error:** The spec explicitly requires that reading an empty file returns an empty string, not an error. This verifies that the tool does not treat zero-length files as an error condition.

## Verification

1. `cargo test -p read-file` compiles and all 5 test functions pass.
2. `cargo clippy -p read-file --tests` reports no warnings on the test module.
3. Test `read_file_returns_contents` confirms basic file reading works.
4. Test `read_file_nonexistent_path` confirms a descriptive error is returned for missing files.
5. Test `read_file_empty_path` confirms a descriptive error is returned for empty paths.
6. Test `read_file_preserves_unicode` confirms unicode content survives the read round-trip.
7. Test `read_file_empty_file` confirms empty files are handled gracefully (empty string, not error).
