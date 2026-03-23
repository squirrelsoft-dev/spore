# Spec: Add Rust integration test wrapper for E2E bootstrap

> From: .claude/tasks/issue-22.md

## Objective

Create a Rust integration test that wraps the `scripts/e2e-test.sh` shell script, providing a `cargo test` entry point for the end-to-end bootstrap test. The test is gated behind a Cargo feature flag (`e2e`) and marked `#[ignore]` so it never runs in the default `cargo test` invocation. This allows CI or developers to explicitly opt in with `cargo test --features e2e -- --ignored` while keeping the normal test suite fast and free of external dependencies.

## Current State

- **Workspace `Cargo.toml`:** Defines a virtual workspace with `resolver = "2"` and six member crates. There is no `[package]` or `[features]` section at the workspace level.
- **`scripts/e2e-test.sh`:** Does not exist yet. This task is blocked by the E2E shell script task, which will create this script. The test wrapper assumes the script exists at the workspace root-relative path `scripts/e2e-test.sh`, is executable, and returns exit code 0 on success and non-zero on failure.
- **No workspace-level integration tests exist yet.** The `tests/` directory at the workspace root does not exist. Existing integration tests live inside individual crates (e.g., `crates/skill-loader/tests/`).
- **Workspace test conventions:**
  - Integration tests use `#[tokio::test]` for async or `#[test]` for sync.
  - Assertions use `assert!`, `assert_eq!`, and pattern matching.
  - Test functions have descriptive names that state the expected behavior.
  - No mocking frameworks are used.

## Requirements

1. **Add `[package]` section to workspace `Cargo.toml`:** Since the current root `Cargo.toml` is a pure virtual workspace (no `[package]`), a minimal `[package]` section must be added so that `[features]` can be defined and `tests/` at the workspace root belongs to a package. This is a standard Cargo pattern for workspaces that also need root-level tests or features.

2. **Add `e2e` feature to workspace `Cargo.toml`:** Add a `[features]` section to the root `Cargo.toml` with an `e2e` feature that has no dependencies (empty array). This feature acts as a compile-time gate for the E2E test.

3. **Create `tests/e2e_bootstrap_test.rs`:** Create a workspace-level integration test file at the repository root. This file contains a single test function that shells out to `scripts/e2e-test.sh`.

4. **Feature gate:** The entire test file must be gated with `#![cfg(feature = "e2e")]` so that the test is not even compiled unless the `e2e` feature is enabled.

5. **`#[ignore]` attribute:** The test function must be annotated with both `#[test]` and `#[ignore]`. The `#[ignore]` attribute provides a second layer of protection: even when compiled with `--features e2e`, the test only runs when explicitly requested with `--ignored` or `--include-ignored`.

6. **Script execution:** The test must:
   - Locate `scripts/e2e-test.sh` relative to the workspace root using `env!("CARGO_MANIFEST_DIR")`.
   - Spawn the script using `std::process::Command`.
   - Set the working directory to the workspace root.
   - Capture stdout/stderr and print them on failure for debugging.
   - Assert that the exit code is 0.

7. **No async runtime needed:** A plain `#[test]` is sufficient. No additional dependencies are required.

8. **No new crate dependencies:** Only `std::process::Command` from the standard library is needed.

## Implementation Details

### File to modify: `Cargo.toml` (workspace root)

Add a `[package]` section and a `[features]` section. The resulting file should look like:

```toml
[package]
name = "spore"
version = "0.0.0"
edition = "2021"
publish = false

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

[features]
e2e = []

[profile.release]
lto = true
opt-level = "z"
codegen-units = 1
strip = true
panic = "abort"
```

**Key points:**
- `[package]` must appear before `[workspace]` for Cargo to parse it correctly as a package that is also a workspace root.
- `publish = false` prevents accidental publishing to crates.io.
- `version = "0.0.0"` signals this is not a real publishable package.
- The root package is implicitly a workspace member when `[package]` and `[workspace]` coexist; no changes to the `members` array are needed.

### File to create: `tests/e2e_bootstrap_test.rs`

