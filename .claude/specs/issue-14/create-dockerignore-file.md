# Spec: Create `.dockerignore` file

> From: .claude/tasks/issue-14.md

## Objective

Create a `.dockerignore` file at the project root to minimize the Docker build context size. Without this file, `docker build` sends the entire working directory to the Docker daemon, including the `target/` directory (which can be several gigabytes of compiled artifacts), the `.git/` history, worktree clones, and other files irrelevant to the image build. A well-crafted `.dockerignore` reduces context transfer from gigabytes to kilobytes, making builds faster and avoiding accidental inclusion of secrets or large binaries.

## Current State

- No `.dockerignore` file exists in the project.
- No `Dockerfile` exists yet either (that is a separate downstream task in the same issue).
- The project root contains these directories and files relevant to exclusion decisions:
  - `target/` -- Rust build artifacts (multi-GB)
  - `.git/` -- Git history
  - `.worktrees/` -- Git worktree checkouts
  - `.claude/` -- Claude Code configuration, logs, specs, tasks
  - `README.md`, `CLAUDE.md` -- Markdown files at the root
  - `.DS_Store` -- macOS metadata
  - `.gitignore` -- Git ignore rules (useful reference but not needed in image)
  - `Cargo.lock`, `Cargo.toml` -- needed for the build (must NOT be excluded)
  - `crates/` -- Rust source code (must NOT be excluded)
  - `tools/` -- Rust tool source code (must NOT be excluded)
  - `skills/` -- Skill definition markdown files (must NOT be excluded; these are copied into the final Docker image)
- The existing `.gitignore` excludes: `target/`, `debug/`, `*.pdb`, `*.swp`, `*.swo`, `*~`, `.vscode/`, `.idea/`, `.DS_Store`, `Thumbs.db`, `.env`, `.env.*`, `CLAUDE.local.md`, and various `.claude/` subdirectories.
- Three skill files exist at `skills/echo.md`, `skills/cogs-analyst.md`, and `skills/skill-writer.md`.

## Requirements

1. **File location:** `.dockerignore` at the project root.

2. **Exclude build artifacts:** `target/` must be excluded. This is the single largest contributor to context size.

3. **Exclude version control:** `.git/` must be excluded. Git history is not needed inside the image and can be hundreds of megabytes.

4. **Exclude worktrees:** `.worktrees/` must be excluded. These are full repository clones used for parallel development.

5. **Exclude Claude Code configuration:** `.claude/` must be excluded. This contains logs, specs, tasks, agent configs, and other development tooling not needed at runtime.

6. **Exclude markdown files globally, then re-include `skills/`:** Use `*.md` to exclude all markdown files (README.md, CLAUDE.md, etc.), then use the negation pattern `!skills/` and `!skills/*.md` to re-include the skill definition files. The skill files (`skills/echo.md`, `skills/cogs-analyst.md`, `skills/skill-writer.md`) are required at runtime because the Dockerfile copies them into the final image for the agent to load.

7. **Exclude editor/IDE files:** `.vscode/`, `.idea/`, `*.swp`, `*.swo`, `*~` -- consistent with what `.gitignore` already excludes.

8. **Exclude OS metadata files:** `.DS_Store`, `Thumbs.db`.

9. **Exclude environment files:** `.env`, `.env.*` -- these may contain API keys and must never be sent to the Docker daemon.

10. **Exclude Cargo debug artifacts:** `*.pdb`, `debug/` -- consistent with `.gitignore`.

11. **Do NOT exclude:** `Cargo.toml`, `Cargo.lock`, `crates/`, `tools/`, `skills/`, `src/` -- these are all required for the Docker build.

## Implementation Details

**File to create:** `.dockerignore`

The file should be organized into clearly commented sections for maintainability. The key Docker-specific behavior to leverage:

