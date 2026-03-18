# Spec: Add release profile optimizations to workspace `Cargo.toml`

> From: .claude/tasks/issue-14.md

## Objective

Add a `[profile.release]` section to the workspace-level `Cargo.toml` with binary-size-optimizing settings. These settings are critical for achieving the 1-5 MB Docker image size target defined in issue #14. Without them, the `agent-runtime` binary (which pulls in `tokio`, `axum`, `serde`, `rig-core`, `reqwest`, `rustls`, and `aws-lc-rs`) will be 10-20 MB. With these optimizations, the stripped binary should be in the 5-8 MB range.

## Current State

The workspace `Cargo.toml` at `/workspaces/spore/Cargo.toml` contains only workspace membership configuration and has no profile sections:

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

None of the six member crate `Cargo.toml` files contain any `[profile.*]` sections. There are no existing release profile settings anywhere in the workspace.

## Requirements

- The workspace-level `Cargo.toml` must contain a `[profile.release]` section with exactly these settings:
  - `lto = true` -- enables full link-time optimization across all crates, allowing the linker to eliminate dead code across crate boundaries
  - `opt-level = "z"` -- optimizes aggressively for binary size over speed (more aggressive than `"s"`)
  - `codegen-units = 1` -- compiles each crate as a single codegen unit, enabling maximum optimization at the cost of compilation parallelism
  - `strip = true` -- strips debug symbols and symbol tables from the final binary
  - `panic = "abort"` -- uses abort-on-panic instead of unwinding, removing the unwinding infrastructure from the binary
- The settings must apply to all workspace members via workspace-level inheritance (no per-crate profile overrides)
- The existing `[workspace]` section and its contents must remain unchanged
- `cargo build --release` must complete without errors after the change
- `cargo test` must still pass (Cargo automatically ignores `panic = "abort"` for test builds and uses `unwind` instead, so tests are unaffected)

## Implementation Details

**File to modify:** `Cargo.toml` (workspace root, `/workspaces/spore/Cargo.toml`)

Append the following section after the existing `[workspace]` block:

```toml
[profile.release]
lto = true
opt-level = "z"
codegen-units = 1
strip = true
panic = "abort"
```

### Effect of each setting

| Setting | Default | New Value | Effect on binary size |
|---------|---------|-----------|----------------------|
| `lto` | `false` (thin) | `true` (fat) | Enables cross-crate dead-code elimination; 20-40% size reduction |
| `opt-level` | `3` (speed) | `"z"` (size) | Prioritizes size over speed; 10-30% size reduction |
| `codegen-units` | `16` | `1` | Allows LLVM to optimize across the entire crate as one unit; 5-15% size reduction |
| `strip` | `false` | `true` | Removes symbol tables and debug info; 30-60% size reduction |
| `panic` | `"unwind"` | `"abort"` | Removes stack unwinding infrastructure; 5-10% size reduction |

### No other files need modification

Profile sections in Cargo are only valid at the workspace root level. Individual member crate `Cargo.toml` files do not need changes and cannot override `[profile.release]`.

## Dependencies

- Blocked by: None
- Blocking: "Create multi-stage Dockerfile" (the Dockerfile's `cargo build --release` depends on these settings to produce a small binary)

## Risks & Edge Cases

1. **`panic = "abort"` and tests:** Cargo automatically overrides `panic = "abort"` with `panic = "unwind"` when running `cargo test`, because the test harness requires unwinding to catch panics. Therefore, `cargo test` will continue to work correctly. However, any integration tests that rely on `std::panic::catch_unwind` in the release binary itself (not in test mode) would not be able to catch panics. Currently no code in this workspace uses `catch_unwind`, so this is not a concern.

2. **`panic = "abort"` and runtime behavior:** In production, any panic will immediately terminate the process rather than unwinding the stack. This is appropriate for a server binary that will be restarted by a container orchestrator (Docker, Kubernetes). It means destructors (`Drop` impls) will not run on panic. Review the codebase for any `Drop` impls that perform critical cleanup (e.g., flushing data to disk). Currently the codebase has no such critical `Drop` implementations.

3. **LTO increases compile times significantly:** Full LTO (`lto = true`) can increase release build times by 2-5x compared to the default thin LTO. This only affects `cargo build --release`; debug builds and `cargo test` are unaffected. For CI, this means release builds will be slower, but this is an acceptable tradeoff for the Docker image size target. If build times become problematic, `lto = "thin"` is a middle ground (smaller size improvement but faster builds).

4. **`opt-level = "z"` and performance:** Optimizing for size (`"z"`) may result in slightly slower runtime performance compared to the default `opt-level = 3`. For an AI agent runtime that is primarily I/O-bound (waiting on LLM API calls), this performance difference is negligible.

5. **`codegen-units = 1` and compile times:** Single codegen unit prevents parallel code generation within a crate, increasing compile times. Combined with full LTO, release builds will be noticeably slower. This is acceptable since release builds are infrequent (Docker image builds, CI releases).

6. **No impact on `cargo build` (debug):** All settings are under `[profile.release]` and do not affect debug builds. Developer iteration speed with `cargo build` / `cargo run` is unchanged.

## Verification

1. **Syntax check:** Run `cargo check` to confirm the workspace `Cargo.toml` parses correctly with the new profile section.

2. **Release build succeeds:** Run `cargo build --release` and confirm it completes without errors.

3. **Tests pass:** Run `cargo test` and confirm all tests pass (verifying `panic = "abort"` does not break the test harness).

4. **Binary size comparison (informational):** Compare the size of `target/release/agent-runtime` before and after the change. Expected reduction is 50-70% (e.g., from ~15 MB to ~5-8 MB). This can be checked with `ls -lh target/release/agent-runtime`.

5. **Lint check:** Run `cargo clippy` to confirm no new warnings are introduced.

6. **Settings are applied:** Run `cargo build --release -v 2>&1 | grep -E '(lto|codegen-units|opt-level)'` to confirm the compiler flags reflect the new settings.
