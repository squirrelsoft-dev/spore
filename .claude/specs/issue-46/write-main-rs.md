# Spec: Write `main.rs`

> From: .claude/tasks/issue-46.md

## Objective
Create the entrypoint file for the `validate-skill` MCP tool server. This file mirrors `tools/echo-tool/src/main.rs` exactly in structure, substituting only the module name, struct name, and log message for the validate-skill tool.

## Current State
- `tools/echo-tool/src/main.rs` exists and serves as the canonical template for all tool entrypoints in this workspace.
- The `validate-skill` crate does not yet have a `main.rs`.
- The `ValidateSkillTool` struct and its handler (in a sibling module) are not yet implemented; this task is blocked by that work.

## Requirements
- Create `tools/validate-skill/src/main.rs` following the exact same pattern as `tools/echo-tool/src/main.rs`.
- The module declaration must be `mod validate_skill;` (not `mod echo;`).
- The use statement must import `validate_skill::ValidateSkillTool` (not `echo::EchoTool`).
- The log line must read `"Starting validate-skill MCP server"`.
- The struct instantiated in `main()` must be `ValidateSkillTool::new()`.
- All logging goes to stderr via `tracing_subscriber` (never stdout, which is the MCP transport channel).
- The return type must be `Result<(), Box<dyn std::error::Error>>` (matching the echo-tool pattern).
- No `clap` dependency. No CLI argument parsing.
- The file must stay under 50 lines.
- No other behavioral or structural changes from the echo-tool template.

## Implementation Details

### File: `tools/validate-skill/src/main.rs`

```rust
use rmcp::ServiceExt;
use tracing_subscriber::{self, EnvFilter};

mod validate_skill;
use validate_skill::ValidateSkillTool;

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::DEBUG.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    tracing::info!("Starting validate-skill MCP server");

    let service = ValidateSkillTool::new()
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| tracing::error!("serving error: {:?}", e))?;

    service.waiting().await?;
    Ok(())
}
```

This is 24 lines, well within the 50-line budget.

### Changes from `tools/echo-tool/src/main.rs`

| Line | echo-tool | validate-skill |
|------|-----------|----------------|
| 4 | `mod echo;` | `mod validate_skill;` |
| 5 | `use echo::EchoTool;` | `use validate_skill::ValidateSkillTool;` |
| 15 | `"Starting echo-tool MCP server"` | `"Starting validate-skill MCP server"` |
| 17 | `EchoTool::new()` | `ValidateSkillTool::new()` |

No other lines change.

## Dependencies
- Blocked by: "Implement `ValidateSkillTool` struct and handler"
- Blocking: "Write integration test"

## Risks & Edge Cases

1. **`ValidateSkillTool` not yet available:** This file imports `validate_skill::ValidateSkillTool`, which must be defined in `tools/validate-skill/src/validate_skill.rs`. If the blocked task has not landed, compilation will fail. Mitigation: ensure the handler task is complete before attempting to build.

2. **Stdout contamination:** Any accidental `println!` or default tracing subscriber writing to stdout will corrupt the MCP stdio transport. Mitigation: the `tracing_subscriber` is explicitly configured with `.with_writer(std::io::stderr)`, and no `println!` calls should appear in this file.

3. **Module naming:** The module file must be named `validate_skill.rs` (with underscore), matching the `mod validate_skill;` declaration. A mismatch (e.g., `validate-skill.rs` with a hyphen) will cause a compilation error.

## Verification

1. **Compilation:** `cargo check -p validate-skill` succeeds with no errors (requires the handler module to exist).
2. **Lint:** `cargo clippy -p validate-skill` produces no warnings.
3. **Line count:** `wc -l tools/validate-skill/src/main.rs` is under 50 lines.
4. **Diff check:** The file differs from `tools/echo-tool/src/main.rs` in exactly four lines (module name, use statement, log message, struct instantiation).
5. **Server starts:** `cargo run -p validate-skill` starts without errors and waits for MCP client input on stdin (verified by checking stderr log output shows "Starting validate-skill MCP server").
6. **Workspace tests:** `cargo test` across the full workspace still passes (no regressions).
