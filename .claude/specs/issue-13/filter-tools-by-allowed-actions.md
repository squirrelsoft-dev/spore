# Spec: Filter tools by `allowed_actions` in tool resolution

> From: .claude/tasks/issue-13.md

## Objective

Enforce skill-level `allowed_actions` constraints by filtering MCP tools at resolution time, before they are ever presented to the LLM. Tools that carry an `action_type` not listed in the skill's `allowed_actions` are excluded from the resolved tool set. This is structural enforcement: the LLM cannot call a disallowed tool because it never sees it.

## Current State

### `ToolEntry` (`crates/tool-registry/src/tool_entry.rs`)

The `ToolEntry` struct has four fields: `name`, `version`, `endpoint`, and `handle`. There is no field to classify a tool by action category. The manual `Clone` impl copies all fields except `handle` (set to `None`), and the `PartialEq` impl compares `name`, `version`, and `endpoint`.

### `resolve_mcp_tools()` (`crates/agent-runtime/src/tool_bridge.rs`)

Takes `&ToolRegistry` and `&SkillManifest`. Calls `registry.resolve_for_skill(manifest)` to get entries matching the manifest's `tools` list, then queries each entry's MCP handle to list available tools. Returns a flat `Vec<McpTool>` with no filtering beyond what the manifest names.

### `Constraints` (`crates/agent-sdk/src/constraints.rs`)

Already defines `allowed_actions: Vec<String>` on the `Constraints` struct, which is accessible via `manifest.constraints.allowed_actions`.

### `main.rs` (`crates/agent-runtime/src/main.rs`)

Constructs `ToolEntry` values from `TOOL_ENDPOINTS` env var with `name`, `version`, `endpoint`, and `handle: None`. The `resolve_tools()` helper in `provider.rs` calls `tool_bridge::resolve_mcp_tools(registry, manifest)` and currently does not pass `allowed_actions`.

### `provider.rs` (`crates/agent-runtime/src/provider.rs`)

The `resolve_tools()` helper calls `tool_bridge::resolve_mcp_tools(registry, manifest)`. The `build_agent()` function passes the full manifest so `allowed_actions` is available at this call site.

## Requirements

1. **New field on `ToolEntry`**: Add `action_type: Option<String>` to `ToolEntry`. When `None`, the tool is unrestricted (included regardless of `allowed_actions`). When `Some(t)`, the tool is categorized as action type `t` and subject to filtering.

2. **Serde annotation**: The new field must have `#[serde(default, skip_serializing_if = "Option::is_none")]` so existing serialized entries without the field deserialize correctly and the field is omitted from JSON when `None`.

3. **Updated `Clone` impl**: The manual `Clone` implementation must copy `action_type`.

4. **Updated `PartialEq` impl**: The manual `PartialEq` implementation must compare `action_type`.

5. **Signature change on `resolve_mcp_tools()`**: Accept an additional parameter `allowed_actions: &[String]` representing the skill's allowed action types.

6. **Filtering logic**: After `registry.resolve_for_skill(manifest)` returns entries but before querying MCP handles, filter entries by action type:
   - If `allowed_actions` is empty, include all entries (no restriction).
   - If `allowed_actions` is non-empty:
     - Include entries where `action_type` is `None` (unrestricted tools always pass).
     - Include entries where `action_type` is `Some(t)` and `t` is in `allowed_actions`.
     - Exclude entries where `action_type` is `Some(t)` and `t` is not in `allowed_actions`.

7. **Updated call sites**: All callers of `resolve_mcp_tools()` must pass the allowed actions slice. Currently the only caller is `resolve_tools()` in `provider.rs`, which must pass `&manifest.constraints.allowed_actions`.

8. **Updated `ToolEntry` construction in `main.rs`**: The `register_tool_endpoints()` function must include `action_type: None` in the `ToolEntry` struct literal. Optionally, extend the `TOOL_ENDPOINTS` env var format to support action types (e.g., `name=endpoint:action_type`), but this is not required for this task — `None` is a safe default since unrestricted tools always pass the filter.

## Implementation Details

### File: `crates/tool-registry/src/tool_entry.rs`

- Add field to the struct:
  ```rust
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub action_type: Option<String>,
  ```
- Update `Clone` impl to include `action_type: self.action_type.clone()`.
- Update `PartialEq` impl to include `&& self.action_type == other.action_type`.

### File: `crates/agent-runtime/src/tool_bridge.rs`

- Change `resolve_mcp_tools()` signature:
  ```rust
  pub async fn resolve_mcp_tools(
      registry: &ToolRegistry,
      manifest: &SkillManifest,
      allowed_actions: &[String],
  ) -> Result<Vec<McpTool>, RegistryError>
  ```
