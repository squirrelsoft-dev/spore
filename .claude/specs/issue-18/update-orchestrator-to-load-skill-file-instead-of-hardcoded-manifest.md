# Spec: Update orchestrator to load skill file instead of hardcoded manifest
> From: .claude/tasks/issue-18.md

## Objective

Replace the hardcoded `build_default_manifest()` function in `crates/orchestrator/src/orchestrator.rs` so that the orchestrator loads its `SkillManifest` from the `skills/orchestrator.md` skill file via `SkillLoader`, proving the system's homogeneity principle: the orchestrator is just runtime + skill file, identical to every other agent.

## Current State

- `build_default_manifest()` (line 350 in `orchestrator.rs`) returns a placeholder `SkillManifest` with `provider: "none"`, `name: "none"`, empty preamble, empty tools list, and zero-value constraints.
- `from_config()` (line 65) and `from_config_with_model()` (line 138) both call `build_default_manifest()` internally to construct the manifest, giving the caller no way to supply a real one.
- `Orchestrator::new()` already accepts a `SkillManifest` parameter -- the issue is only in the `from_config` / `from_config_with_model` convenience constructors.
- Tests in `orchestrator_test.rs` use a separate `build_test_manifest()` helper (line 76) that constructs a similar placeholder manifest. These tests call `Orchestrator::new()` directly and are not affected by changes to `from_config`.
- `SkillLoader` (in `crates/skill-loader/src/lib.rs`) provides `async fn load(&self, skill_name: &str) -> Result<SkillManifest, SkillError>`, which reads `{skill_dir}/{skill_name}.md`, parses frontmatter, validates tools, and returns a `SkillManifest`.
- `SkillLoader::new()` requires a `PathBuf` skill directory, an `Arc<ToolRegistry>`, and a `Box<dyn ToolExists + Send + Sync>`.

## Requirements

1. **`from_config` and `from_config_with_model` must accept a `SkillManifest` parameter** instead of calling `build_default_manifest()`. This is the simplest change that decouples manifest construction from orchestrator construction, letting the caller load the manifest however they choose (via `SkillLoader`, from a test fixture, etc.).

2. **Remove `build_default_manifest()`**. Once the callers pass the manifest in, the hardcoded fallback serves no purpose and should be deleted to avoid confusion.

3. **Update `OrchestratorConfig`** (optional). If it makes sense for config-driven construction, add an optional `skill_file` or `skill_dir` field to `OrchestratorConfig`. However, since `SkillLoader` requires async I/O and a `ToolRegistry`, and `OrchestratorConfig` is a plain deserialization struct, the simpler approach is to keep manifest loading external and pass the result in. Do NOT add `SkillManifest` as a field on `OrchestratorConfig` because `SkillManifest` is not `Deserialize`.

4. **Preserve test patterns**. The existing `build_test_manifest()` in `orchestrator_test.rs` is fine as-is. Tests that call `Orchestrator::new()` directly are unaffected. If any tests call `from_config` or `from_config_with_model`, update them to pass a manifest. Currently no tests use these methods (they all use `Orchestrator::new()`), so test changes are expected to be minimal.

5. **Add an `OrchestratorError` variant for skill loading failures**, or convert `SkillError` into the existing `Config` variant, so that errors from `SkillLoader` propagate cleanly through the orchestrator's error type.

## Implementation Details

### Step 1: Update `from_config` signature

Change:
```rust
pub fn from_config(config: OrchestratorConfig) -> Result<Self, OrchestratorError>
```
To:
```rust
pub fn from_config(config: OrchestratorConfig, manifest: SkillManifest) -> Result<Self, OrchestratorError>
```

Remove the `let manifest = build_default_manifest();` line and use the passed-in `manifest` parameter instead.

### Step 2: Update `from_config_with_model` signature

Change:
```rust
pub async fn from_config_with_model<M: EmbeddingModel>(
    config: OrchestratorConfig,
    model: &M,
    similarity_threshold: f64,
) -> Result<Self, OrchestratorError>
```
To:
```rust
pub async fn from_config_with_model<M: EmbeddingModel>(
    config: OrchestratorConfig,
    manifest: SkillManifest,
    model: &M,
    similarity_threshold: f64,
) -> Result<Self, OrchestratorError>
```

Remove the `let manifest = build_default_manifest();` line and use the passed-in `manifest` parameter instead.

