---
name: triage
description: >
  Triage GitHub issues by analyzing the codebase and posting structured implementation plans as comments.
  Use this skill when the user wants to triage, analyze, assess, or plan a GitHub issue — especially
  when they provide an issue number (e.g., "triage #12", "look at issue 45", "assess this issue").
  Also supports triaging all issues in a milestone at once with --milestone flag.
  Trigger on phrases like "triage", "assess issue", "plan issue", "what would it take to implement #X",
  "investigate issue", or "analyze issue".
---

# Triage

Investigate a GitHub issue (or all issues in a milestone), explore the codebase to understand how to implement it, and post a structured triage comment on each issue.

## Usage

```
/triage <issue-number>           # Triage a single issue
/triage --milestone <number>     # Triage all issues in a milestone
```

## Workflow

### 1. Parse Arguments

Determine the mode from the user's input:

- **Single issue**: Extract the issue number from the argument (e.g., `42`, `#42`)
- **Milestone mode**: If `--milestone <N>` is provided, fetch all issues belonging to that milestone

### 2. Fetch Issues

Use the `gh` CLI to retrieve issue details.

**Single issue:**

```bash
gh issue view <number> --json number,title,body,labels,assignees,milestone,comments
```

**Milestone (all open issues):**

```bash
gh issue list --milestone <milestone-title-or-number> --state open --json number,title,body,labels,assignees --limit 100
```

If fetching a milestone, first get the milestone details to resolve the title:

```bash
gh api repos/{owner}/{repo}/milestones/<number> --jq '.title'
```

### 3. Investigate the Codebase

For each issue, understand what the issue is asking for and then explore the codebase to figure out how to implement it. This is the most important step — the quality of the triage depends on understanding both the request and the existing code.

**What to investigate:**

- Read the issue title and body carefully — understand the user story, bug report, or feature request
- Search for files, modules, and patterns relevant to the issue (use Glob and Grep)
- Identify existing code that would need to change or that the new code would interact with
- Look for related tests, configurations, and documentation
- Check for similar patterns already implemented elsewhere in the codebase that could serve as a reference
- Note any dependencies or infrastructure that might be involved

**Use subagents for parallel investigation** when triaging a milestone — spawn an Explore agent for each issue to investigate concurrently. This dramatically speeds up milestone triaging. For a single issue, investigate inline.

### 4. Compose the Triage Comment

Write a clear, actionable implementation plan. The comment should give a developer everything they need to start working on the issue confidently.

**Comment structure:**

```markdown
## Triage

### Summary

<!-- One or two sentences: what this issue is asking for, in your own words.
     Show that you understood the intent, not just the surface request. -->

### Affected Areas

<!-- List the specific files, modules, or areas of the codebase that are relevant.
     Use file paths. Briefly note why each is relevant. -->

- `src/components/Dashboard.tsx` — will need a new panel for the widget
- `src/api/routes/metrics.ts` — new endpoint needed for data
- `src/types/index.ts` — extend the MetricsResponse type

### Implementation Approach

<!-- Step-by-step plan. Be specific about what to do, not vague hand-waving.
     Reference existing patterns in the codebase where applicable. -->

1. **Create the data model** — Add `Widget` type to `src/types/index.ts`, following the pattern used by `Dashboard` type on line 45
2. **Add API endpoint** — Create `GET /api/widgets` in `src/api/routes/`, similar to the existing `metrics.ts` route
3. **Build the component** — ...
4. **Write tests** — ...

### Considerations

<!-- Risks, open questions, edge cases, or things the implementer should watch out for.
     Only include if there are genuine concerns — don't pad this section. -->

- The current auth middleware doesn't support widget-level permissions — may need extending
- Consider pagination if the widget list could grow large

### Complexity

<!-- One of: Small, Medium, Large — with a brief justification -->

**Medium** — New endpoint + component, but follows established patterns
```

Adapt the structure to fit the issue. Bug fixes might emphasize root cause analysis over implementation steps. Small issues don't need lengthy plans. Use judgment — the goal is to be helpful, not to fill in a template mechanically.

### 5. Post the Comment

```bash
gh issue comment <number> --body "<triage-comment>"
```

Use a heredoc to avoid shell escaping issues:

```bash
gh issue comment <number> --body "$(cat <<'TRIAGE_EOF'
<triage comment content here>
TRIAGE_EOF
)"
```

### 6. Report Back

After posting, confirm to the user what was done:

- For a single issue: "Posted triage comment on #42"
- For a milestone: Summarize how many issues were triaged, list them briefly, and note any that couldn't be triaged (e.g., insufficient detail in the issue)

## Milestone Mode Details

When triaging a milestone:

1. Fetch all open issues in the milestone
2. Show the user the list of issues that will be triaged and confirm before proceeding
3. Use subagents to investigate and triage issues in parallel where possible (batch 3-5 at a time to avoid overwhelming the system)
4. Post a comment on each issue
5. Give a summary at the end

If an issue lacks enough detail to produce a meaningful triage, skip it and note it in the summary rather than posting a vague comment.

## Tips

- Reference specific line numbers and file paths — vague comments like "update the frontend" aren't useful
- If the codebase has a CLAUDE.md or contributing guide, factor those conventions into the plan
- Look for existing tests to understand testing patterns, and suggest tests that follow the same style
- If the issue references other issues or PRs, read those too for context
- Keep the comment concise — a developer should be able to scan it in under 2 minutes
