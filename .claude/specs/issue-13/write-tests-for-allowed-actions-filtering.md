# Spec: Write tests for allowed_actions filtering

> From: .claude/tasks/issue-13.md

## Objective

Add tests that verify the `allowed_actions` filtering logic introduced by the "Filter tools by `allowed_actions` in tool resolution" task. The filtering happens in `resolve_mcp_tools()` (in `tool_bridge.rs`), which accepts `allowed_actions` and excludes `ToolEntry` instances whose `action_type` is not in the allowed list. These tests ensure that:

1. Tools with a matching `action_type` are included.
2. Tools with a non-matching `action_type` are excluded.
3. Tools with `action_type: None` are always included (no restriction).
4. When `allowed_actions` is empty, all tools pass through regardless of `action_type`.

## Current State

### `crates/tool-registry/src/tool_entry.rs`

`ToolEntry` currently has three fields: `name`, `version`, `endpoint`, plus a `handle: Option<McpHandle>` (skipped in serde). The "Filter tools by `allowed_actions`" task will add `action_type: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

### `crates/agent-runtime/src/tool_bridge.rs`

`resolve_mcp_tools()` currently takes `&ToolRegistry` and `&SkillManifest`, calls `registry.resolve_for_skill(manifest)` to get entries, then iterates entries with MCP handles to list and wrap tools. The "Filter tools by `allowed_actions`" task will modify this function to also accept `&[String]` for allowed_actions and filter entries before querying MCP handles.

### `crates/tool-registry/tests/tool_registry_test.rs`

Contains synchronous tests for `ToolRegistry` operations: register, get, assert_exists, resolve_for_skill, duplicate detection, and the `ToolExists` trait. Uses helper functions `make_entry(name)` and `make_manifest(tools)` to construct test fixtures. The `make_manifest` function builds a `SkillManifest` with `allowed_actions: vec![]`.

### `crates/tool-registry/tests/mcp_connection_test.rs`

Contains async tests using a `MockServer` (implementing `ServerHandler`) that starts a real TCP/Unix MCP server. Provides helpers `start_tcp_server`, `make_registry_with_entry`, and `json_object_schema`. Tests verify connect, list_tools, and call_tool through live MCP connections.

### Test strategy decision

The `allowed_actions` filtering has two layers:

1. **Entry-level filtering** (in `tool_bridge.rs`): filtering `Vec<ToolEntry>` by `action_type` against `allowed_actions`. This is the core logic being tested.
2. **MCP tool resolution** (in `tool_bridge.rs`): the filtered entries are then used to query MCP handles for actual `McpTool` objects.

Testing layer 1 in isolation is the most valuable and reliable approach. Since the filtering modifies the entries before MCP handle queries, we can test it by:
- Placing pure filtering tests in `crates/tool-registry/tests/tool_registry_test.rs` if the filtering is exposed as a registry method, OR
- Placing integration tests in `crates/agent-runtime/tests/tool_bridge_test.rs` using the MockServer pattern from `mcp_connection_test.rs`.

The task description says the file is `crates/tool-registry/tests/tool_registry_test.rs` **or** `crates/agent-runtime/tests/tool_bridge_test.rs`. Choose based on where the filtering logic lands:

- If `resolve_mcp_tools()` performs the filtering inline, tests go in `crates/agent-runtime/tests/tool_bridge_test.rs` using the mock MCP server pattern.
- If the filtering is factored into a method on `ToolRegistry` (e.g., `resolve_for_skill_filtered`), pure synchronous tests can go in `crates/tool-registry/tests/tool_registry_test.rs`.

Either way, the test cases are the same. The implementation below covers both locations.

## Requirements

1. **Test: non-matching `action_type` is excluded** -- Register three tools: `reader` with `action_type: Some("read")`, `writer` with `action_type: Some("write")`, `querier` with `action_type: Some("query")`. Call the filtering logic with `allowed_actions: ["read", "query"]`. Assert that `writer` is excluded and `reader` and `querier` are included.

2. **Test: matching `action_type` is included** -- Covered by requirement 1 (reader and querier should be present in results).

3. **Test: `action_type: None` is always included** -- Register a tool with `action_type: None` alongside tools with `action_type: Some("write")`. Call with `allowed_actions: ["read"]`. Assert the `None`-typed tool is included and the write-typed tool is excluded.

4. **Test: empty `allowed_actions` passes all tools** -- Register tools with `action_type: Some("read")`, `Some("write")`, and `None`. Call with `allowed_actions: []` (empty). Assert all three tools are included.

5. **Test: all tools excluded when none match** -- Register tools with `action_type: Some("write")` and `Some("admin")`. Call with `allowed_actions: ["read"]`. Assert zero tools are returned (only tools whose `action_type` is `Some` and not in the list are excluded; but here all have `Some` types that don't match).

6. All existing tests continue to pass (the new `action_type` field defaults to `None`, so existing `ToolEntry` constructions remain valid).

## Implementation Details

### Option A: `crates/tool-registry/tests/tool_registry_test.rs` (preferred if filtering is on the registry)

**Modify `make_entry` helper** to optionally accept `action_type`:

- Add a `make_entry_with_action(name: &str, action_type: Option<&str>) -> ToolEntry` helper that constructs a `ToolEntry` with the `action_type` field set.
- The existing `make_entry(name)` helper should continue to work (producing `action_type: None`), either by keeping it as-is or delegating to the new helper.

**Modify `make_manifest` helper** to accept `allowed_actions`:

- Add a `make_manifest_with_actions(tools: Vec<String>, allowed_actions: Vec<String>) -> SkillManifest` helper, or modify the existing `make_manifest` to accept an optional `allowed_actions` parameter.

**Add test functions:**

- `fn allowed_actions_excludes_non_matching_action_type()` -- Tests requirement 1.
- `fn allowed_actions_includes_tools_with_no_action_type()` -- Tests requirement 3.
- `fn empty_allowed_actions_passes_all_tools()` -- Tests requirement 4.
- `fn allowed_actions_excludes_all_when_none_match()` -- Tests requirement 5.

Each test follows the existing pattern: construct a `ToolRegistry`, register entries, build a manifest, call the filtering function, and assert on the returned `Vec<ToolEntry>`.

### Option B: `crates/agent-runtime/tests/tool_bridge_test.rs` (if filtering is inline in `resolve_mcp_tools`)

This requires the MockServer pattern from `mcp_connection_test.rs` since `resolve_mcp_tools` queries MCP handles. Tests would:

1. Start mock MCP servers advertising tools with various names.
2. Register `ToolEntry` instances with different `action_type` values.
3. Connect to the mock servers.
4. Call `resolve_mcp_tools()` with a manifest containing the appropriate `allowed_actions`.
5. Assert on the resulting `Vec<McpTool>` (checking tool names via the `ToolDyn` trait's `definition().name`).

These are `#[tokio::test(flavor = "multi_thread", worker_threads = 2)]` tests, following the pattern in `mcp_connection_test.rs`.

