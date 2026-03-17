# Spec: Update README.md to show markdown-with-frontmatter format

> From: .claude/tasks/issue-7.md

## Objective

Update all references in `README.md` from the pure YAML skill file format to the markdown-with-frontmatter format that `skill-loader` now uses. This includes replacing the canonical code example, updating descriptive text, and changing file extension references from `.yaml` to `.md`.

## Current State

The README contains 7 locations referencing the old YAML skill file format:

1. **Line 17** — "A YAML document declaring everything the agent needs"
2. **Lines 19-53** — A `` ```yaml `` fenced code block with the full `cogs-analyst` skill definition in pure YAML format (including `preamble` as a YAML block scalar)
3. **Line 65** — Architecture tree comment: `# Skill file definitions (YAML)`
4. **Line 74** — skill-loader crate description: "Parses YAML skill files"
5. **Line 98** — `skill-writer.yaml` and "produces a validated YAML skill file"
6. **Line 99** — `tool-coder.yaml`
7. **Line 101** — `deploy-agent.yaml`

## Requirements

- Replace all 7 YAML references with markdown-with-frontmatter equivalents
- The code example must match the content of `skills/cogs-analyst.md` (created in Group 1)
- No other README content should be modified
- The `serde` / `schemars` reference (line 111) should remain unchanged — it refers to the serialization library, not file format

## Implementation Details

### Change 1: Line 17 — Format description
- **Old:** `A YAML document declaring everything the agent needs`
- **New:** `A markdown file with YAML frontmatter declaring everything the agent needs`

### Change 2: Lines 19-53 — Code example
- Change fenced code block language from `` ```yaml `` to `` ```markdown ``
- Replace the pure YAML content with the markdown-with-frontmatter format from `skills/cogs-analyst.md`
- The frontmatter contains all fields except `preamble`, and the markdown body after the closing `---` is the preamble

### Change 3: Line 65 — Architecture tree comment
- **Old:** `# Skill file definitions (YAML)`
- **New:** `# Skill file definitions (markdown)`

### Change 4: Line 74 — skill-loader description
- **Old:** `Parses YAML skill files`
- **New:** `Parses markdown skill files with YAML frontmatter` (or similar)

### Change 5: Line 98 — skill-writer reference
- **Old:** `skill-writer.yaml` and "produces a validated YAML skill file"
- **New:** `skill-writer.md` and "produces a validated skill file"

### Change 6: Line 99 — tool-coder reference
- **Old:** `tool-coder.yaml`
- **New:** `tool-coder.md`

### Change 7: Line 101 — deploy-agent reference
- **Old:** `deploy-agent.yaml`
- **New:** `deploy-agent.md`

### Files to modify
- `README.md` — the sole file to modify

## Dependencies

- **Blocked by:** "Create `skills/cogs-analyst.md` finance domain skill" — the code example must match the actual file content
- **Blocking:** None

## Risks & Edge Cases

1. **Code example drift:** If the README code example diverges from `skills/cogs-analyst.md`, they become inconsistent. The example should be copied from the actual file or kept tightly in sync.
2. **Markdown in markdown:** The code example will show markdown-with-frontmatter inside a markdown fenced code block. Using `` ```markdown `` is the correct approach.
3. **Line number shifts:** If the code example changes in length, subsequent line numbers will shift. All changes should be made in a single pass.

## Verification

1. All 7 old YAML references are replaced.
2. No remaining occurrences of `skill-writer.yaml`, `tool-coder.yaml`, or `deploy-agent.yaml` in README.md.
3. The code example block uses `` ```markdown `` and shows frontmatter-delimited format.
4. The text accurately describes the markdown-with-frontmatter format.
5. The `serde` / `schemars` reference is unchanged.
6. `cargo test` and `cargo clippy` pass (no code changes, but confirm nothing is broken).
