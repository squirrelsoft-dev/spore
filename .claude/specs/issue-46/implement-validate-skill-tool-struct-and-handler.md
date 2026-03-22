# Spec: Implement `ValidateSkillTool` struct and handler

> From: .claude/tasks/issue-46.md

## Objective
Create the core MCP tool implementation for the `validate-skill` crate. The tool accepts a full skill file content string (markdown with YAML frontmatter), parses and validates it using the `skill-loader` crate, and returns a structured JSON response indicating whether the skill is valid along with any errors or the parsed manifest.

## Current State
- The `echo-tool` crate (`tools/echo-tool/src/echo.rs`) establishes the canonical pattern for MCP tool structs: a request struct with `#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]`, a tool struct holding `ToolRouter<Self>`, a `#[tool_router]` impl block with the tool method, and a `#[tool_handler]` impl for `ServerHandler`.
- The `skill-loader` crate (`crates/skill-loader/`) currently exposes `SkillError`, `ToolExists`, `AllToolsExist`, and `validate` publicly. The `extract_frontmatter` function and `SkillFrontmatter` struct are `pub(crate)` -- not available to external consumers.
- The `skill-loader` crate's `load` method on `SkillLoader` reads from the filesystem and requires a `ToolRegistry`. A standalone `parse_content` function that takes a `&str` and returns a `SkillManifest` does not yet exist.
- The companion task "Expose a public `parse_content` function in skill-loader" must land first to provide `skill_loader::parse_content(&str) -> Result<SkillManifest, SkillError>`.
- The companion task "Create `tools/validate-skill/Cargo.toml`" must land first to provide the crate scaffolding with dependencies on `rmcp`, `serde`, `serde_json`, `schemars`, `skill-loader`, and `agent-sdk`.
- `agent_sdk::SkillManifest` implements `Serialize` (used in the success response to include the manifest in the JSON output).
- `skill_loader::validate` accepts `&SkillManifest` and `&dyn ToolExists`, returning `Result<(), SkillError>`.
- `skill_loader::AllToolsExist` is a stub that always returns `true` for tool existence checks.
- `SkillError::ValidationError` contains a `reasons: Vec<String>` field with human-readable validation failure messages.

## Requirements
1. Define `ValidateSkillRequest` with a single field `content: String` and derive `Debug`, `serde::Deserialize`, `schemars::JsonSchema`. The `content` field must have the doc comment `/// The full skill file content (markdown with YAML frontmatter)`.
2. Define `ValidateSkillTool` struct with a `tool_router: ToolRouter<Self>` field and a `new()` constructor that calls `Self::tool_router()`.
3. The `validate_skill` method must be annotated with `#[tool]` and accept `Parameters<ValidateSkillRequest>`.
4. The method must return a JSON string (via `serde_json`) with the following behavior:
   - Call `skill_loader::parse_content(&request.content)` to parse frontmatter into a `SkillManifest`.
   - If parsing fails, return `{ "valid": false, "errors": ["<error_message>"] }`.
   - If parsing succeeds, call `skill_loader::validate(&manifest, &AllToolsExist)`.
   - If validation fails, return `{ "valid": false, "errors": ["<reason1>", "<reason2>", ...] }` using the `reasons` from `SkillError::ValidationError`.
   - If everything passes, return `{ "valid": true, "errors": [], "manifest": <manifest> }`.
5. Implement `ServerHandler` for `ValidateSkillTool` with `#[tool_handler]`, advertising tool capabilities only.
6. Include inline `#[cfg(test)] mod tests` with unit tests.
7. Each function must be no more than 50 lines. Break larger logic into helpers if needed.

## Implementation Details

### File: `tools/validate-skill/src/validate_skill.rs`

#### Request struct

```rust
#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ValidateSkillRequest {
    /// The full skill file content (markdown with YAML frontmatter)
    pub content: String,
}
```

#### Tool struct and constructor

```rust
#[derive(Debug, Clone)]
pub struct ValidateSkillTool {
    tool_router: ToolRouter<Self>,
}

impl ValidateSkillTool {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}
```

Follow the same `#[derive(Debug, Clone)]` pattern as `EchoTool`.

#### Tool router impl

```rust
#[tool_router]
impl ValidateSkillTool {
    #[tool(description = "Validates a skill file's frontmatter and content")]
    fn validate_skill(
        &self,
        Parameters(request): Parameters<ValidateSkillRequest>,
    ) -> String {
        // Step 1: parse
        // Step 2: validate
        // Step 3: build JSON response
    }
}
```

The method should use a helper to build the JSON response strings, keeping the main method concise.

#### Response construction helpers

Define two private helper functions (or a single helper with an enum) to build the JSON responses:

