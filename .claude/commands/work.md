---
description: 'Pick the next task grouping from a breakdown and spin up an agent team to implement it'
---

# Work Next Task

Pick a task grouping from a task breakdown and spin up an agent team to implement it, guided by the specs.

> **This command stops after agents complete.** It does NOT run quality gates or merge branches.

**Input:** `$ARGUMENTS` — optional task list name (maps to `.claude/tasks/$ARGUMENTS.md`). If omitted, the user picks from available task lists.

## Steps

### 1. Resolve the task list

- If `$ARGUMENTS` is provided, read `.claude/tasks/$ARGUMENTS.md`. If it doesn't exist, tell the user and stop.
- If `$ARGUMENTS` is **not** provided:
  1. Glob for all files matching `.claude/tasks/*.md`.
  2. For each file, read the first heading line (`# Task Breakdown: ...`) and check whether any tasks remain incomplete (lines matching `- [ ]`). Exclude files where every task is `- [x]`.
  3. If no task lists have incomplete tasks, tell the user "All task lists are complete" and stop.
  4. Present the list to the user with `AskUserQuestion` — show each task list name, its heading, and a count of remaining tasks (e.g. "my-feature — 4 tasks remaining"). Let the user pick one.

### 2. Parse task groupings

Read the selected task list file and parse its structure:

- Groups are headed by `## Group N — <label>` lines.
- Tasks within a group match the pattern `- [ ] **Task title**` (incomplete) or `- [x] **Task title**` (complete).
- A group is **fully complete** if all its tasks are `- [x]`.
- A group is **available** if it is not fully complete and all groups it depends on (listed in the `_Depends on: ..._` line) are fully complete.
- A group is **blocked** if it depends on a group that still has incomplete tasks.

### 3. Let the user choose a task grouping

Use `AskUserQuestion` to present the available (non-blocked, non-complete) groups. For each group show:

- Group number and label
- Count of incomplete tasks in the group
- Task titles at a glance

If only one group is available, skip the question and auto-select it.

If no groups are available (all remaining groups are blocked), tell the user which groups are blocking and stop.

### 4. Verify specs exist

Check that `.claude/specs/$ARGUMENTS/` exists and contains spec files. For each incomplete task in the selected group, look for a matching spec file at `.claude/specs/<task-list-name>/<task-title-kebab>.md`.

- If specs are missing for any task, tell the user which specs are missing and suggest running `/spec <task-list-name>` first. Stop.

### 5. Create a feature branch

Derive the **task group name** as `<task-list-name>-group-<N>` (kebab-case). Create and switch to a branch for this work:

```
git checkout -b feat/<task-group-name>
```

If the branch already exists, switch to it instead.

Record the feature branch name — it will be needed by `/work-merge`.

### 6. Plan the agent team

Switch to **plan mode** (`EnterPlanMode`). In the plan:

1. Read each spec file for the tasks in the selected group from `.claude/specs/<task-list-name>/`.
2. Identify dependencies between tasks — tasks within the group are parallel by design, but note any ordering preferences from `Blocked by` / `Blocking` fields in the specs.
3. Search for relevant community skills by running `npx skills find <topic>` for key technologies or patterns across the group's tasks. If useful skills are found, run `npx skills add <owner/repo@skill>` to install them before spawning agents.
4. For each task, define an agent assignment:
   - **Agent name** — derived from the task title (kebab-case)
   - **Worktree branch** — `work/<task-group-name>-<agent-number>` (e.g. `work/issue-10-group-1-1`)
   - **Prompt** — include the full spec content, the list of files to create/modify, and explicit instructions to implement the spec (not just plan). Include instructions to search for and install skills when encountering unfamiliar libraries or patterns.
5. Present the plan to the user for approval via `ExitPlanMode`.

### 7. Spawn implementation agents

After plan approval, spawn one subagent per task using the `Agent` tool. All agents run in parallel.

**Every agent MUST use `isolation: "worktree"`.** Do NOT create or modify files directly on the feature branch. Parallel agents write to the same working directory without isolation and can silently overwrite each other's work. There are no exceptions to this rule.

Each agent's worktree branch MUST be named `work/<task-group-name>-<agent-number>` (e.g. `work/issue-10-group-1-1`, `work/issue-10-group-1-2`), where `<agent-number>` is a sequential integer starting at 1.

For each task, call the `Agent` tool with:

- `subagent_type`: `"implementer"` (or `"general-purpose"` if implementer is unavailable)
- `isolation`: `"worktree"`
- `run_in_background`: `true`
- `mode`: `"auto"`
- `prompt`: Include the full spec content, the list of files to create/modify, and explicit instructions to implement the spec. Instruct the agent to **commit all changes** to its `work/` worktree branch before completing. Include the Skills CLI instructions below.

