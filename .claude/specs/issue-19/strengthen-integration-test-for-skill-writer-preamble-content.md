# Spec: Strengthen integration test for skill-writer preamble content
> From: .claude/tasks/issue-19.md

## Objective

Add keyword-presence assertions to the `load_skill_writer_skill` integration test so it verifies that the skill-writer preamble actually encodes the SkillManifest schema specification, rather than merely checking that the preamble is non-empty and contains a newline.

## Current State

The `load_skill_writer_skill` test in `crates/skill-loader/tests/example_skills_test.rs` (lines 88-124) asserts all frontmatter fields correctly but only has two weak preamble assertions:

```rust
assert!(!manifest.preamble.is_empty());
assert!(manifest.preamble.contains('\n'));
```

By contrast, other tests in the same file use content-based preamble checks:
- `load_cogs_analyst_skill` asserts `manifest.preamble.contains("COGS")`
- `load_orchestrator_skill` asserts `manifest.preamble.contains("route") || manifest.preamble.contains("router")`

The skill-writer test should follow these same patterns but with assertions that confirm the format specification content is present.

## Requirements

Replace the two existing weak preamble assertions with a stronger set. Each assertion should use `contains()` with keyword presence checks (not exact string matches). Use `||` alternatives where multiple phrasings are acceptable.

The following concepts must be verified as present in the preamble:

1. **Schema/format reference**: The preamble mentions the manifest schema or skill file format.
   - Check: `manifest.preamble.contains("SkillManifest") || manifest.preamble.contains("skill file format")`

2. **Confidence threshold**: The preamble documents the `confidence_threshold` constraint.
   - Check: `manifest.preamble.contains("confidence_threshold")`

3. **Model configuration**: The preamble documents model configuration.
   - Check: `manifest.preamble.contains("ModelConfig") || manifest.preamble.contains("model")`

4. **Output schema**: The preamble documents the output format/schema.
   - Check: `manifest.preamble.contains("OutputSchema") || manifest.preamble.contains("output format")`

5. **Validation**: The preamble includes validation rules or guidance.
   - Check: `manifest.preamble.contains("validation") || manifest.preamble.contains("Validation")`

Keep the existing `assert!(!manifest.preamble.is_empty())` assertion. The `contains('\n')` assertion can be kept or removed (it is implied by the richer checks but harmless to retain).

## Implementation Details

Edit the `load_skill_writer_skill` test function in `crates/skill-loader/tests/example_skills_test.rs`. Replace lines 122-123 with the expanded assertions. The changes are confined to this single function; no other tests or files are modified.

The assertions should follow the existing style in the file:
- Use `assert!()` with `manifest.preamble.contains(...)`
- Use `||` for alternative acceptable phrasings within a single `assert!()`
- Add a descriptive failure message string as the second argument to `assert!()` so failures are diagnosable, e.g.:
  ```rust
  assert!(
      manifest.preamble.contains("SkillManifest") || manifest.preamble.contains("skill file format"),
      "preamble should reference the skill manifest schema or file format"
  );
  ```

No new imports, helper functions, or test utilities are needed. No new test functions are created -- this only modifies the existing `load_skill_writer_skill` function.

## Dependencies

**Blocked by**: "Expand skill-writer preamble with complete SkillManifest schema documentation" (Group 1 task). That task rewrites the preamble body of `skills/skill-writer.md` to include the full format specification. Until that preamble contains the expected keywords, these new assertions will fail.

**Blocking**: Nothing. This is a non-blocking leaf task (Group 2).

## Risks & Edge Cases

1. **Keyword drift**: If the preamble task uses different terminology (e.g., "manifest schema" instead of "SkillManifest"), assertions could fail. Mitigated by using `||` alternatives and checking for both Rust type names and plain-English equivalents.

2. **Case sensitivity**: `contains()` is case-sensitive. The check for "validation" uses `|| manifest.preamble.contains("Validation")` to handle both heading case and inline usage. If the preamble uses all-caps "VALIDATION", add that variant too.

3. **Over-constraining**: The assertions intentionally use broad keyword checks rather than exact substrings. Do not assert on specific sentence structures, field ordering, or formatting details. The goal is to confirm the presence of key concepts, not to pin down exact wording.

4. **Frontmatter assertions unchanged**: Do not modify any of the existing frontmatter assertions (lines 93-120). Those are correct and tested. This task only touches preamble assertions.

## Verification

1. After Group 1 completes (preamble expansion), run `cargo test --test example_skills_test load_skill_writer_skill` and confirm all new assertions pass.
2. Run the full test suite with `cargo test` to confirm no regressions in other tests.
3. Run `cargo clippy` to confirm no new warnings from the added assertions.