### Key types and interfaces (post-prerequisite task)

- `ToolEntry` gains: `pub action_type: Option<String>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`
- `resolve_mcp_tools()` signature changes to accept allowed_actions (either as a separate parameter or via the manifest's `constraints.allowed_actions`)
- Filtering logic: if `allowed_actions` is non-empty, exclude entries where `entry.action_type` is `Some(t)` and `t` is not in `allowed_actions`. If `action_type` is `None`, include the entry.

### No new dependencies required.

## Dependencies

- Blocked by: "Filter tools by `allowed_actions` in tool resolution" (the filtering logic and `action_type` field must exist before tests can be written)
- Blocking: "Run verification suite"

## Risks & Edge Cases

- **`action_type` field missing from existing `ToolEntry` constructions**: The new field uses `#[serde(default)]` and `Option<String>`, so existing code constructing `ToolEntry` with struct literal syntax will get a compile error unless the field is added. All test helpers (`make_entry`, `make_registry_with_entry`) must be updated to include `action_type: None`. This is a straightforward mechanical change.
- **Case sensitivity of `action_type` matching**: The spec assumes exact string matching (e.g., `"read"` matches `"read"`, not `"Read"`). Tests should use consistent casing. If the implementation normalizes case, tests should verify that behavior explicitly.
- **Mock MCP server tool names vs. ToolEntry names**: In the integration test approach (Option B), the MockServer advertises tool names at the MCP protocol level, while `ToolEntry.name` is the registry-level name. These are different namespaces. The filtering applies to `ToolEntry.action_type`, not to individual MCP tool names. Tests must be careful to assert on the correct layer.
- **Empty results**: When all tools are filtered out, `resolve_mcp_tools` should return `Ok(vec![])`, not an error. The "all excluded" test case verifies this.
- **Interaction with `resolve_for_skill`**: The existing `resolve_for_skill` checks that all tools in the manifest exist in the registry and errors on missing tools. The `allowed_actions` filter runs after this check, so it cannot cause `ToolNotFound` errors. Tests should not conflate these two concerns.

## Verification

1. `cargo check --workspace` compiles cleanly after the prerequisite task and this test task are both complete.
2. `cargo test --workspace` passes all tests, including:
   - All four (or more) new `allowed_actions` filtering tests.
   - All existing tests in `tool_registry_test.rs`, `tool_entry_test.rs`, and `mcp_connection_test.rs`.
3. `cargo clippy --workspace` produces no new warnings.
4. Each test case exercises a distinct filtering scenario (matching, non-matching, None, empty allowed_actions) so that a regression in any single filtering branch is caught.