- Lines starting with `#` are comments.
- Lines starting with `!` are negation patterns (re-include previously excluded paths).
- Patterns are evaluated top-to-bottom; later rules override earlier ones.
- `.dockerignore` uses Go's `filepath.Match` rules plus a `**` wildcard for directory matching.

**Recommended structure:**

```
# Build artifacts
target/
debug/
*.pdb

# Version control
.git/

# Worktrees
.worktrees/

# Claude Code
.claude/

# Documentation (skill files re-included below)
*.md

# Re-include skill files needed at runtime
!skills/
!skills/*.md

# Editor / IDE
.vscode/
.idea/
*.swp
*.swo
*~

# OS metadata
.DS_Store
Thumbs.db

# Environment / secrets
.env
.env.*

# Docker (not needed inside build context)
Dockerfile
.dockerignore
```

**Critical ordering note:** The `*.md` exclusion must appear BEFORE the `!skills/` and `!skills/*.md` negation patterns. Docker processes `.dockerignore` rules sequentially, and negation patterns only work if the path was previously excluded by an earlier rule.

**Dockerfile and .dockerignore self-exclusion:** Including `Dockerfile` and `.dockerignore` themselves is a common best practice -- they are not needed inside the build context since Docker reads them before sending the context.

## Dependencies

- Blocked by: None
- Blocking: None (the downstream "Create multi-stage Dockerfile" task does not strictly depend on this file existing first, but having it in place ensures the Dockerfile build is efficient from the start)

## Risks & Edge Cases

1. **Negation pattern ordering:** If `!skills/*.md` is placed before `*.md`, the negation has no effect because there is nothing to negate yet. The exclusion of `*.md` must come first. This is the most common mistake with `.dockerignore` negation patterns.

2. **Nested markdown files in crates:** Files like `crates/*/README.md` will be excluded by the `*.md` glob. This is acceptable because no Rust crate in this project uses markdown files at build time. If a crate ever adds a `build.rs` that reads a markdown file, the `.dockerignore` would need updating.

3. **New skill files in subdirectories:** If skill files are added in subdirectories of `skills/` (e.g., `skills/advanced/my-skill.md`), the pattern `!skills/*.md` would not match them. The pattern would need to be changed to `!skills/**/*.md`. For now, all three skill files are at the top level of `skills/`, so `!skills/*.md` is sufficient. The implementer should add a comment noting this limitation.

4. **`Cargo.lock` must not be excluded:** Some `.dockerignore` templates exclude `*.lock` files. This would break reproducible builds. The spec explicitly requires `Cargo.lock` to remain included.

5. **`.env` files and secrets:** Excluding `.env` and `.env.*` is a security measure. Even though `.gitignore` excludes them from version control, they could still exist on disk and be sent to the Docker daemon without a `.dockerignore` entry. The Docker daemon context is not encrypted and could be exposed in CI logs.

6. **No `**` prefix needed for top-level patterns:** In `.dockerignore`, patterns like `target/` are anchored to the build context root by default. Since `target/` only exists at the project root, `target/` is correct (not `**/target/`).

## Verification

1. **File exists at the project root:** Confirm `.dockerignore` exists at the repository root.
2. **Context size is small:** Run `docker build --no-cache -f /dev/null .` (or equivalent) and verify the context size is under 1 MB, not several GB.
3. **Skill files are included:** After creating the Dockerfile (downstream task), verify that `docker build` can `COPY skills/ /skills/` successfully -- meaning the skill `.md` files are present in the build context despite the `*.md` exclusion.
4. **No secrets in context:** Verify that `.env` patterns are excluded by creating a dummy `.env` file and confirming it is not sent to the daemon.
5. **Syntax is valid:** The file should contain no trailing whitespace on pattern lines, no blank negation patterns, and no Windows-style line endings (use LF, not CRLF).
6. **Consistency with `.gitignore`:** Visually confirm that all editor/IDE and OS patterns from `.gitignore` are also present in `.dockerignore`. The `.dockerignore` should be a superset of `.gitignore` for build-irrelevant files.
