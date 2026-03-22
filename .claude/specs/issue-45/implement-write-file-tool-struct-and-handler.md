# Spec: Implement `WriteFileTool` struct and handler

> From: .claude/tasks/issue-45.md

## Objective

Create `tools/write-file/src/write_file.rs` following the established `echo.rs` pattern from the echo-tool crate. The file implements a `WriteFileTool` MCP tool that writes string content to a file on disk, creating parent directories as needed, and returns a confirmation message on success or a descriptive error string on failure.

## Current State

No `tools/write-file/` crate exists yet. The echo-tool crate at `tools/echo-tool/src/echo.rs` serves as the reference implementation for how MCP tools are structured in this project. It demonstrates the use of `ToolRouter`, `#[tool_router]`, `#[tool_handler]`, `ServerHandler`, `Parameters`, `ServerCapabilities`, and `ServerInfo` from the `rmcp` crate.

## Requirements

1. **`WriteFileRequest` struct** -- Derive `Debug`, `serde::Deserialize`, and `schemars::JsonSchema`. Two fields:
   - `path: String` -- The file path to write to. Include a doc comment (e.g., `/// The file path to write to`).
   - `content: String` -- The content to write. Include a doc comment (e.g., `/// The content to write to the file`).

2. **`WriteFileTool` struct** -- Derive `Debug`, `Clone`. Single field `tool_router: ToolRouter<Self>`. Provide a `new()` constructor that calls `Self::tool_router()` (identical pattern to `EchoTool::new()`).

3. **`#[tool_router]` impl block** -- Contains a single method:
   - Name: `write_file`
   - Attribute: `#[tool(description = "Write content to a file on disk, creating parent directories as needed")]`
   - Signature: `fn write_file(&self, Parameters(request): Parameters<WriteFileRequest>) -> String`
   - Behavior:
     - Call a helper `validate_write_path(&request.path)` that returns `Err(String)` if the path is empty. Propagate the error string as the return value if validation fails.
     - Extract the parent directory from the path using `std::path::Path::new(&request.path).parent()`.
     - Call `std::fs::create_dir_all` on the parent directory. On failure, return a descriptive error string including the underlying IO error.
     - Call `std::fs::write(&request.path, &request.content)`. On failure, return a descriptive error string including the underlying IO error (covers permission denied, disk full, invalid path, etc.).
     - On success, return `format!("Wrote {} bytes to {}", request.content.len(), request.path)`.

4. **Helper function `validate_write_path`** -- A standalone function (not a method) that takes `path: &str` and returns `Result<(), String>`. Returns `Err` with a descriptive message (e.g., `"Path must not be empty"`) when the path is empty, `Ok(())` otherwise. This keeps the `write_file` method under 50 lines.

5. **`#[tool_handler]` impl of `ServerHandler`** -- Identical to the echo-tool pattern:
   - `get_info()` returns `ServerInfo::new(ServerCapabilities::builder().enable_tools().build())`.

## Implementation Details

- **Imports**: Mirror the echo-tool imports from `rmcp`, adding `use std::path::Path;` and `use std::fs;` as needed.
- **No `#[cfg(test)]` module in this file** -- Tests are covered by a separate task/spec ("Write unit tests").
- **Error formatting**: Use `format!` to include the IO error via its `Display` impl (e.g., `format!("Failed to create directory '{}': {}", dir.display(), e)`).
- **Parent directory edge case**: If `Path::new(path).parent()` returns `None` or an empty path (e.g., path is just a filename like `"foo.txt"`), skip the `create_dir_all` step since there is no parent directory to create. Alternatively, `create_dir_all("")` is a no-op, but explicitly checking is clearer.
- **Byte count**: Use `request.content.len()` which returns the byte length of the UTF-8 string, consistent with what `std::fs::write` writes.

## Dependencies

- **Blocked by**: "Add `tools/write-file` to workspace members" -- The `tools/write-file` crate with its `Cargo.toml` (declaring the `rmcp`, `serde`, and `schemars` dependencies) must exist and be added to the workspace before this file can compile.
- **Blocking**: "Write unit tests" -- Unit tests for `write_file`, `validate_write_path`, and error paths depend on this implementation being complete.

## Risks & Edge Cases

- **Empty path**: Handled by `validate_write_path` returning an error before any filesystem operations.
- **Path is a directory**: `std::fs::write` will return an IO error (e.g., "Is a directory") which gets surfaced in the error string.
- **Permission denied / read-only filesystem**: `std::fs::write` or `create_dir_all` returns an IO error; the tool returns it as a descriptive string.
- **Disk full**: `std::fs::write` returns an IO error; surfaced in the return string.
- **Symlink traversal / path traversal**: Not addressed in this task. Security constraints (sandboxing, path allowlists) are out of scope for the initial implementation.
- **Race conditions**: Another process could remove the parent directory between `create_dir_all` and `write`. This is inherent to filesystem operations and acceptable for a tool of this nature.
- **Large content**: No size limit is enforced. The caller is responsible for not sending excessively large payloads.

## Verification

1. `cargo check -p write-file` compiles without errors (requires the workspace member task to be complete first).
2. `cargo clippy -p write-file` reports no warnings.
3. The `write_file` method body is under 50 lines.
4. The `validate_write_path` helper is a standalone function, not a method on `WriteFileTool`.
5. Manual review confirms the file mirrors the `echo.rs` structural pattern: request struct, tool struct with `ToolRouter`, `#[tool_router]` impl, `#[tool_handler]` impl.
