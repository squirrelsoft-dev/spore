---
description: 'Implement the next task group (or all groups) from a task breakdown using focused agents'
---

# Work

Orchestrate implementation of a task breakdown by coordinating the Git Expert, Implementer, and Quality agents. Uses Claude Code's native task tools to track group progress so state survives across context resets.

**Input:** `$ARGUMENTS` — task list name, optionally followed by `--all`

- `work <task-name>` — implement the next available group only
- `work <task-name> --all` — enqueue and implement every incomplete group in dependency order

---

## Steps

### 1. Parse arguments

Split `$ARGUMENTS` on whitespace. Extract:
- `taskListName` — first token (e.g. `issue-3`)
- `allFlag` — true if `--all` is present

Read `.claude/tasks/<taskListName>.md`. If it doesn't exist, tell the user and stop.

### 2. Parse the task list

Parse the file structure:

- Groups are headed by `## Group N — <label>` lines
- Tasks match `- [ ] **Task title**` (incomplete) or `- [x] **Task title**` (complete)
- A group is **complete** if all its tasks are `- [x]`
- A group is **available** if not complete AND all groups in its `_Depends on: ..._` line are complete
- A group is **blocked** if any dependency group has incomplete tasks

If all groups are already complete, tell the user and stop.

### 3. Verify specs exist

Before doing anything, check that `.claude/specs/<taskListName>/` contains a spec file for every incomplete task across all groups you intend to process. For each incomplete task, look for `.claude/specs/<taskListName>/<task-title-kebab>.md`.

If any specs are missing, list them and suggest running `/spec <taskListName>` first. Stop.

### 4. Enqueue groups via TaskCreate

**If `--all` flag:**
For each incomplete group in dependency order, call `TaskCreate` with:
- `title`: `<taskListName> — Group <N>: <label>`
- `description`: comma-separated list of incomplete task titles in the group

Print the queue:
```
Work All: <taskListName>
──────────────────────────
[queued] Group 1 — <label>  (<N> tasks)
[queued] Group 2 — <label>  (<N> tasks)
...
[complete] Group N — <label>  (skipped)
──────────────────────────
```

**If single group:**
Find the next available group. If multiple are available, use `AskUserQuestion` to let the user pick. Auto-select if only one is available. Call `TaskCreate` for that group only.

### 5. Process the queue

Call `TaskList` to get pending tasks. For each pending task in order, run the pipeline below. After each group completes successfully, call `TaskUpdate` to mark it done before moving to the next.

If `--all` is not set, stop after the first group.

---

## Pipeline (per group)

Run these phases in sequence for each group.

### Phase 1 — Setup

Derive names:
- `taskGroupName` = `<taskListName>-group-<N>` (kebab-case)
- `featureBranch` = `feat/<taskGroupName>`
- For each incomplete task in the group, assign:
  - `agentNumber` — sequential integer starting at 1
  - `worktreeBranch` = `work/<taskGroupName>-<agentNumber>`

Invoke the **Git Expert** agent with `Operation: SETUP`:

```
Agent({
  agent: "git-expert",
  prompt: `
    Operation: SETUP
    taskGroupName: <taskGroupName>
    featureBranch: <featureBranch>
    tasks:
      - agentNumber: 1
        taskTitle: <task title>
        worktreeBranch: work/<taskGroupName>-1
      - agentNumber: 2
        taskTitle: <task title>
        worktreeBranch: work/<taskGroupName>-2
  `
})
```

Wait for `GIT_SETUP_COMPLETE`. If setup fails, mark the group's task entry failed and stop.

### Phase 2 — Implement

Spawn one **Implementer** agent per incomplete task in the group. Tasks within a group are parallel — spawn all at once with `run_in_background: true`.

For each task:

```
Agent({
  agent: "implementer",
  isolation: "worktree",
  worktreeBranch: "<worktreeBranch>",
  run_in_background: true,
  prompt: `
    Task: <task title>
    Branch: <worktreeBranch>

    <full contents of .claude/specs/<taskListName>/<task-title-kebab>.md>

    Implement this spec exactly. Commit all changes to your branch when done.
  `
})
```

Wait for all Implementers to complete. Collect `IMPLEMENTATION_COMPLETE` blocks.

If any Implementer reports `status: failed`, warn the user and ask via `AskUserQuestion`:
- `"Continue with passing tasks"` — proceed to Phase 3, skipping failed branches
- `"Stop"` — mark the group failed, stop

