# Spec: Add `unique_temp_dir` helper to `mcp-test-utils`

> Task: Add `unique_temp_dir` helper to `mcp-test-utils` `[S]`

## Objective

Move the `unique_temp_dir(test_name) -> PathBuf` helper from the write-file tool's inline test module into the `mcp-test-utils` crate as a public function. Replace the hard-coded `"write_file_tests"` prefix with `"spore_tests"` so the helper is tool-agnostic and reusable across all tool crates in the workspace.

## Current State

- **Source function:** `tools/write-file/src/write_file.rs` contains a private `unique_temp_dir` function inside its `#[cfg(test)] mod tests` block (lines 77-85). It constructs a unique temporary directory at `<temp_dir>/write_file_tests/<test_name>/<pid>`, removes any prior contents, creates the directory tree, and returns the `PathBuf`.

- **Current implementation:**
  ```rust
  fn unique_temp_dir(test_name: &str) -> std::path::PathBuf {
      let dir = env::temp_dir()
          .join("write_file_tests")
          .join(test_name)
          .join(format!("{}", std::process::id()));
      let _ = fs::remove_dir_all(&dir);
      fs::create_dir_all(&dir).expect("failed to create temp dir");
      dir
  }
  ```

- **`mcp-test-utils` crate:** Does not exist yet. This task is blocked by the "Create `crates/mcp-test-utils` crate" task, which will set up the crate scaffolding (`Cargo.toml`, `lib.rs`, workspace member entry).

## Requirements

1. **File location:** Add the `unique_temp_dir` function to `crates/mcp-test-utils/src/lib.rs` (or a submodule re-exported from `lib.rs`).

2. **Public visibility:** The function must be `pub` so that downstream tool crates can call it as `mcp_test_utils::unique_temp_dir("test_name")`.

3. **Signature:** `pub fn unique_temp_dir(test_name: &str) -> std::path::PathBuf`

4. **Prefix change:** The directory prefix must be `"spore_tests"` instead of `"write_file_tests"`. The resulting path is `<temp_dir>/spore_tests/<test_name>/<pid>`.

5. **Behaviour must match the original:**
   - Construct path as `env::temp_dir().join("spore_tests").join(test_name).join(format!("{}", std::process::id()))`.
   - Remove any existing directory at that path (silently ignore errors via `let _ = fs::remove_dir_all(&dir)`).
   - Create the full directory tree via `fs::create_dir_all(&dir).expect("failed to create temp dir")`.
   - Return the `PathBuf`.

6. **No new dependencies:** The function uses only `std::env`, `std::fs`, and `std::path::PathBuf`. No external crates are needed beyond what the `mcp-test-utils` crate scaffolding already provides.

7. **Unit tests:** Add at least three `#[test]` functions in an inline `#[cfg(test)] mod tests` block within the same file:

   | # | Test name | Assertion |
   |---|-----------|-----------|
   | 1 | `unique_temp_dir_creates_directory` | The returned path exists and is a directory (`dir.exists()` and `dir.is_dir()`). Clean up after. |
   | 2 | `unique_temp_dir_includes_test_name` | The returned path string contains the provided test name substring. |
   | 3 | `unique_temp_dir_uses_spore_tests_prefix` | The returned path string contains `"spore_tests"` and does **not** contain `"write_file_tests"`. |

## Implementation Details

### File to modify: `crates/mcp-test-utils/src/lib.rs`

Add the following public function and test module (the crate scaffolding will already have `lib.rs` in place from the prerequisite task):

```rust
use std::env;
use std::fs;
use std::path::PathBuf;

/// Creates a unique temporary directory for a test, scoped by test name and process ID.
///
/// Path format: `<temp_dir>/spore_tests/<test_name>/<pid>`
///
/// Any pre-existing directory at the path is removed before creation, ensuring
/// a clean slate for each test run.
pub fn unique_temp_dir(test_name: &str) -> PathBuf {
    let dir = env::temp_dir()
        .join("spore_tests")
        .join(test_name)
        .join(format!("{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("failed to create temp dir");
    dir
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unique_temp_dir_creates_directory() {
        let dir = unique_temp_dir("creates_directory");
        assert!(dir.exists(), "directory should exist");
        assert!(dir.is_dir(), "path should be a directory");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_temp_dir_includes_test_name() {
        let dir = unique_temp_dir("my_specific_test");
        let path_str = dir.to_string_lossy();
        assert!(
            path_str.contains("my_specific_test"),
            "path should contain test name, got: {}",
            path_str
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unique_temp_dir_uses_spore_tests_prefix() {
        let dir = unique_temp_dir("prefix_check");
        let path_str = dir.to_string_lossy();
        assert!(
            path_str.contains("spore_tests"),
            "path should contain 'spore_tests', got: {}",
            path_str
        );
        assert!(
            !path_str.contains("write_file_tests"),
            "path should not contain old prefix 'write_file_tests', got: {}",
            path_str
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
```

### No other files created or modified

This task only adds the `unique_temp_dir` function and its tests to the `mcp-test-utils` crate. It does not modify write-file or any other tool crate (that is handled by the downstream "Migrate write-file tests" task).

## Dependencies

- **Blocked by:**
  - "Create `crates/mcp-test-utils` crate" -- the crate scaffolding (`Cargo.toml`, `lib.rs`, workspace member entry) must exist before this function can be added.
- **Blocking:**
  - "Migrate write-file tests" -- downstream tool crates will replace their local `unique_temp_dir` with `mcp_test_utils::unique_temp_dir` once this function is available.

## Risks & Edge Cases

1. **Parallel test execution:** The path includes `std::process::id()`, which is unique per process. Since `cargo test` runs all tests within a single process, two tests with the same `test_name` argument would collide. Each call site must use a distinct `test_name` value.

2. **Non-UTF-8 temp directory:** `env::temp_dir()` could theoretically return a non-UTF-8 path on some platforms. Tests use `to_string_lossy()` for assertions, which handles this gracefully. Callers using `to_str().unwrap()` (as the write-file tests do) would panic, but this is acceptable in test environments on all major platforms.

3. **Cleanup responsibility:** The function does not register any automatic cleanup. Callers are responsible for removing the directory after use (typically `let _ = fs::remove_dir_all(&dir)` at the end of each test). The function does clean up stale directories from prior runs of the same test.

4. **`remove_dir_all` on first call:** On the very first run, the directory does not exist, so `remove_dir_all` returns an error which is silently ignored via `let _ = ...`. This is intentional.

## Verification

1. `cargo test -p mcp-test-utils` compiles and all 3 test functions pass.
2. `cargo clippy -p mcp-test-utils --tests` reports no warnings.
3. `cargo check --workspace` confirms no breakage in the wider workspace.