- `fn error_response(errors: Vec<String>) -> String` -- serializes `{ "valid": false, "errors": [...] }` using `serde_json::json!`.
- `fn success_response(manifest: &SkillManifest) -> String` -- serializes `{ "valid": true, "errors": [], "manifest": ... }` using `serde_json::json!`.

These helpers keep the tool method body under 50 lines and separate concerns.

#### Error extraction logic

When `skill_loader::parse_content` fails, convert the `SkillError` to a single-element error array containing the error's `Display` string (`err.to_string()`).

When `skill_loader::validate` fails with `SkillError::ValidationError { reasons, .. }`, use the `reasons` vector directly. For any other `SkillError` variant from `validate`, fall back to `vec![err.to_string()]`.

#### ServerHandler impl

```rust
#[tool_handler]
impl ServerHandler for ValidateSkillTool {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
    }
}
```

Identical pattern to `EchoTool`.

#### Required imports

```rust
use rmcp::{
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router, ServerHandler,
};
use skill_loader::{AllToolsExist, SkillError};
```

The `serde_json::json!` macro is used for building response objects. `agent_sdk::SkillManifest` is referenced indirectly through the return of `parse_content`.

### Unit tests

Include a `#[cfg(test)] mod tests` block covering at minimum:

1. **Valid skill content** -- provide a complete, well-formed markdown string with valid YAML frontmatter. Assert the response contains `"valid": true` and includes a `"manifest"` key.
2. **Missing frontmatter delimiter** -- provide content without `---`. Assert the response contains `"valid": false` and `"errors"` is non-empty.
3. **Invalid YAML in frontmatter** -- provide content with `---` delimiters but malformed YAML. Assert `"valid": false`.
4. **Valid frontmatter but failing validation** -- provide content where frontmatter parses but has an empty `name` or out-of-range `confidence_threshold`. Assert `"valid": false` with specific error messages in the `"errors"` array.
5. **Empty content string** -- assert `"valid": false`.

Tests should instantiate `ValidateSkillTool::new()` and call `validate_skill` directly with `Parameters(ValidateSkillRequest { content: ... })`, then deserialize the returned JSON string with `serde_json::from_str::<serde_json::Value>` to make assertions.

### Line budget

Estimated line counts:
- Imports: ~8 lines
- `ValidateSkillRequest` struct: ~5 lines
- `ValidateSkillTool` struct + `new()`: ~10 lines
- Helper functions: ~15 lines
- `#[tool_router]` impl with `validate_skill`: ~20 lines
- `#[tool_handler]` impl: ~8 lines
- Tests: ~80 lines
- Total: ~146 lines

All individual functions stay within the 50-line limit.

## Dependencies
- Blocked by: "Create `tools/validate-skill/Cargo.toml`", "Expose a public `parse_content` function in skill-loader"
- Blocking: "Write `main.rs`", "Write integration test"

## Risks & Edge Cases

1. **`parse_content` API not yet available:** This task depends on a new public function `skill_loader::parse_content` that does not exist yet. The blocked-by task must land first. If the function signature differs from `fn parse_content(content: &str) -> Result<SkillManifest, SkillError>`, adjust the call site accordingly.

2. **`SkillManifest` serialization:** The success response includes the full manifest as JSON. This requires `SkillManifest` to implement `serde::Serialize`. Verify that `agent_sdk::SkillManifest` derives `Serialize` -- if not, the success response must omit the manifest or this becomes a new dependency.

3. **`SkillError` variant matching:** The `validate` function returns `SkillError::ValidationError { skill_name, reasons }` on failure. The code must pattern-match on this variant to extract `reasons`. Other `SkillError` variants (e.g., `ParseError`, `IoError`) should be handled via the `Display` fallback.

4. **`AllToolsExist` as default checker:** Using `AllToolsExist` means the validate-skill tool will not catch references to nonexistent tools. This is an intentional design choice for the initial implementation -- the tool validates structural correctness only. A future enhancement could accept a tool registry or tool list parameter.

5. **JSON serialization failure:** `serde_json::json!` and `serde_json::to_string` are infallible for basic types but could theoretically fail if `SkillManifest` contains non-serializable data. The success helper should handle `serde_json` errors gracefully by falling back to an error response rather than panicking.

6. **Large content strings:** No size limit is enforced on the `content` field. Extremely large inputs could cause performance issues during YAML parsing. This is acceptable for the initial implementation; a size guard can be added later.

## Verification

1. **Compilation:** `cargo check -p validate-skill` succeeds with no errors.
2. **Lint:** `cargo clippy -p validate-skill` produces no warnings.
3. **Unit tests:** `cargo test -p validate-skill` passes all tests.
4. **Line count:** No function in `validate_skill.rs` exceeds 50 lines (`cargo clippy` or manual review).
5. **Workspace tests:** `cargo test` across the full workspace still passes (no regressions).
