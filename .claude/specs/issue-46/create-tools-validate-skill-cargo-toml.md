# Spec: Create `tools/validate-skill/Cargo.toml`

> From: .claude/tasks/issue-46.md

## Objective

Create the `tools/validate-skill/Cargo.toml` manifest by copying the structure from `tools/echo-tool/Cargo.toml`, changing the package name to `validate-skill`, and adding path dependencies on `skill-loader` and `agent-sdk`. This establishes the crate manifest for the validate-skill MCP tool, which unlike echo-tool requires access to workspace library crates for skill file parsing and validation.

## Current State

- `tools/echo-tool/` exists as a fully implemented MCP tool crate and serves as the template for new tools.
- `tools/echo-tool/Cargo.toml` declares:
  - `edition = "2024"`, `version = "0.1.0"`
  - Dependencies: `rmcp` (with `transport-io`, `server`, `macros` features), `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`
  - Dev-dependencies: `tokio` (with `macros`, `rt`, `rt-multi-thread`), `rmcp` (with `client`, `transport-child-process`), `serde_json`
- `crates/skill-loader/` exists with `skill-loader` crate at version `0.1.0`, edition `2024`.
- `crates/agent-sdk/` exists with `agent-sdk` crate at version `0.1.0`, edition `2024`.
- No `tools/validate-skill/` directory or crate exists yet.
- The workspace root `Cargo.toml` does not yet include `tools/validate-skill` in its members list (that is a separate task).

## Requirements

- Create `tools/validate-skill/Cargo.toml` with the following `[package]` section:
  - `name = "validate-skill"`
  - `version = "0.1.0"`
  - `edition = "2024"`
- The `[dependencies]` section must contain exactly these eight entries:
  - `rmcp = { version = "1", features = ["transport-io", "server", "macros"] }`
  - `tokio = { version = "1", features = ["macros", "rt", "io-std"] }`
  - `serde = { version = "1", features = ["derive"] }`
  - `serde_json = "1"`
  - `tracing = "0.1"`
  - `tracing-subscriber = { version = "0.3", features = ["env-filter"] }`
  - `skill-loader = { path = "../../crates/skill-loader" }`
  - `agent-sdk = { path = "../../crates/agent-sdk" }`
- The `[dev-dependencies]` section must contain exactly these three entries (copied from echo-tool):
  - `tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }`
  - `rmcp = { version = "1", features = ["client", "transport-child-process"] }`
  - `serde_json = "1"`
- No new third-party dependencies beyond what echo-tool already uses.
- Create a placeholder `tools/validate-skill/src/main.rs` with `fn main() {}` so that `cargo check` succeeds once the crate is added to the workspace.

## Implementation Details

### Files to create

1. **`tools/validate-skill/Cargo.toml`** -- the crate manifest:

   ```toml
   [package]
   name = "validate-skill"
   version = "0.1.0"
   edition = "2024"

   [dependencies]
   rmcp = { version = "1", features = ["transport-io", "server", "macros"] }
   tokio = { version = "1", features = ["macros", "rt", "io-std"] }
   serde = { version = "1", features = ["derive"] }
   serde_json = "1"
   tracing = "0.1"
   tracing-subscriber = { version = "0.3", features = ["env-filter"] }
   skill-loader = { path = "../../crates/skill-loader" }
   agent-sdk = { path = "../../crates/agent-sdk" }

   [dev-dependencies]
   tokio = { version = "1", features = ["macros", "rt", "rt-multi-thread"] }
   rmcp = { version = "1", features = ["client", "transport-child-process"] }
   serde_json = "1"
   ```

2. **`tools/validate-skill/src/main.rs`** -- minimal placeholder:

   ```rust
   fn main() {}
   ```

### Files NOT modified by this task

- `Cargo.toml` (root workspace) -- adding the crate to workspace members is a separate task ("Add `tools/validate-skill` to workspace members").

### Key design decisions

- **Base dependencies copied from echo-tool:** The validate-skill tool uses the same MCP server pattern (rmcp over stdio transport with tracing to stderr), so it needs the identical base dependency set as echo-tool.
- **Path dependencies on `skill-loader` and `agent-sdk`:** Unlike echo-tool and read-file which are standalone binaries, validate-skill needs access to `skill_loader::parse_content`, `skill_loader::validate`, `skill_loader::AllToolsExist`, and `agent_sdk::SkillManifest` for its validation logic. The relative paths `../../crates/skill-loader` and `../../crates/agent-sdk` are correct from `tools/validate-skill/`.
- **Dev-dependencies include client-side rmcp:** The `client` and `transport-child-process` features in dev-dependencies support integration tests that spawn the tool as a child process and communicate over MCP, matching the echo-tool test pattern.
- **No additional third-party crates:** The `skill-loader` and `agent-sdk` crates bring their own transitive dependencies (e.g., `serde_yaml`, `async-trait`, `schemars`, `uuid`), so no new third-party dependencies need to be declared directly.

## Dependencies

- Blocked by: Nothing (Group 2 task, can begin once Group 1 is complete per task breakdown, but this Cargo.toml creation has no code dependency on Group 1)
- Blocking: "Implement `ValidateSkillTool` struct and handler", "Write `main.rs`", "Write integration test" (all require this crate manifest to exist)

## Risks & Edge Cases

- **Placeholder main.rs warnings:** The placeholder `fn main() {}` does not use any declared dependencies, which will produce unused-dependency warnings from `cargo clippy`. This is acceptable since the real implementation replaces it in a subsequent task.
- **Workspace root not updated:** This task intentionally does NOT add the crate to the workspace `members` list. Until the companion task completes, `cargo build` from the workspace root will not build validate-skill. This is by design.
- **Path dependency resolution:** The relative paths `../../crates/skill-loader` and `../../crates/agent-sdk` must resolve correctly from `tools/validate-skill/`. Verify directory structure matches.
- **Transitive dependency conflicts:** Adding `skill-loader` and `agent-sdk` as path dependencies pulls in their dependency trees. Since both already exist in the workspace and all crates use compatible versions, no conflicts are expected.

## Verification

- `tools/validate-skill/Cargo.toml` exists and contains the exact `[package]`, `[dependencies]`, and `[dev-dependencies]` sections specified above.
- `tools/validate-skill/src/main.rs` exists and contains `fn main() {}`.
- The directory structure is `tools/validate-skill/src/main.rs` and `tools/validate-skill/Cargo.toml` (no extra files).
- After the companion task "Add `tools/validate-skill` to workspace members" completes, `cargo check -p validate-skill` succeeds without errors.
- The `[dependencies]` section contains exactly eight entries: the six from echo-tool plus `skill-loader` and `agent-sdk`.
- The `[dev-dependencies]` section matches echo-tool exactly (three entries).
- Path dependencies use correct relative paths (`../../crates/skill-loader` and `../../crates/agent-sdk`).
