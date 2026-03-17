# Spec: Integrate validation into SkillLoader::load()

> From: .claude/tasks/issue-6.md

## Objective

Wire the `validate` function into the `SkillLoader::load()` pipeline so that every skill loaded from disk is automatically validated before being returned to the caller. This ensures invalid skill files cause failures at load time (startup), not at runtime. The `SkillLoader` struct must gain access to a `tool_checker: Box<dyn ToolExists>` so that tool-name validation can be performed during `validate`. This task modifies the existing `SkillLoader` struct and its constructor, and adds a single `validate` call into the `load` method.

## Current State

- `crates/skill-loader/src/lib.rs` is currently a placeholder containing only a trivial `add()` function and test. Issue #5 will replace this with the real `SkillLoader` struct and `load` method.
- Per the issue #5 spec (`implement-skill-loader-struct-and-load-method.md`), the post-issue-5 state of `SkillLoader` will be:
  ```rust
  pub struct SkillLoader {
      skill_dir: PathBuf,
      tool_registry: Arc<ToolRegistry>,
  }
  ```
  with `new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>) -> Self` and `pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>`.
- The `load` method (issue #5) reads a `.md` file, extracts frontmatter, deserializes YAML into `SkillFrontmatter`, constructs a `SkillManifest`, and returns it -- with no validation step.
- `SkillError` (issue #5, `crates/skill-loader/src/error.rs`) already has a `ValidationError { skill_name: String, reasons: Vec<String> }` variant, designed for this task.
- Issue #6 Group 2 defines the `ToolExists` trait (either in `lib.rs` or a dedicated `validation.rs` module): `pub trait ToolExists { fn tool_exists(&self, name: &str) -> bool; }`, along with a `struct AllToolsExist;` stub that always returns `true`.
- Issue #6 Group 3 defines the `validate` function: `pub fn validate(manifest: &SkillManifest, tool_checker: &dyn ToolExists) -> Result<(), SkillError>`, which collects all violations and returns `SkillError::ValidationError` if any are found.
- `crates/tool-registry/src/lib.rs` is a placeholder. `ToolRegistry` may or may not exist as a real type by the time this task runs. The `ToolExists` trait decouples validation from the concrete registry.
- `crates/agent-sdk/src/skill_manifest.rs` defines `SkillManifest` with fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec<String>), `constraints` (Constraints), `output` (OutputSchema).

## Requirements

1. **Add a `tool_checker` field to `SkillLoader`:** The struct must store a `Box<dyn ToolExists>` field named `tool_checker`. This is preferred over `Arc<dyn ToolExists>` because `SkillLoader` owns the checker and does not need to share it across threads -- `&self` borrows suffice for the `load` method. If `SkillLoader` needs to be `Send + Sync` (for use in async contexts with tokio), the bound must be `Box<dyn ToolExists + Send + Sync>`.

2. **Update `SkillLoader::new()` signature:** Add `tool_checker: Box<dyn ToolExists>` (or `Box<dyn ToolExists + Send + Sync>`) as a third parameter. The constructor stores it alongside `skill_dir` and `tool_registry`.

3. **Call `validate` in `load()` before returning:** After the `SkillManifest` is successfully constructed from the deserialized frontmatter and body (step (e) in the issue #5 spec), insert a call: `validate(&manifest, &*self.tool_checker)?;`. This uses the `?` operator to propagate any `SkillError::ValidationError` to the caller. Only if validation passes does `load` return `Ok(manifest)`.

4. **Preserve existing `load()` error behavior:** The `IoError` and `ParseError` paths from issue #5 must remain unchanged. Validation errors are a new, additional failure mode that occurs after successful parsing.

5. **Import the `validate` function and `ToolExists` trait:** If `validate` and `ToolExists` live in a `validation` submodule, add the appropriate `use` or module-path references. If they live directly in `lib.rs`, no additional imports are needed.

6. **Re-export `ToolExists` from the crate root:** Downstream crates (e.g., the orchestrator, integration tests) need to implement or use `ToolExists`. Ensure `pub use` makes it available as `skill_loader::ToolExists`. Also re-export `AllToolsExist` for test convenience.

7. **Retain the `tool_registry` field:** Even though validation now uses `tool_checker` (the trait object), the `tool_registry: Arc<ToolRegistry>` field from issue #5 should be retained for now. It may be used by future functionality, or it may be the concrete type behind the `ToolExists` trait once `tool-registry` is implemented (issue #8). Removing it is a separate refactoring decision. However, if the `ToolExists` trait fully replaces the need for a direct `ToolRegistry` reference, the `tool_registry` field and the `tool-registry` dependency MAY be removed in this task -- but only if it simplifies the code without breaking other specs. Prefer the conservative approach: keep both fields.

## Implementation Details

### File to modify: `crates/skill-loader/src/lib.rs`

**Struct change:**
```rust
pub struct SkillLoader {
    skill_dir: PathBuf,
    tool_registry: Arc<ToolRegistry>,
    tool_checker: Box<dyn ToolExists + Send + Sync>,
}
```

**Constructor change:**
```rust
pub fn new(
    skill_dir: PathBuf,
    tool_registry: Arc<ToolRegistry>,
    tool_checker: Box<dyn ToolExists + Send + Sync>,
) -> Self {
    Self {
        skill_dir,
        tool_registry,
        tool_checker,
    }
}
```

**`load` method change -- add validation call after manifest construction:**

The existing `load` method (from issue #5) ends with constructing `SkillManifest` and returning `Ok(manifest)`. Insert the validation call between construction and return:

```rust
pub async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError> {
    let path = self.skill_dir.join(format!("{skill_name}.md"));
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|err| SkillError::IoError {
            path: path.clone(),
            source: err.to_string(),
        })?;
    let (yaml, body) = frontmatter::extract_frontmatter(&content)?;
    let fm: frontmatter::SkillFrontmatter =
        serde_yaml::from_str(yaml).map_err(|err| SkillError::ParseError {
            path: path.clone(),
            source: err.to_string(),
        })?;
    let manifest = SkillManifest {
        name: fm.name,
        version: fm.version,
        description: fm.description,
        model: fm.model,
        preamble: body.trim().to_string(),
        tools: fm.tools,
        constraints: fm.constraints,
        output: fm.output,
    };

    // NEW: validate before returning
    validate(&manifest, &*self.tool_checker)?;

    Ok(manifest)
}
```

**Imports / module wiring:**

If `validate` and `ToolExists` live in `crates/skill-loader/src/validation.rs`:
```rust
mod validation;
pub use validation::{validate, ToolExists, AllToolsExist};
```

If they are defined directly in `lib.rs`, no module declaration is needed -- just ensure the function and trait are `pub`.

### No new files created by this task

This task only modifies `crates/skill-loader/src/lib.rs`. The `validation.rs` module (containing `ToolExists`, `AllToolsExist`, and `validate`) is created by the preceding tasks in issue #6 Groups 2 and 3.

### Integration points

- **Callers of `SkillLoader::new`** must now provide a third argument. This includes:
  - Integration tests in `crates/skill-loader/tests/skill_loader_test.rs` (from issue #5 Group 4) -- these must be updated to pass `Box::new(AllToolsExist)` as the tool checker.
  - Any future orchestrator code that constructs a `SkillLoader`.
- **`validate` function** is called with a borrowed reference to the manifest and the tool checker. It returns `Result<(), SkillError>`, and the `?` operator propagates `ValidationError` to the caller of `load`.
- **`SkillError::ValidationError`** is now a reachable error variant from `load()`. Callers that match on `SkillError` must handle this variant (they already should, since the enum is exhaustive and was defined in issue #5).

## Dependencies

- **Blocked by:**
  - "Implement validate function" (issue #6, Group 3) -- the `validate` function and `ToolExists` trait must exist before this task can call them.
  - Issue #5 completion -- the `SkillLoader` struct, `load` method, `SkillError` enum, frontmatter parsing, and all dependencies must be in place. This task modifies code that issue #5 creates.
- **Blocking:**
  - "Write integration test for load-with-validation" (issue #6, Group 5) -- tests that exercise the validation-in-load path depend on this integration being wired up.

## Risks & Edge Cases

1. **Issue #5 not yet landed.** This task directly modifies code produced by issue #5. If issue #5 is incomplete, this task cannot proceed. Mitigation: the task description explicitly says to defer if issue #5 is not yet landed. Check that `SkillLoader`, `load`, `SkillError`, and frontmatter modules exist before starting.

2. **Breaking existing issue #5 tests.** Adding a third parameter to `SkillLoader::new` will break any existing call sites, including integration tests from issue #5. Mitigation: update those call sites to pass `Box::new(AllToolsExist)` so existing tests continue to pass without exercising validation. This preserves backward compatibility of test behavior while enabling the new validation path.

3. **`Send + Sync` bounds on `Box<dyn ToolExists>`.** If `SkillLoader` is used inside a `tokio::spawn` or across `.await` points, the struct must be `Send + Sync`. Since `Box<dyn ToolExists>` is not `Send + Sync` by default, the bound must be explicitly specified as `Box<dyn ToolExists + Send + Sync>`. Forgetting this will cause compile errors in async contexts. Mitigation: always use the `+ Send + Sync` bound.

4. **Validation ordering.** Validation runs after successful parsing. If the YAML is malformed, `ParseError` is returned before `validate` is ever called. If the YAML is valid but semantically wrong (e.g., `confidence_threshold: 2.0`), `ValidationError` is returned. This ordering is intentional: there is no point validating semantics on data that could not be parsed.

5. **Performance.** `validate` is a synchronous function called on every `load`. Since it performs only in-memory checks (string emptiness, range checks, hash-set lookups for tool names), the overhead is negligible. No mitigation needed.

6. **`tool_registry` field may become redundant.** If `ToolExists` fully replaces the need for `Arc<ToolRegistry>`, the `tool_registry` field is dead code. This creates a clippy warning for unused fields. Mitigation: if `tool_registry` is truly unused after this change, either (a) add `#[allow(dead_code)]` with a comment referencing issue #8, or (b) remove the field and the `tool-registry` dependency. The conservative approach is (a).

7. **Dereference syntax for `Box<dyn ToolExists>`.** The call `validate(&manifest, &*self.tool_checker)` uses `&*` to go from `Box<dyn ToolExists>` to `&dyn ToolExists`. Alternatively, `self.tool_checker.as_ref()` or relying on Deref coercion (just `&self.tool_checker`) may work. Use whichever is clearest; `&*self.tool_checker` is explicit and idiomatic.

## Verification

1. **Compile check:** `cargo check -p skill-loader` succeeds with no errors.
2. **Lint check:** `cargo clippy -p skill-loader` succeeds with no warnings (except possibly the `tool_registry` dead-code warning, which should be addressed per Risk #6).
3. **Existing tests pass:** `cargo test -p skill-loader` passes. All issue #5 tests must still pass after updating `SkillLoader::new` call sites to include the `tool_checker` parameter.
4. **Struct inspection:** `SkillLoader` has three fields: `skill_dir: PathBuf`, `tool_registry: Arc<ToolRegistry>`, `tool_checker: Box<dyn ToolExists + Send + Sync>`.
5. **Constructor inspection:** `SkillLoader::new` accepts three parameters: `PathBuf`, `Arc<ToolRegistry>`, `Box<dyn ToolExists + Send + Sync>`.
6. **`load` method inspection:** The method calls `validate(&manifest, &*self.tool_checker)?` after constructing the manifest and before returning `Ok(manifest)`.
7. **Re-exports inspection:** `skill_loader::ToolExists` and `skill_loader::AllToolsExist` are publicly accessible from external crates.
8. **Manual smoke test:** Create a temporary skill file with `confidence_threshold: 2.0` (out of range), call `SkillLoader::load`, and confirm it returns `SkillError::ValidationError` with a reason mentioning the threshold. (This is formalized in the "Write integration test for load-with-validation" task.)
