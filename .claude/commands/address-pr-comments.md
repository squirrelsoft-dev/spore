# Address PR Comments

Fetch, analyze, and address code review comments from a GitHub pull request.

## Steps

1. **Fetch PR info** — Run `gh pr view --json number,headRefName,headRepository` to get the current PR number and repo.
2. **Fetch comments** — Run both:
   - `gh api /repos/{owner}/{repo}/issues/{number}/comments` for PR-level comments
   - `gh api /repos/{owner}/{repo}/pulls/{number}/comments` for inline review comments
3. **Display comments** — Format and show all comments to the user, including diff hunks and file/line context for inline comments. Ignore bot comments (e.g. gemini-code-assist, dependabot).
4. **Analyze validity** — For each code review comment that suggests a change:
   - Read the referenced file and line to understand the current code
   - Evaluate whether the suggestion is valid (correct, improves quality, fixes a real bug)
   - Classify as: valid, partially valid, or not applicable
5. **Present findings** — Show the user a summary of which comments are valid and which are not, with reasoning.
6. **Plan fixes** — For all valid issues, enter plan mode and create a plan to address them. Present the plan to the user for approval before making any changes.
