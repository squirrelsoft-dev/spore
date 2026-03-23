# Spec: Add integration test for tool-coder skill

> From: .claude/tasks/issue-20.md

## Objective

Add an integration test (`load_tool_coder_skill`) to the existing example skills test suite that verifies the `tool-coder.md` skill file can be loaded by `SkillLoader` and that all frontmatter fields and preamble content match expected values. This ensures the tool-coder skill is well-formed and consistent with the project's skill contract before it is used at runtime.

## Current State

The integration test file `crates/skill-loader/tests/example_skills_test.rs` already contains four tests following an identical pattern:

- `load_cogs_analyst_skill`
- `load_echo_skill`
- `load_skill_writer_skill`
- `load_orchestrator_skill`

Each test:
1. Calls `skills_dir()` to resolve the `skills/` directory at the repo root.
2. Calls `make_loader(&dir)` to create a `SkillLoader` with an `AllToolsExist` checker (bypasses real tool validation).
3. Calls `loader.load("<skill-name>").await.unwrap()` to parse the markdown file and produce a `SkillManifest`.
4. Asserts exact values for all frontmatter fields: `name`, `version`, `description`, `model.provider`, `model.name`, `model.temperature`, `tools`, `constraints.max_turns`, `constraints.confidence_threshold`, `constraints.escalate_to`, `constraints.allowed_actions`, `output.format`, `output.schema`.
5. Asserts the preamble is non-empty and contains domain-relevant keywords.

The `SkillManifest` struct (from `crates/agent-sdk/src/skill_manifest.rs`) has these fields: `name: String`, `version: String`, `description: String`, `model: ModelConfig`, `preamble: String`, `tools: Vec<String>`, `constraints: Constraints`, `output: OutputSchema`.

The `skills/` directory currently has: `cogs-analyst.md`, `echo.md`, `orchestrator.md`, `skill-writer.md`. The `tool-coder.md` file does not yet exist (blocked by upstream tasks).

## Requirements

- Add a single `#[tokio::test]` async function named `load_tool_coder_skill` to `crates/skill-loader/tests/example_skills_test.rs`.
- Reuse the existing `skills_dir()` and `make_loader()` helpers (no new helpers needed).
- Call `loader.load("tool-coder").await.unwrap()` to load `skills/tool-coder.md`.
- Assert exact equality for all frontmatter fields. The exact values depend on whatever the upstream task ("Create `skills/tool-coder.md` with YAML frontmatter") defines, but the test must cover every field:
  - `manifest.name` == `"tool-coder"`
  - `manifest.version` (exact value from frontmatter)
  - `manifest.description` (exact value from frontmatter)
  - `manifest.model.provider` (expected: `"anthropic"`)
  - `manifest.model.name` (exact model name from frontmatter)
  - `manifest.model.temperature` (use `(value - expected).abs() < f64::EPSILON` pattern)
  - `manifest.tools` (exact vec of tool name strings from frontmatter)
  - `manifest.constraints.max_turns` (exact u32 from frontmatter)
  - `manifest.constraints.confidence_threshold` (use epsilon comparison)
  - `manifest.constraints.escalate_to` (exact `Option<String>` from frontmatter)
  - `manifest.constraints.allowed_actions` (exact vec from frontmatter)
  - `manifest.output.format` (exact string from frontmatter)
  - `manifest.output.schema` (assert `.len()` and each key-value pair)
- Assert `!manifest.preamble.is_empty()`.
- Assert keyword presence in the preamble using `contains` checks with `||` alternatives and descriptive panic messages, following the `load_skill_writer_skill` pattern:
  - Contains `"MCP"` or `"mcp"` (tool-coder works with MCP tool servers)
  - Contains `"Rust"` or `"rust"` (the tool-coder writes Rust code)
  - Contains `"cargo"` or `"build"` (build/compile verification)
  - Contains `"tool-registry"` or `"missing tool"` (references tool registry concerns)

## Implementation Details

- **File to modify:** `crates/skill-loader/tests/example_skills_test.rs`
- **No new files to create.** The test is appended to the existing file.
- **No new dependencies required.**

### Test function signature

```rust
#[tokio::test]
async fn load_tool_coder_skill() {
    let dir = skills_dir();
    let loader = make_loader(&dir);
    let manifest = loader.load("tool-coder").await.unwrap();

    // Exact frontmatter assertions (values TBD from upstream skill file)
    assert_eq!(manifest.name, "tool-coder");
    // ... all other field assertions ...

    // Preamble assertions
    assert!(!manifest.preamble.is_empty());
    assert!(
        manifest.preamble.contains("MCP") || manifest.preamble.contains("mcp"),
        "preamble should reference MCP tool protocol"
    );
    assert!(
        manifest.preamble.contains("Rust") || manifest.preamble.contains("rust"),
        "preamble should reference Rust as the implementation language"
    );
    assert!(
        manifest.preamble.contains("cargo") || manifest.preamble.contains("build"),
        "preamble should reference cargo build or build verification"
    );
    assert!(
        manifest.preamble.contains("tool-registry") || manifest.preamble.contains("missing tool"),
        "preamble should reference tool-registry or missing tool handling"
    );
}
```

### Notes for the implementer

- The exact frontmatter field values (`version`, `description`, `model.name`, `model.temperature`, `tools`, `constraints.*`, `output.*`) must be filled in after the upstream task creates `skills/tool-coder.md`. The test must match the file exactly.
- Follow the epsilon comparison pattern for float fields: `assert!((manifest.model.temperature - EXPECTED).abs() < f64::EPSILON);`
- Follow the `output.schema` pattern from other tests: assert `.len()` first, then assert each key-value pair with `.get("key").unwrap()`.

## Dependencies

- **Blocked by:** "Create `skills/tool-coder.md` with YAML frontmatter", "Write tool-coder preamble body" -- the exact assertion values cannot be finalized until the skill file exists.
- **Blocking:** "Run verification suite" -- the test must pass before verification is complete.

## Risks & Edge Cases

- **Frontmatter value mismatch:** If the skill file changes after the test is written, the test will fail. This is intentional -- the test acts as a contract test. The implementer should write the test and skill file in coordination.
- **Preamble keyword drift:** The `contains` checks use `||` alternatives to be somewhat resilient to wording changes, but if the preamble is rewritten significantly, these assertions may need updating.
- **Float comparison:** Using `f64::EPSILON` works for values that are exact in IEEE 754 (like 0.0, 0.1, 0.2, 0.5, 1.0 when parsed from YAML). If the temperature uses a value like 0.3 that has no exact representation, consider using a small tolerance like `1e-10` instead.

## Verification

- `cargo test --test example_skills_test load_tool_coder_skill` passes (requires `skills/tool-coder.md` to exist).
- `cargo test --test example_skills_test` passes (all existing tests still pass, confirming no regressions).
- `cargo clippy` reports no new warnings in the test file.
