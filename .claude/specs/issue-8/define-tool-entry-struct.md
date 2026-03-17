# Spec: Define `ToolEntry` struct

> From: .claude/tasks/issue-8.md

## Objective

Create the `ToolEntry` data struct that represents a registered tool in the tool-registry crate. This struct maps a tool's name and version to its MCP endpoint URL, forming the core data type that `ToolRegistry` will store and look up. It must be serializable, deserializable, and JSON Schema-capable so it can be persisted, transmitted, and validated consistently with other types in the workspace.

## Current State

- The `tool-registry` crate exists at `crates/tool-registry/` with a placeholder unit struct `pub struct ToolRegistry;` in `src/lib.rs` and an empty `[dependencies]` section in `Cargo.toml`.
- The `agent-sdk` crate defines all domain types using a consistent derive stack: `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]` with imports from `schemars::JsonSchema` and `serde::{Deserialize, Serialize}`.
- Examples of this pattern: `Constraints` (`crates/agent-sdk/src/constraints.rs`), `ToolCallRecord` (`crates/agent-sdk/src/tool_call_record.rs`), `SkillManifest` (`crates/agent-sdk/src/skill_manifest.rs`).
- The `ToolEntry` file does not yet exist.

## Requirements

- Create `crates/tool-registry/src/tool_entry.rs` containing a single public struct `ToolEntry`.
- The struct must have exactly three public fields:
  - `name: String` -- the unique tool name used for registry lookups.
  - `version: String` -- the tool's version string.
  - `endpoint: String` -- the MCP endpoint URL (e.g., `"mcp://localhost:7001"`) or a unix socket path (e.g., `"/tmp/tool.sock"`).
- Apply the derive stack `#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]` matching the codebase convention.
- Import `schemars::JsonSchema` and `serde::{Deserialize, Serialize}` at the top of the file, matching the import style used in `agent-sdk` types.
- Do NOT include a `handle: ToolServerHandle` field. That type does not exist yet and depends on `rmcp` integration in issue #9.
- Do NOT add any methods, trait implementations, or test modules in this file. Tests will be added in Group 5.
- The file must compile once the `serde` and `schemars` dependencies are added to `tool-registry/Cargo.toml` (a parallel Group 1 task).

## Implementation Details

### File to create

**`crates/tool-registry/src/tool_entry.rs`**

- Add imports: `use schemars::JsonSchema;` and `use serde::{Deserialize, Serialize};`
- Define the struct with the derive stack and three `pub` fields as specified above.
- No additional code -- no `impl` blocks, no `#[cfg(test)]` modules, no helper functions.

### Integration points

- This file will be declared as `mod tool_entry;` and re-exported as `pub use tool_entry::ToolEntry;` from `crates/tool-registry/src/lib.rs` in the "Wire up `lib.rs`" task (Group 3).
- The `ToolRegistry` struct (Group 2) will store `HashMap<String, ToolEntry>` and return `ToolEntry` values from `get()` and `resolve_for_skill()`.
- The `ToolEntry` struct must be `Clone` (for returning owned copies from behind `RwLock`) and `PartialEq` (for test assertions).

## Dependencies

- Blocked by: None (Group 1, independent). However, the file will not compile in isolation until the parallel task "Add dependencies to tool-registry Cargo.toml" adds `serde` and `schemars`.
- Blocking: "Implement `ToolRegistry` struct and methods" (Group 2)

## Risks & Edge Cases

- **Compile order**: This file depends on `serde` and `schemars` being present in `Cargo.toml`. If implemented before the dependency task, `cargo check` on the crate will fail. Both tasks are in Group 1 and should be completed together before moving to Group 2.
- **Endpoint format not validated**: The `endpoint` field is a plain `String` with no URL validation. This is intentional -- validation logic belongs in the connection layer (issue #9). If stricter typing is desired later, `endpoint` can be changed to a newtype or `url::Url` without breaking the registry's internal logic.
- **Future field additions**: When `rmcp` integration lands (issue #9), a `handle` field or companion struct will be needed. The current design keeps `ToolEntry` as pure data, making it straightforward to extend or wrap later without breaking serialization.

## Verification

- After both Group 1 tasks are complete, run `cargo check -p tool-registry` and confirm no compiler errors (the struct won't be wired into `lib.rs` yet, so the module will only compile if explicitly checked or once `lib.rs` is updated in Group 3).
- Verify the file contains exactly the three specified fields with the correct types.
- Verify the derive stack matches the codebase convention: `Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema`.
- Verify there are no extra imports, methods, or test modules.
- After Group 3 (lib.rs wiring), confirm `cargo check -p tool-registry` passes with the module declared and re-exported.
