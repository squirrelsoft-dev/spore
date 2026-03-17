# Spec: Add integration test loading skill files from `skills/` directory

> From: .claude/tasks/issue-7.md

## Objective

Add an integration test file at `crates/skill-loader/tests/example_skills_test.rs` that loads the three actual skill files (`cogs-analyst.md`, `echo.md`, `skill-writer.md`) from the `skills/` directory at the workspace root. This test uses `SkillLoader` with `AllToolsExist` as the tool checker to bypass stub tool name validation. It asserts that each file loads without error and that every field of the resulting `SkillManifest` matches expected values. This serves as a regression test: if the schema evolves, the test catches breakage in the example skill files.

## Current State

**Loader API:** `SkillLoader::new(skill_dir: PathBuf, tool_registry: Arc<ToolRegistry>, tool_checker: Box<dyn ToolExists + Send + Sync>)` constructs a loader. `SkillLoader::load(skill_name: &str) -> Result<SkillManifest, SkillError>` is async and appends `.md` to the skill name.

**`AllToolsExist`** is exported from `skill_loader` and implements `ToolExists` by always returning `true`.

**`SkillManifest`** has eight fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec), `constraints` (Constraints), `output` (OutputSchema).

**`Constraints.escalate_to`** is `Option<String>` with `#[serde(default)]`.

**Existing test patterns** in `crates/skill-loader/tests/skill_loader_test.rs` and `validation_integration_test.rs`:
- `make_loader(dir: &Path) -> SkillLoader` helper
- `Arc::new(ToolRegistry)` for tool registry
- `Box::new(AllToolsExist)` for tool checker
- `#[tokio::test]` async functions
- Field-by-field assertions with `assert_eq!` for strings/integers and `(value - expected).abs() < f64::EPSILON` for floats

**`Cargo.toml`** already includes `tokio` and `tempfile` as dev-dependencies. No new dependencies needed.

## Requirements

1. **File location:** `crates/skill-loader/tests/example_skills_test.rs`

2. **Locate `skills/` directory** using `env!("CARGO_MANIFEST_DIR")` → `{CARGO_MANIFEST_DIR}/../../skills/`, canonicalized.

3. **Test `cogs-analyst.md`** — load and assert all fields:
   - `name` == `"cogs-analyst"`, `version` == `"1.0.0"`, `description` == `"Handles COGS-related finance queries"`
   - `model.provider` == `"anthropic"`, `model.name` == `"claude-sonnet-4-6"`, `model.temperature` ≈ `0.1`
   - `tools` == `["get_account_groups", "execute_sql", "query_store_lookup"]`
   - `constraints.max_turns` == `5`, `constraints.confidence_threshold` ≈ `0.75`
   - `constraints.escalate_to` == `Some("general-finance-agent".to_string())`
   - `constraints.allowed_actions` == `["read", "query"]`
   - `output.format` == `"structured_json"`, `output.schema` has 4 entries (`sql`, `explanation`, `confidence`, `source`)
   - `preamble` is non-empty and contains key phrases (e.g., `"COGS"`)

4. **Test `echo.md`** — load and assert all fields:
   - `name` == `"echo"`, `version` == `"1.0"`
   - `model.name` == `"claude-haiku-4-5-20251001"`, `model.temperature` ≈ `0.0`
   - `tools` is empty, `constraints.max_turns` == `1`, `constraints.confidence_threshold` ≈ `1.0`
   - `constraints.escalate_to` == `None`
   - `constraints.allowed_actions` is empty
   - `output.format` == `"text"`, `output.schema` is empty
   - `preamble` == `"Echo back the input exactly as received. Do not modify, summarize, or interpret."`

5. **Test `skill-writer.md`** — load and assert all fields:
   - `name` == `"skill-writer"`, `version` == `"0.1"`
   - `model.name` == `"claude-sonnet-4-6"`, `model.temperature` ≈ `0.2`
   - `tools` == `["write_file", "validate_skill"]`
   - `constraints.max_turns` == `10`, `constraints.confidence_threshold` ≈ `0.9`
   - `constraints.escalate_to` == `None`
   - `constraints.allowed_actions` == `["read", "write"]`
   - `output.format` == `"structured_json"`, `output.schema` has 2 entries
   - `preamble` is non-empty and multi-line (contains `'\n'`)

6. **Use `AllToolsExist`** for all three tests.

7. **No new dependencies** needed.

## Implementation Details

### Helper functions

```rust
fn skills_dir() -> std::path::PathBuf {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../../skills").canonicalize().expect("skills/ directory must exist")
}

fn make_loader(dir: &std::path::Path) -> SkillLoader {
    let registry = Arc::new(ToolRegistry);
    SkillLoader::new(dir.to_path_buf(), registry, Box::new(AllToolsExist))
}
```

### Test functions

Three `#[tokio::test]` async functions:

1. **`async fn load_cogs_analyst_skill()`** — Full field-by-field assertions. Preamble checked via `contains("COGS")` and non-empty.
2. **`async fn load_echo_skill()`** — Boundary value test. Exact preamble match. Empty collections.
3. **`async fn load_skill_writer_skill()`** — Stub tools test. Multi-line preamble assertion via `contains('\n')`.

### Assertion patterns

Follow existing test conventions:
- Strings: `assert_eq!(manifest.name, "cogs-analyst");`
- Floats: `assert!((manifest.model.temperature - 0.1).abs() < f64::EPSILON);`
- Vecs: `assert_eq!(manifest.tools, vec!["get_account_groups", ...]);`
- Options: `assert_eq!(manifest.constraints.escalate_to, Some("...".to_string()));`
- HashMaps: `assert_eq!(manifest.output.schema.get("sql").unwrap(), "string");` + `assert_eq!(manifest.output.schema.len(), 4);`

### What NOT to do

- Do not use `tempfile` or inline strings. Load actual files from `skills/`.
- Do not test with a real `ToolExists` implementation.

## Dependencies

- **Blocked by:** All Group 1 tasks (the three skill files must exist)
- **Blocking:** None

## Risks & Edge Cases

1. **Path resolution:** `env!("CARGO_MANIFEST_DIR")` resolves to `crates/skill-loader/`. Must navigate `../../skills/` to reach workspace root. `canonicalize()` panics with a clear message if directory missing.
2. **Preamble content sensitivity:** Use substring checks for multi-line preambles, exact match only for echo's single-line preamble.
3. **Schema map ordering:** `HashMap` iteration is non-deterministic, but all assertions use `.get(key)` lookups.
4. **`escalate_to` handling:** Echo and skill-writer omit `escalate_to` → assert `None`. Cogs-analyst includes it → assert `Some(...)`.

## Verification

1. `cargo test --test example_skills_test` — all three tests pass.
2. Temporarily rename a skill file to confirm the corresponding test fails with `SkillError::IoError`.
3. `cargo test` (full suite) — no regressions.
4. `cargo clippy` — no lint warnings in the new test file.
