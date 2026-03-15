---
description: 'Run quality gates on completed work branches, then merge them into the feature branch'
---

# Work Merge

Run quality gates on all `work/` branches from a `/work` session, confirm each branch with the user, then merge passing branches into the feature branch and mark tasks complete.

**Input:** `$ARGUMENTS` — task group name (e.g. `my-feature-group-1`). This must match a session state file at `.claude/work-sessions/$ARGUMENTS.json`.

If `$ARGUMENTS` is omitted, glob for all files matching `.claude/work-sessions/*.json` and let the user pick one.

---

## Steps

### 1. Load the session state

Read `.claude/work-sessions/<task-group-name>.json`.

If the file does not exist, tell the user and stop. Suggest running `/work <task-list-name>` first.

From the session file, extract:

- `taskListName`
- `taskGroupName`
- `featureBranch`
- `groupNumber`
- `agents` array (each with `agentNumber`, `taskTitle`, `worktreeBranch`, `status`, `specFile`)

Filter the agents list to only those with `status: "success"`. Warn the user about any `"failed"` agents and ask if they want to continue without them or stop.

### 2. Switch to the feature branch

```
git checkout <featureBranch>
```

Confirm the branch exists. If not, tell the user and stop.

### 3. Gate and merge each branch — one at a time

For each successful agent branch, execute the full sequence below **before moving to the next branch**. Do not process branches in parallel — gate, confirm, and merge one branch at a time.

---

#### 3a. Announce the branch

Tell the user:

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
Branch [N of M]: work/<task-group-name>-<agent-number>
Task: <task title>
Running quality gates...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

#### 3b. Spawn the QA agent

Spawn a **single QA subagent** for this branch. The QA agent is responsible for running all four gates and producing a structured report. It does NOT merge anything.

Call the `Agent` tool with:

- `subagent_type`: `"general-purpose"`
- `isolation`: `"worktree"` — use the **existing** worktree branch `work/<task-group-name>-<agent-number>`
- `run_in_background`: `false` — wait for this agent to finish before continuing
- `mode`: `"auto"`
- `prompt`:

```
You are a QA agent. Your job is to run four quality gates on this branch and produce a structured report. You will NOT merge anything. You will NOT modify the feature branch.

Branch under review: work/<task-group-name>-<agent-number>
Task: <task title>
Spec file: <specFile>

Run each gate in order. For each gate, if issues are found, spawn a remediation subagent to fix them on THIS branch (work/<task-group-name>-<agent-number>), then re-run the gate to confirm the fix. Only mark a gate as PASS once it is clean.

## Gate 1 — Simplify
Run /simplify on the branch changes.
- Review for reuse, quality, and efficiency.
- If issues are found, spawn a refactoring agent on this worktree branch to fix them, then re-run.
- Record result as PASS or FAIL with a brief summary.

## Gate 2 — Review
Run /review on the branch changes.
- If any medium or higher severity issues are found, spawn an agent to address them on this worktree branch, then re-run.
- Record result as PASS or FAIL with a brief summary.

## Gate 3 — Security Review
Run the security-review skill: Skill({ skill: "security-review" })
- If any issues are found, spawn a secops developer agent to fix them on this worktree branch, then re-run.
- Record result as PASS or FAIL with a brief summary.

## Gate 4 — Security Scan
Run /security-scan on the branch changes.
- If any issues are found, spawn an agent to fix them on this worktree branch, then re-run.
- Record result as PASS or FAIL with a brief summary.

## Required output

After all gates are complete, output EXACTLY this block and nothing after it:

QA_REPORT_START
branch: work/<task-group-name>-<agent-number>
task: <task title>
gate_simplify: PASS | FAIL
gate_review: PASS | FAIL
gate_security_review: PASS | FAIL
gate_security_scan: PASS | FAIL
overall: PASS | FAIL
notes: <one-line summary of any remaining issues, or "none">
QA_REPORT_END

overall is PASS only if all four gates are PASS. Otherwise overall is FAIL.
```

#### 3c. Parse the QA report

Read the QA agent's output and extract the `QA_REPORT_START ... QA_REPORT_END` block. Parse each field.

If the output does not contain a valid `QA_REPORT_START` block, treat the branch as **FAIL** and report the raw output to the user.

#### 3d. Present results and ask for confirmation

Show the user a formatted summary:

```
QA Results — work/<task-group-name>-<agent-number> (<task title>)

  Simplify        [PASS|FAIL]
  Review          [PASS|FAIL]
  Security Review [PASS|FAIL]
  Security Scan   [PASS|FAIL]

  Overall: [PASS|FAIL]
  Notes: <notes>
```

Then use `AskUserQuestion` with options:

- If overall is **PASS**:
  - `"Merge this branch"` ✅
  - `"Skip this branch (do not merge)"` ⏭
  - `"Stop here"` 🛑

- If overall is **FAIL**:
  - `"Skip this branch (do not merge)"` ⏭
  - `"Merge anyway (I accept the risk)"` ⚠️
  - `"Stop here"` 🛑

If the user selects **"Stop here"**, stop immediately. Do not process any remaining branches. Tell the user they can re-run `/work-merge <task-group-name>` to resume (already-merged branches will conflict gracefully since they've been committed).

#### 3e. Merge (if confirmed)

If the user chose to merge:

```
git checkout <featureBranch>
git merge work/<task-group-name>-<agent-number> --no-ff -m "merge: <task-title>"
```

If there are merge conflicts, resolve them and report which files had conflicts.

Record this branch as merged.

---

### 4. Integration verification

After all branches have been processed (merged, skipped, or stopped), run the full verification suite on the feature branch:

```
build, lint, typecheck, test
```

Report the results. If verification fails, tell the user which checks failed before proceeding. Do not mark tasks complete if the integration suite fails.

### 5. Mark tasks complete

For each task whose branch was **successfully merged** (not skipped, not failed):

1. Read `.claude/tasks/<taskListName>.md`.
2. Find the line `- [ ] **<task title>**` and replace it with `- [x] **<task title>**`.
3. Write the updated file back using the `Edit` tool.
4. Re-read the file and confirm the change is present. If it is missing, fix it.

This step is mandatory. The task list file is the source of truth for progress and must reflect what was merged.

### 6. Clean up the session file

If all agents were processed (none skipped due to "Stop here"), delete the session state file:

```
.claude/work-sessions/<task-group-name>.json
```

If the user stopped early, leave the session file in place so it can be resumed.

### 7. Summary

Print a final summary:

- Feature branch: `feat/<task-group-name>`
- For each agent branch: task name, gate results, merged / skipped / failed
- Integration verification result
- Tasks marked complete in the task list (list them)
- Which groups are now unblocked for the next `/work` run
- Suggest: `git diff feat/<task-group-name> && /squash-pr` when ready
