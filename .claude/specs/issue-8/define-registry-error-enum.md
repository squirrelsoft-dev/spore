# Spec: Define `RegistryError` enum

> From: .claude/tasks/issue-8.md

## Objective

Create a `RegistryError` enum for the `tool-registry` crate that represents the three failure modes of tool registry operations: looking up a missing tool, registering a duplicate tool, and failing to connect to an MCP endpoint. This error type follows the codebase convention of manual `Display` and `Error` trait implementations (no `thiserror` dependency) and will be used by `ToolRegistry` methods that return `Result`.

## Current State

- The `tool-registry` crate exists at `crates/tool-registry/` with a placeholder `pub struct ToolRegistry;` in `lib.rs` and no error types.
- The codebase has an established error pattern in `crates/agent-sdk/src/agent_error.rs`:
  - Enum derives `Debug, Clone, PartialEq` (plus `Serialize, Deserialize` for that crate's needs).
  - Manual `impl fmt::Display` with a `match self` block mapping each variant to a `write!` call.
  - Blanket `impl std::error::Error for AgentError {}` with no method overrides.
- The `tool-registry` crate currently has no dependencies beyond the standard library.

## Requirements

- Create a new file `crates/tool-registry/src/registry_error.rs`.
- Define a `pub enum RegistryError` with exactly three variants:
  1. `ToolNotFound { name: String }` -- a requested tool is not registered.
  2. `DuplicateEntry { name: String }` -- a tool with the same name is already registered.
  3. `ConnectionFailed { endpoint: String, reason: String }` -- an MCP endpoint connection attempt failed.
- Derive `Debug`, `Clone`, and `PartialEq` on the enum. Do NOT derive `Serialize`/`Deserialize` (not needed for error types in this crate; no `serde` dependency required for this file).
- Implement `fmt::Display` manually with these exact messages:
  - `ToolNotFound` -> `"tool not found: '{name}'"` (single quotes around name)
  - `DuplicateEntry` -> `"duplicate tool entry: '{name}'"` (single quotes around name)
  - `ConnectionFailed` -> `"connection to '{endpoint}' failed: {reason}"` (single quotes around endpoint, no quotes around reason)
- Implement `std::error::Error` as an empty impl block (matching the `AgentError` pattern).
- The file must use `use std::fmt;` and reference `fmt::Display`, `fmt::Formatter`, `fmt::Result` (not `std::fmt::Display` inline), consistent with `agent_error.rs`.
- Do NOT use `thiserror` or any external crate.
- All functions/blocks must be under 50 lines per project rules.

## Implementation Details

### File to create

**`crates/tool-registry/src/registry_error.rs`**

- Import `std::fmt`.
- Define the enum with `#[derive(Debug, Clone, PartialEq)]`.
- Three variants, all with named fields (struct-style), no tuple variants.
- `impl fmt::Display for RegistryError` with a single `fn fmt` containing a `match self` block with three arms, each calling `write!` with the specified format string.
- `impl std::error::Error for RegistryError {}` -- empty body.

### Structure reference (from `agent_error.rs`)

```rust
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum RegistryError {
    ToolNotFound { name: String },
    DuplicateEntry { name: String },
    ConnectionFailed { endpoint: String, reason: String },
}

impl fmt::Display for RegistryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegistryError::ToolNotFound { name } => {
                write!(f, "tool not found: '{}'", name)
            }
            RegistryError::DuplicateEntry { name } => {
                write!(f, "duplicate tool entry: '{}'", name)
            }
            RegistryError::ConnectionFailed { endpoint, reason } => {
                write!(f, "connection to '{}' failed: {}", endpoint, reason)
            }
        }
    }
}

impl std::error::Error for RegistryError {}
```

### Integration points

- This file will be declared as `mod registry_error;` in `lib.rs` and re-exported as `pub use registry_error::RegistryError;` (handled by the "Wire up `lib.rs`" task in Group 3).
- `ToolRegistry` methods (`register`, `assert_exists`, `resolve_for_skill`) will return `Result<T, RegistryError>` (handled by the "Implement `ToolRegistry` struct and methods" task in Group 2).
- No dependency on other crates or modules within `tool-registry` -- this file is self-contained.

## Dependencies

- Blocked by: None (Group 1, independent)
- Blocking: "Implement `ToolRegistry` struct and methods" (Group 2)

## Risks & Edge Cases

- **Display message format must be exact**: Downstream tests (Group 5) will assert on substring matches against the display output. The single-quote wrapping and exact wording must match the specification. Deviations will cause test failures in later tasks.
- **No `Serialize`/`Deserialize` needed**: Unlike `AgentError`, this error type is not serialized over the wire. Adding serde derives would introduce an unnecessary dependency for this file. If serialization is needed later, it can be added without breaking changes.
- **`PartialEq` on `ConnectionFailed`**: Both `endpoint` and `reason` are `String` fields, so `PartialEq` derivation is straightforward. This enables direct equality assertions in tests (e.g., `assert_eq!(err, RegistryError::ToolNotFound { name: "foo".into() })`).

## Verification

- The file compiles without errors: `cargo check -p tool-registry` (requires the `mod registry_error;` declaration in `lib.rs`, which can be temporarily added to verify, or verified after the "Wire up `lib.rs`" task).
- `cargo clippy -p tool-registry` produces no warnings for this file.
- The `Display` output for each variant matches the specified format strings exactly:
  - `RegistryError::ToolNotFound { name: "my-tool".into() }.to_string()` == `"tool not found: 'my-tool'"`
  - `RegistryError::DuplicateEntry { name: "my-tool".into() }.to_string()` == `"duplicate tool entry: 'my-tool'"`
  - `RegistryError::ConnectionFailed { endpoint: "mcp://localhost:7001".into(), reason: "timeout".into() }.to_string()` == `"connection to 'mcp://localhost:7001' failed: timeout"`
- The type implements `std::error::Error` (can be used as `Box<dyn std::error::Error>`).
- The type implements `Debug`, `Clone`, and `PartialEq` (verified by derive usage in tests).