- After `let entries = registry.resolve_for_skill(manifest)?;`, add a filtering step:
  ```rust
  let entries: Vec<ToolEntry> = if allowed_actions.is_empty() {
      entries
  } else {
      entries
          .into_iter()
          .filter(|entry| match &entry.action_type {
              None => true,
              Some(t) => allowed_actions.iter().any(|a| a == t),
          })
          .collect()
  };
  ```
- The rest of the function (MCP handle querying, McpTool creation) remains unchanged.

### File: `crates/agent-runtime/src/provider.rs`

- Update `resolve_tools()` to pass `allowed_actions`:
  ```rust
  async fn resolve_tools(
      registry: &ToolRegistry,
      manifest: &SkillManifest,
  ) -> Result<Vec<rig::tool::rmcp::McpTool>, ProviderError> {
      tool_bridge::resolve_mcp_tools(registry, manifest, &manifest.constraints.allowed_actions)
          .await
          .map_err(|e| ProviderError::ClientBuild(e.to_string()))
  }
  ```

### File: `crates/agent-runtime/src/main.rs`

- Update the `ToolEntry` construction in `register_tool_endpoints()` to include `action_type: None`.

### File: `crates/tool-registry/src/tool_registry.rs`

- Update `resolve_for_skill()` to include `action_type` in the cloned `ToolEntry`:
  ```rust
  .map(|entry| ToolEntry {
      name: entry.name.clone(),
      version: entry.version.clone(),
      endpoint: entry.endpoint.clone(),
      handle: entry.handle.clone(),
      action_type: entry.action_type.clone(),
  })
  ```

### Files that construct `ToolEntry` in tests

- Any test file that constructs `ToolEntry` directly (e.g., `crates/tool-registry/tests/tool_registry_test.rs`) must be updated to include `action_type: None` (or a specific value for filter-testing purposes).

## Dependencies

- **Blocked by**: "Add `ActionDisallowed` variant to `AgentError`" — The `ActionDisallowed` variant must exist on `AgentError` before this task lands. Although this task's filtering logic does not raise `ActionDisallowed` directly (it silently excludes tools), the variant establishes the broader pattern and the HTTP status mapping (403 Forbidden) that downstream tasks depend on. Both tasks ship together in Group 2.
- **Blocking**: "Write tests for allowed_actions filtering" — Tests will exercise the filtering logic added here, verifying inclusion/exclusion behavior for various `action_type` and `allowed_actions` combinations.

## Risks & Edge Cases

1. **All tools filtered out**: If `allowed_actions` is non-empty and every resolved tool has an `action_type` that is not in the list, `resolve_mcp_tools()` returns an empty `Vec<McpTool>`. The agent will have no tools and can only respond with text. This is correct behavior (the skill author intentionally restricted actions), but downstream callers should handle a toolless agent gracefully. No code change is needed here — rig-core supports agents with zero tools.

2. **Case sensitivity**: `action_type` matching is case-sensitive string comparison. The skill manifest's `allowed_actions` and the tool entry's `action_type` must use the same casing. Document this as a convention (e.g., lowercase: `"read"`, `"write"`, `"query"`).

3. **Backward compatibility of `ToolEntry` serialization**: The `#[serde(default, skip_serializing_if = "Option::is_none")]` annotation ensures existing JSON without `action_type` deserializes to `None`, and entries with `action_type: None` omit the field from output. No breaking change.

4. **Existing tests that construct `ToolEntry`**: The struct literal gains a new field. All existing test files and production code that construct `ToolEntry` must be updated. The compiler will catch missing fields, so this is a compile-time error, not a runtime risk.

5. **`TOOL_ENDPOINTS` env var format**: This task sets `action_type: None` for all entries parsed from the env var. A future task could extend the format to support `name=endpoint@action_type`, but that is out of scope here.

6. **Performance**: The filter is applied to a small in-memory `Vec<ToolEntry>` (typically single digits). No performance concern.

## Verification

1. **Compilation**: `cargo check` succeeds across the entire workspace with no errors.
2. **Lint**: `cargo clippy` passes with no warnings.
3. **Existing tests**: `cargo test` passes — all existing tests still work after updating `ToolEntry` construction sites.
4. **Manual inspection**: Confirm that `resolve_mcp_tools()` accepts the new `allowed_actions` parameter and that the filtering logic matches the specification (empty = no filter, `None` action_type = always included, `Some(t)` = included only if `t` is in `allowed_actions`).
5. **Downstream readiness**: The signature change is reflected in `provider.rs` and the `ToolEntry` struct literal in `main.rs` includes `action_type: None`.