Each agent:

1. Reads its assigned spec file
2. If the task involves an unfamiliar library or pattern, searches for a skill first: `npx skills find <topic>`, then `npx skills add <owner/repo@skill>` if a match is found
3. Implements the changes described in the spec
4. Runs any verification steps listed in the spec (tests, lint, type checks)
5. **Commits all changes** to its `work/<task-group-name>-<agent-number>` branch
6. Reports completion status clearly

As agents complete, monitor for failures. If an agent fails, report which agent failed and why.

### 8. Write the session state file

After all agents finish (success or failure), write a session state file at:

```
.claude/work-sessions/<task-group-name>.json
```

This file is the **only input** `/work-merge` accepts. If the schema is wrong, `/work-merge` will fail or behave unpredictably.

#### Required schema — copy this structure exactly

The example below shows a completed run for task list `issue-2`, group 3, with two agents. Use it as a direct template — substitute real values for the placeholders, keep every key name identical.

```json
{
  "taskListName": "issue-2",
  "taskGroupName": "issue-2-group-3",
  "featureBranch": "feat/issue-2-group-3",
  "groupNumber": 3,
  "agents": [
    {
      "agentNumber": 1,
      "taskTitle": "Add database migrations",
      "worktreeBranch": "work/issue-2-group-3-1",
      "status": "success",
      "specFile": ".claude/specs/issue-2/add-database-migrations.md"
    },
    {
      "agentNumber": 2,
      "taskTitle": "Write unit tests",
      "worktreeBranch": "work/issue-2-group-3-2",
      "status": "failed",
      "specFile": ".claude/specs/issue-2/write-unit-tests.md"
    }
  ]
}
```

#### Field rules

| Field | Type | Rule |
|---|---|---|
| `taskListName` | string | The task list file name without `.md` (e.g. `issue-2`) |
| `taskGroupName` | string | `<taskListName>-group-<N>` (e.g. `issue-2-group-3`) |
| `featureBranch` | string | Always `feat/<taskGroupName>` |
| `groupNumber` | number | The integer group number, not a string |
| `agents[].agentNumber` | number | Sequential integer starting at 1 |
| `agents[].taskTitle` | string | Exact task title as it appears in the task list (spaces, not kebab) |
| `agents[].worktreeBranch` | string | Must start with `work/` — never `feat/` |
| `agents[].status` | string | Exactly `"success"` or `"failed"` — no other values |
| `agents[].specFile` | string | Full relative path to the spec file |

#### Common mistakes — do not do these

- ❌ Using `"group"` instead of `"taskGroupName"`
- ❌ Using `"branch"` instead of `"featureBranch"`
- ❌ Using `"name"` instead of `"taskTitle"`
- ❌ Setting `worktreeBranch` to a `feat/` branch — it must be a `work/` branch
- ❌ Using `"complete"` or `"done"` as a status — only `"success"` or `"failed"` are valid
- ❌ Adding extra fields like `"verification"`, `"tasks"`, or `"status"` at the top level

#### Self-validation — check before saving

Before writing the file, verify each item in this checklist:

- [ ] Top-level keys are exactly: `taskListName`, `taskGroupName`, `featureBranch`, `groupNumber`, `agents`
- [ ] `taskGroupName` matches the pattern `<taskListName>-group-<N>`
- [ ] `featureBranch` starts with `feat/`
- [ ] `groupNumber` is a number (not a string)
- [ ] Every agent entry has exactly these keys: `agentNumber`, `taskTitle`, `worktreeBranch`, `status`, `specFile`
- [ ] Every `worktreeBranch` starts with `work/`
- [ ] Every `status` is either `"success"` or `"failed"`
- [ ] No extra keys exist anywhere in the document

If any item is not checked, fix the JSON before writing it.

### 9. Summary

Print a summary:

- Task list and group that was worked
- Feature branch created: `feat/<task-group-name>`
- Each agent's worktree branch and status
- Path to the session state file written
- **Next step:** `Run /work-merge <task-group-name> to run quality gates and merge into the feature branch.`

Do not mark tasks complete in the task list. Do not run quality gates. Do not merge anything.
That is the job of `/work-merge`.

---

## Skills CLI

When a task involves a library, framework, or pattern you're not confident about, use the skills CLI to find and install community skills that provide expert guidance.

```bash
# Search for skills by topic
npx skills find <topic>

# Install a skill from the results
npx skills add <owner/repo@skill>
```

Include these instructions in every agent prompt so agents can discover and install skills during implementation.
