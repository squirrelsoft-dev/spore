# Spec: Convert orchestrator from binary to library crate

> From: .claude/tasks/issue-15.md

## Objective

Convert the `orchestrator` crate from a binary crate (with a `main.rs` stub) to a library crate (with a `lib.rs` declaring public modules). The orchestrator is not a standalone binary -- it runs inside `agent-runtime` as a `MicroAgent` -- so it must expose its types through a library interface. This is the foundational scaffolding step that unblocks all subsequent orchestrator implementation work.

## Current State

**`crates/orchestrator/src/main.rs`** -- A 3-line placeholder stub:
```rust
fn main() {
    println!("Hello, world!");
}
```

**`crates/orchestrator/Cargo.toml`** -- Minimal package definition with no dependencies:
```toml
[package]
name = "orchestrator"
version = "0.1.0"
edition = "2024"

[dependencies]
```

The Cargo.toml has no `[[bin]]` or `[lib]` section, so Cargo uses its default convention: `src/main.rs` means it is compiled as a binary crate. There is no `src/lib.rs` file. The crate is already a member of the workspace (listed in the root `Cargo.toml`).

No other crate in the workspace currently depends on `orchestrator`.

## Requirements

- Delete `crates/orchestrator/src/main.rs` entirely.
- Create `crates/orchestrator/src/lib.rs` with exactly four public module declarations:
  - `pub mod agent_endpoint;`
  - `pub mod error;`
  - `pub mod orchestrator;`
  - `pub mod config;`
- Create empty placeholder files for each declared module so the crate compiles:
  - `crates/orchestrator/src/agent_endpoint.rs` (empty)
  - `crates/orchestrator/src/error.rs` (empty)
  - `crates/orchestrator/src/orchestrator.rs` (empty)
  - `crates/orchestrator/src/config.rs` (empty)
- The `Cargo.toml` does NOT need a `[lib]` section added -- Cargo's default convention will detect `src/lib.rs` automatically and treat the crate as a library.
- The crate must compile successfully with `cargo check -p orchestrator`.
- The crate must pass `cargo clippy -p orchestrator` with no warnings.
- No new dependencies are added in this task (dependencies are handled by the sibling task "Update orchestrator Cargo.toml with dependencies").

## Implementation Details

### Files to delete

- `crates/orchestrator/src/main.rs` -- Remove entirely. This file contains only a `println!` stub and has no code worth preserving.

### Files to create

1. **`crates/orchestrator/src/lib.rs`**
   - Declare four public modules in this order:
     ```rust
     pub mod agent_endpoint;
     pub mod config;
     pub mod error;
     pub mod orchestrator;
     ```
   - No other content (no imports, no re-exports, no functions). Keep it minimal -- re-exports and convenience imports will be added in later tasks as the module contents are implemented.

2. **`crates/orchestrator/src/agent_endpoint.rs`** -- Empty file (module placeholder).

3. **`crates/orchestrator/src/config.rs`** -- Empty file (module placeholder).

4. **`crates/orchestrator/src/error.rs`** -- Empty file (module placeholder).

5. **`crates/orchestrator/src/orchestrator.rs`** -- Empty file (module placeholder).

### Files unchanged

- **`crates/orchestrator/Cargo.toml`** -- No modifications needed. The `[dependencies]` section remains empty. Cargo will automatically detect `src/lib.rs` and compile as a library crate. No `[lib]` section is necessary.

- **`Cargo.toml` (workspace root)** -- No modifications needed. The `orchestrator` crate is already listed in `workspace.members`.

### Integration points

- Once this task is complete, other crates can add `orchestrator = { path = "../orchestrator" }` to their dependencies and `use orchestrator::{agent_endpoint, config, error, orchestrator};` to import modules.
- The `agent-runtime` crate will eventually depend on `orchestrator` and instantiate the `Orchestrator` as a `MicroAgent`, but that wiring is out of scope for this task.

## Dependencies

- **Blocked by:** Nothing -- this is a Group 1 task with no prerequisites.
- **Blocking:** All Group 2 tasks ("Implement AgentEndpoint struct", "Define registry config format and loader") and all Group 3+ tasks. Every subsequent orchestrator task depends on the library crate structure existing.

## Risks & Edge Cases

- **Module name collision with crate name:** The module `pub mod orchestrator;` shares the same name as the crate itself. This is valid Rust -- the module is accessed as `orchestrator::orchestrator::Orchestrator` from external crates (or `crate::orchestrator::Orchestrator` internally). This is a deliberate design choice from the task definition and is acceptable.
- **Empty modules and warnings:** Empty module files may trigger `clippy` or compiler warnings in some configurations. In practice, Rust and Clippy do not warn on empty modules, so this should not be an issue.
- **Edition 2024 implications:** The crate uses `edition = "2024"`. The module resolution behavior is the same as edition 2021 for this use case (file-per-module convention). No special handling needed.
- **Parallel task conflict:** The sibling task "Update orchestrator Cargo.toml with dependencies" modifies `Cargo.toml` while this task modifies the `src/` directory. These tasks touch different files and can proceed in parallel without merge conflicts.

## Verification

1. **File structure check:**
   - Confirm `crates/orchestrator/src/main.rs` does not exist.
   - Confirm `crates/orchestrator/src/lib.rs` exists and contains the four `pub mod` declarations.
   - Confirm all four module files exist: `agent_endpoint.rs`, `config.rs`, `error.rs`, `orchestrator.rs`.

2. **Compilation check:**
   ```bash
   cargo check -p orchestrator
   ```
   Must succeed with no errors.

3. **Lint check:**
   ```bash
   cargo clippy -p orchestrator
   ```
   Must succeed with no warnings.

4. **Workspace integrity:**
   ```bash
   cargo check
   ```
   Full workspace build must still succeed (no regressions in `agent-sdk`, `agent-runtime`, `skill-loader`, `tool-registry`).

5. **Test suite:**
   ```bash
   cargo test
   ```
   Full workspace tests must still pass. The orchestrator crate will have no tests yet (empty modules), but existing crate tests must not regress.
