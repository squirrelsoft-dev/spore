# Task Breakdown: Create skill-writer seed agent

> Flesh out `skills/skill-writer.md` from its current stub into the full seed agent that can produce validated skill files from plain-language descriptions, encoding the complete SkillManifest schema specification in its preamble.

## Group 1 — Expand skill-writer preamble with full format spec

_Tasks in this group can be done in parallel._

- [x] **Expand skill-writer preamble with complete SkillManifest schema documentation** `[M]`
      The current `skills/skill-writer.md` has correct YAML frontmatter but its preamble is a shallow stub (18 lines). The core value of the skill-writer agent is its preamble -- it must encode the complete skill file format specification so the LLM can generate valid files. Rewrite the markdown body to include: (1) An expanded introductory paragraph establishing this as the first seed agent in Spore's self-bootstrapping factory, partnered with the tool-coder (#20). (2) A `## Skill File Format Specification` section documenting every `SkillManifest` field, its type, valid values, and semantics -- referencing the exact struct definitions from agent-sdk: `SkillManifest` (8 fields), `ModelConfig` (provider: String, name: String, temperature: f64), `Constraints` (max_turns: u32, confidence_threshold: f64 in [0.0, 1.0], escalate_to: Option<String>, allowed_actions: Vec<String>), `OutputSchema` (format: String one of "json"/"structured_json"/"text", schema: HashMap<String, String>). Document that the file format is markdown-with-frontmatter where YAML frontmatter contains all fields except `preamble`, and the markdown body becomes the preamble. (3) A `## Validation Rules` section documenting: `version` must be quoted to prevent YAML float coercion, `confidence_threshold` must be in [0.0, 1.0], `max_turns` must be > 0, required strings (name, version, model.provider, model.name) must be non-empty, preamble must be non-empty, `escalate_to` must be omitted (not empty string) when not needed, output format must be one of the three allowed values, `---` must not appear alone on a line in the preamble body. (4) An expanded `## Process` section with detailed numbered steps that reference format spec validation. (5) Keep the existing `## Output` section describing `skill_yaml` and `validation_result` structured JSON fields. Do NOT modify the YAML frontmatter -- it is already correct and passes integration tests.
      Files: `skills/skill-writer.md`
      Blocking: "Strengthen integration test for skill-writer preamble content"

## Group 2 — Strengthen integration test

_Depends on: Group 1._

- [x] **Strengthen integration test for skill-writer preamble content** `[S]`
      The existing `load_skill_writer_skill` test in `crates/skill-loader/tests/example_skills_test.rs` only checks that the preamble is non-empty and contains a newline. Add assertions that the preamble contains key phrases confirming the format spec is present, following the pattern used by the cogs-analyst test (`assert!(manifest.preamble.contains("COGS"))`) and the orchestrator test (`assert!(manifest.preamble.contains("route"))`). Assert that the preamble contains phrases like "SkillManifest" or "skill file format", "confidence_threshold", "ModelConfig" or "model", "OutputSchema" or "output format", and "validation" -- confirming the preamble encodes the schema specification. Do not over-constrain with exact string matches; use keyword presence checks.
      Files: `crates/skill-loader/tests/example_skills_test.rs`
      Blocked by: "Expand skill-writer preamble with complete SkillManifest schema documentation"
      Non-blocking

## Group 3 — Verification

_Depends on: Group 2._

- [x] **Run verification suite** `[S]`
      Run `cargo check`, `cargo clippy`, and `cargo test` across the full workspace. Verify all existing tests pass including the strengthened skill-writer integration test. Confirm `SkillLoader::load("skill-writer")` succeeds with `AllToolsExist` and the preamble is multi-line and contains format spec content.
      Files: (none -- command-line verification only)
      Blocked by: All other tasks

## Implementation Notes

1. **Do not modify frontmatter**: The YAML frontmatter is already correct and all 4 integration tests pass. The issue is purely about preamble content depth.

2. **Preamble is documentation-as-configuration**: The skill-writer's effectiveness depends entirely on how thoroughly it encodes the skill file schema in its preamble. The LLM will use this as its only reference for generating valid skill files.

3. **Reference exact types from agent-sdk**: The preamble should document the exact Rust types and their constraints. For example, `confidence_threshold` is `f64` in range `[0.0, 1.0]`, `max_turns` is `u32` and must be `> 0`, `allowed_actions` is `Vec<String>`, output `format` must be one of `ALLOWED_OUTPUT_FORMATS: ["json", "structured_json", "text"]`.

4. **`escalate_to` is `Option<String>`**: This field uses `#[serde(default, skip_serializing_if = "Option::is_none")]`, meaning it can be omitted from frontmatter to default to `None`. The preamble must document this behavior.

5. **Stub tools are intentional**: `write_file` and `validate_skill` are declared in the frontmatter but do not exist in the tool registry yet. They will be implemented by the tool-coder (#20) or manually via issue #8.

6. **Avoid `---` on its own line in preamble**: The frontmatter parser uses `---` as a delimiter. A standalone `---` line in the markdown body could cause issues. Use `----` or horizontal rule alternatives if needed.
