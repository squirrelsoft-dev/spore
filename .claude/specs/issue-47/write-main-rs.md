# Spec: Write `src/main.rs`

> From: .claude/tasks/issue-47.md

## Objective

Create the entry-point file `tools/cargo-build/src/main.rs` that boots the `cargo-build` MCP tool via `mcp_tool_harness::serve_stdio_tool`. The file must mirror the structure of `tools/echo-tool/src/main.rs` exactly and remain under 10 lines.

## Current State

The file `tools/cargo-build/src/main.rs` does not yet exist. The established pattern is visible in `tools/echo-tool/src/main.rs` (7 lines): declare the module, import the tool struct, define an async `main`, and delegate to `serve_stdio_tool`.

## Requirements

1. Declare `mod cargo_build;` to pull in the sibling module.
2. Import `CargoBuildTool` from that module (`use cargo_build::CargoBuildTool;`).
3. Annotate `main` with `#[tokio::main(flavor = "current_thread")]`.
4. Return `Result<(), Box<dyn std::error::Error>>`.
5. Body: a single expression `mcp_tool_harness::serve_stdio_tool(CargoBuildTool::new(), "cargo-build").await`.
6. Total file length must be under 10 lines (target: 7 lines, matching echo-tool).

## Implementation Details

The file should be exactly:

```rust
mod cargo_build;
use cargo_build::CargoBuildTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    mcp_tool_harness::serve_stdio_tool(CargoBuildTool::new(), "cargo-build").await
}
```

No additional imports, logic, or error handling beyond what `serve_stdio_tool` already provides.

## Dependencies

- **Blocked by**: `src/cargo_build.rs` must exist and export `CargoBuildTool` with a `new()` constructor; otherwise this file will not compile.
- **Crate dependencies**: `tokio` (with `rt` and `macros` features) and `mcp_tool_harness` must be listed in `tools/cargo-build/Cargo.toml`.

## Risks & Edge Cases

- If the module file is named incorrectly (e.g., `cargo-build.rs` with a hyphen instead of `cargo_build.rs` with an underscore), `mod cargo_build;` will fail to resolve. Rust module names use underscores.
- If `CargoBuildTool::new()` is not publicly exported, compilation will fail.

## Verification

1. `cargo check -p cargo-build` compiles without errors (requires the sibling module and Cargo.toml to be in place).
2. `wc -l tools/cargo-build/src/main.rs` reports fewer than 10 lines.
3. `diff <(sed 's/echo/cargo_build/g; s/EchoTool/CargoBuildTool/g; s/echo-tool/cargo-build/g' tools/echo-tool/src/main.rs) tools/cargo-build/src/main.rs` produces no output, confirming structural parity with echo-tool.
