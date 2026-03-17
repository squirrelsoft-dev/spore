---
description: 'Squash all group branches for a task list into one clean feature branch and open a PR'
---

# Squash & PR

Squash all `feat/<taskListName>-group-*` branches into a single clean feature branch named from the task list title, then open a pull request.

**Input:** `$ARGUMENTS` — task list name (e.g. `issue-3`). Maps to `.claude/tasks/$ARGUMENTS.md`. Required.

---

## Steps

### 1. Read the task list

Read `.claude/tasks/$ARGUMENTS.md`. If it doesn't exist, tell the user and stop.

Extract:
- **Title** — the first line, which matches `# Task Breakdown: <title>`. Strip the prefix to get the raw title (e.g. `Set up Turborepo monorepo structure`).
- **Issue number** — parse from `$ARGUMENTS` using the pattern `issue-<N>` → `<N>`. If the task list name doesn't follow that pattern, proceed without an issue number.
- **Groups** — all `## Group N — <label>` entries, with their tasks and completion status (`- [x]` vs `- [ ]`).

### 2. Derive the target branch name

Convert the raw title to kebab-case:
- Lowercase everything
- Replace spaces and special characters with `-`
- Collapse multiple dashes
- Example: `Set up Turborepo monorepo structure` → `feat/set-up-turborepo-monorepo-structure`

Store as `targetBranch`.

### 3. Detect base branch

Run `git branch -a` and determine whether the repo uses `main` or `master`. Store as `<base>`.

### 4. Find all group branches

Run:
```bash
git branch --list "feat/$ARGUMENTS-group-*"
```

Collect all matching branches in group-number order (group-1, group-2, etc.).

If no group branches are found, tell the user and stop. Suggest running `/work $ARGUMENTS --all` first.

Warn about any groups that are still incomplete in the task list (tasks with `- [ ]`). Ask the user if they want to continue anyway or stop.

### 5. Detect git platform

Run `git remote get-url origin` and classify:
- Contains `github.com` → **GitHub** (use `gh` CLI)
- Contains `gitlab.com` or self-hosted GitLab pattern → **GitLab** (use `glab` CLI)
- Contains `bitbucket.org` → **Bitbucket** (provide manual URL)

### 6. Create the target branch

Check if `targetBranch` already exists:
```bash
git branch --list "<targetBranch>"
```

If it exists, ask the user:
- `"Reset and rebuild it"` — delete and recreate from `<base>`
- `"Stop"` — halt so the user can handle it manually

If it does not exist, create it from `<base>`:
```bash
git checkout <base>
git checkout -b <targetBranch>
```

### 7. Merge all group branches in order

For each group branch in order:
```bash
git merge <group-branch> --no-ff --no-edit
```

If there are merge conflicts:
- Resolve by accepting incoming changes where unambiguous
- Document every conflict and resolution in the final summary
- Stage and complete the merge:
  ```bash
  git add -A
  git commit --no-edit
  ```

After all merges, confirm the result with:
```bash
git log --oneline <base>..HEAD
```

### 8. Squash into one commit

```bash
git reset --soft $(git merge-base <base> HEAD)
```

Generate a conventional commit message:

```
feat(<scope>): <title lowercase, imperative> (#<issue>)

<PR body — see step 9>

Closes #<issue>
```

- `scope` — the primary package or area affected, inferred from the task list (e.g. `monorepo`, `auth`, `db`)
- If no issue number, omit `(#<issue>)` and `Closes #<issue>`

Commit:
```bash
git commit -m "<message>"
```

### 9. Build the PR body

Construct the PR body from the task list structure:

```markdown
## Summary

<one paragraph describing what this PR does, derived from the task list title and group labels>

## Changes

### Group 1 — <label>
- ✅ Task title
- ✅ Task title

### Group 2 — <label>
- ✅ Task title
- ✅ Task title

...

## Test plan

- `turbo build` — all packages and apps build cleanly
- `turbo lint` — no lint errors
- `turbo typecheck` — no type errors
- `turbo dev` — verify apps start

## Issue

Closes #<issue>
```

For the test plan, infer appropriate verification steps from the task list content (build commands, dev server ports, key integration points mentioned in tasks).

### 10. Push the branch

```bash
git push --force-with-lease origin <targetBranch>
```

### 11. Open the PR

Check if a PR already exists for `targetBranch`:
- **GitHub:** `gh pr list --head <targetBranch>`
- **GitLab:** `glab mr list --source-branch <targetBranch>`

If a PR already exists, print the existing PR URL and stop.

If no PR exists, create one:

- **GitHub:**
  ```bash
  gh pr create \
    --title "feat(<scope>): <title> (#<issue>)" \
    --body "<PR body>" \
    --base <base>
  ```

- **GitLab:**
  ```bash
  glab mr create \
    --title "feat(<scope>): <title> (#<issue>)" \
    --description "<PR body>" \
    --target-branch <base>
  ```

- **Bitbucket:** Print the manual PR URL:
  ```
  https://bitbucket.org/<org>/<repo>/pull-requests/new?source=<targetBranch>
  ```

### 12. Summary

Print:

```
══════════════════════════════════
  Squash PR: $ARGUMENTS
══════════════════════════════════

  Task list:     .claude/tasks/$ARGUMENTS.md
  Title:         <raw title>
  Target branch: <targetBranch>
  Base branch:   <base>

  Group branches squashed:
    feat/$ARGUMENTS-group-1 — <label>
    feat/$ARGUMENTS-group-2 — <label>
    ...

  Commit: feat(<scope>): <title> (#<issue>)

  PR: <url>

══════════════════════════════════
```