### Step 3: Remove `build_default_manifest()`

Delete the entire `build_default_manifest()` function (lines 350-373). It is no longer called.

### Step 4: Update call sites

Search the workspace for any callers of `from_config` or `from_config_with_model` (e.g., in `main.rs` or binary entrypoints) and update them to load the manifest via `SkillLoader` before calling the constructor. The typical call site pattern becomes:

```rust
let loader = SkillLoader::new(skill_dir, tool_registry, tool_checker);
let manifest = loader.load("orchestrator").await?;
let orchestrator = Orchestrator::from_config(config, manifest)?;
```

If no binary entrypoints exist yet (the orchestrator crate is a library), this step is a no-op.

### Step 5: Error bridging

Add a `From<skill_loader::SkillError>` impl for `OrchestratorError`, mapping it to the existing `Config` variant:

```rust
impl From<SkillError> for OrchestratorError {
    fn from(err: SkillError) -> Self {
        OrchestratorError::Config {
            reason: err.to_string(),
        }
    }
}
```

This requires adding `skill-loader` as a dependency in `crates/orchestrator/Cargo.toml`. Evaluate whether this coupling is acceptable -- if the orchestrator will always load its own skill file, the dependency is justified. If not, the error conversion can live at the call site instead, and no new dependency is needed.

### Step 6: Update tests (if needed)

- Verify no tests in `crates/orchestrator/tests/` call `from_config` or `from_config_with_model`. Currently none do -- they all use `Orchestrator::new(build_test_manifest(), ...)`.
- If any are found, update them to pass a test manifest as the new parameter.
- The `build_test_manifest()` helper in `orchestrator_test.rs` remains unchanged.

### Files modified

- `crates/orchestrator/src/orchestrator.rs` -- change `from_config` and `from_config_with_model` signatures, remove `build_default_manifest()`
- `crates/orchestrator/src/config.rs` -- no changes expected unless adding a `skill_dir` field
- `crates/orchestrator/Cargo.toml` -- add `skill-loader` dependency only if error bridging is added in the orchestrator crate
- Any binary entrypoints that call `from_config` or `from_config_with_model`

## Dependencies

- **Blocked by**: "Create `skills/orchestrator.md` routing skill file" -- the skill file must exist before callers can load it via `SkillLoader`. However, the code changes in this task (accepting a `SkillManifest` parameter) can be implemented and compiled without the skill file existing.
- **Uses**: `SkillLoader` from `crates/skill-loader/src/lib.rs` at the call site level. The orchestrator crate itself only needs to accept a `SkillManifest` (defined in `agent-sdk`), which it already depends on.

## Risks & Edge Cases

1. **Breaking change to public API**: `from_config` and `from_config_with_model` are public methods. Any external callers will need to update. Since this is an internal project, the risk is low, but grep the workspace for all call sites before merging.

2. **Skill file not found at runtime**: If the caller uses `SkillLoader` and the `skills/orchestrator.md` file is missing or malformed, the orchestrator will fail to start. This is intentional -- a misconfigured orchestrator should fail fast rather than silently running with a dummy manifest.

3. **Circular dependency risk**: If `orchestrator` depends on `skill-loader`, verify there is no reverse dependency. Currently `skill-loader` does not depend on `orchestrator`, so adding the forward dependency is safe. However, this dependency is only needed if error bridging is done inside the orchestrator crate; the simpler approach avoids it entirely.

4. **Async requirement**: `SkillLoader::load()` is async, which matches `from_config_with_model` (already async) but not `from_config` (currently sync). Since the manifest is now passed in pre-loaded, `from_config` remains sync. The async loading happens at the call site.

5. **Test isolation**: Tests use `build_test_manifest()` which constructs manifests in-memory. This is correct and should not be changed to load from disk, as that would couple unit tests to the file system.

## Verification

1. `cargo check` -- confirms the changed signatures compile across the workspace
2. `cargo clippy` -- no new warnings
3. `cargo test` -- all existing tests pass (tests use `Orchestrator::new()` directly, not `from_config`)
4. Grep for `build_default_manifest` -- zero occurrences remain
5. Grep for `from_config` -- all call sites pass a `SkillManifest` argument
6. If a binary entrypoint exists, verify it loads `skills/orchestrator.md` via `SkillLoader` and passes the result to `from_config`