### Phase 3 — Quality gates

For each branch where Implementer reported `status: success`, spawn a **Quality** agent. Run them sequentially — one at a time, not in parallel.

```
Agent({
  agent: "quality",
  isolation: "worktree",
  worktreeBranch: "<worktreeBranch>",
  run_in_background: false,
  prompt: `
    branch: <worktreeBranch>
    taskTitle: <task title>
    specFile: .claude/specs/<taskListName>/<task-title-kebab>.md
  `
})
```

Parse the `QA_REPORT_START ... QA_REPORT_END` block from each Quality agent's output.

After each QA report, show the user:

```
QA Results — <worktreeBranch> (<task title>)

  Simplify        [PASS|FAIL]
  Review          [PASS|FAIL]
  Security Review [PASS|FAIL]
  Security Scan   [PASS|FAIL]

  Overall: [PASS|FAIL]
  Notes: <notes>
```

Then ask for confirmation via `AskUserQuestion`:

If overall **PASS**:
- `"Merge this branch"` ✅
- `"Skip this branch"` ⏭
- `"Stop here"` 🛑

If overall **FAIL**:
- `"Skip this branch"` ⏭
- `"Merge anyway (I accept the risk)"` ⚠️
- `"Stop here"` 🛑

If the user selects **"Stop here"**: call `TaskUpdate` to mark the group stopped and halt entirely. Do not process remaining branches or groups.

Collect the list of branches the user approved for merging.

### Phase 4 — Merge

Invoke the **Git Expert** agent with `Operation: MERGE`:

```
Agent({
  agent: "git-expert",
  run_in_background: false,
  prompt: `
    Operation: MERGE
    featureBranch: <featureBranch>
    taskListFile: .claude/tasks/<taskListName>.md
    branches:
      - <worktreeBranch-1>
      - <worktreeBranch-2>
    completedTasks:
      - <task title 1>
      - <task title 2>
  `
})
```

Wait for `GIT_MERGE_COMPLETE`. Report any conflicts that were resolved.

### Phase 5 — Verify

Invoke the **Git Expert** agent with `Operation: VERIFY`:

```
Agent({
  agent: "git-expert",
  run_in_background: false,
  prompt: `
    Operation: VERIFY
    featureBranch: <featureBranch>
    verificationCommands:
      - npm run build
      - npm run lint
      - npm run typecheck
      - npm test
  `
})
```

Wait for `GIT_VERIFY_COMPLETE`. If verification fails, report which checks failed and do not mark the group complete — tell the user to resolve issues and re-run.

### Phase 6 — Complete

Call `TaskUpdate` to mark the group's task entry as complete.

Print progress:

```
──────────────────────────
✓ Group <N> — <label> complete. Feature branch: feat/<taskGroupName>
  Tasks merged: <N>  |  Skipped: <N>
  Next: Group <M> — <label>   (or "All groups complete")
──────────────────────────
```

---

## Final summary (after all groups processed)

```
══════════════════════════════════
  Work Complete: <taskListName>
══════════════════════════════════

Groups completed this session:
  ✓ Group 1 — <label>  →  feat/<taskGroupName>
  ✓ Group 2 — <label>  →  feat/<taskGroupName>

Previously complete (skipped):
  ✓ Group N — <label>

Still incomplete (if any):
  ✗ Group M — <label>  (<reason>)

Next steps:
  Review feature branches and run /squash-pr when ready.
══════════════════════════════════
```

---

## Error handling

- **Implementer failure**: warn user, offer continue-with-passing or stop
- **QA report missing**: treat branch as FAIL, show raw output to user
- **Git conflict**: Git Expert resolves best-effort and documents it; user sees conflict summary before Phase 4 completes
- **Verification failure**: do not mark group complete; tell user what failed
- **Blocked groups** (in `--all` mode): if a group's dependencies are unmet after prior groups complete, report blockage and stop

## Rules

- Do NOT implement anything directly — always delegate to the Implementer agent
- Do NOT merge anything directly — always delegate to the Git Expert agent
- Do NOT mark tasks complete in the task list directly — Git Expert owns that
- Do NOT skip the Quality phase — every Implementer branch must pass through Quality before merging
- Do NOT process groups in parallel — dependency order must be respected
- DO spawn all Implementers in a group in parallel — that is the point of grouping