```rust
#![cfg(feature = "e2e")]

use std::path::Path;
use std::process::Command;

#[test]
#[ignore]
fn e2e_bootstrap_runs_successfully() {
    let workspace_root = env!("CARGO_MANIFEST_DIR");
    let script_path = Path::new(workspace_root).join("scripts/e2e-test.sh");

    assert!(
        script_path.exists(),
        "E2E test script not found at: {}",
        script_path.display()
    );

    let output = Command::new(&script_path)
        .current_dir(workspace_root)
        .output()
        .unwrap_or_else(|e| panic!(
            "Failed to execute E2E test script at {}: {e}",
            script_path.display()
        ));

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        panic!(
            "E2E bootstrap test failed with exit code {:?}\n\n--- stdout ---\n{stdout}\n\n--- stderr ---\n{stderr}",
            output.status.code()
        );
    }
}
```

### Design decisions

- **`#![cfg(feature = "e2e")]` at crate level:** Using a crate-level inner attribute gates the entire file so nothing compiles without the feature. This prevents any compilation overhead when the feature is off.

- **`#[ignore]` as double gate:** The feature flag prevents compilation; `#[ignore]` prevents execution even when compiled. This two-layer approach means:
  - `cargo test` -- does not compile or run the E2E test.
  - `cargo test --features e2e` -- compiles but skips (shows "ignored").
  - `cargo test --features e2e -- --ignored` -- compiles and runs the E2E test.
  - `cargo test --features e2e -- --include-ignored` -- runs all tests including the E2E test.

- **`Command::output()` over `Command::status()`:** Captures stdout and stderr, which are printed on failure for easier CI debugging.

- **`env!("CARGO_MANIFEST_DIR")`:** Compile-time macro that resolves to the directory containing the `Cargo.toml` owning the test. For workspace-root integration tests, this is the workspace root. More reliable than `std::env::current_dir()`.

- **Existence check before execution:** The `assert!(script_path.exists(), ...)` provides a clear error message if the script is missing, rather than an opaque OS error.

- **Plain `#[test]`, not `#[tokio::test]`:** No async operations needed. `std::process::Command` blocks synchronously until the child process exits.

## Dependencies

- **Blocked by:**
  - "E2E shell script" -- the `scripts/e2e-test.sh` script must exist for the test to pass. The Rust test file and Cargo.toml changes can be created before the script exists; the test will fail with the "script not found" assertion until the script is in place.

- **Blocking:**
  - "README section" -- the README documentation for E2E testing depends on this test wrapper existing so it can document the `cargo test --features e2e -- --ignored` invocation.

## Risks & Edge Cases

1. **Script not executable:** If `scripts/e2e-test.sh` lacks the executable bit (`chmod +x`), `Command` will fail with a permission error. The E2E shell script task should ensure the file is executable. The `unwrap_or_else` panic message will clearly indicate the problem.

2. **Windows compatibility:** Running a `.sh` script via `std::process::Command` will not work on Windows without a shell interpreter. This is acceptable because the E2E test targets Linux/macOS CI. If Windows support is needed later, the script can be invoked via `bash scripts/e2e-test.sh` instead.

3. **Long-running script:** The E2E bootstrap script may take significant time (building, spawning servers, HTTP requests). The `#[test]` harness has no default timeout. If the script hangs, the test hangs. A `timeout` wrapper could be added later if needed.

4. **Adding `[package]` to root `Cargo.toml`:** Converting from a pure virtual workspace to a package+workspace is a well-supported Cargo pattern, but it changes semantics slightly: `cargo build` in the root will now also consider the root package (which has no `src/`). Since there is no `src/lib.rs` or `src/main.rs`, Cargo will not try to build a library or binary for the root package -- it will only compile tests when `cargo test` is run. This is the desired behavior. If Cargo complains about a missing `src/`, an empty `src/lib.rs` can be created, but this should not be necessary for a package with only `tests/`.

5. **Feature leakage:** The `e2e` feature is defined on the root package only and has no downstream effects on member crates. It only controls conditional compilation within the root package's test files.

6. **Cargo workspace member list:** The root package is automatically a workspace member when `[package]` and `[workspace]` coexist in the same file. No changes to the `members` array are needed.

## Verification

1. `cargo check --features e2e` compiles successfully (including the test file).
2. `cargo check` (without `--features e2e`) does not compile the test file.
3. `cargo test` does not show `e2e_bootstrap_runs_successfully` in output at all.
4. `cargo test --features e2e` shows `e2e_bootstrap_runs_successfully` as `ignored`.
5. `cargo test --features e2e -- --ignored` runs the E2E test (passes once `scripts/e2e-test.sh` exists and works).
6. `cargo clippy --features e2e --tests` reports no warnings on the test file.
7. `cargo clippy --tests` (without `--features e2e`) does not lint the test file.
8. The existing `cargo test` suite across all workspace members continues to pass with no regressions.
