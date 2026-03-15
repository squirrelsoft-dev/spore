---
description: 'Deep security scan of the project (on-demand)'
---

# Deep Security Scan

Run a comprehensive security analysis of the project. This is the thorough version — use for pre-deploy or periodic audits.

## Steps

Run each tool that is available (skip any that aren't installed):

1. **Gitleaks** — full repo history: `gitleaks detect --source=. --verbose`
2. **Semgrep** — SAST with extended rules: `semgrep --config=auto --config=p/owasp-top-ten`
3. **npm audit** — full dependency tree: `npm audit`
4. **Trivy** — filesystem scan: `trivy fs --severity HIGH,CRITICAL .`
5. **Knip** — dead code and unused deps: `npx knip`
6. **Madge** — circular dependencies: `npx madge --circular --extensions ts src/`

## Output

Summarize findings grouped by severity:

- 🔴 **Critical** — blocks deployment, must fix immediately
- 🟡 **High** — should fix before next release
- 🟢 **Medium/Low** — track and address when convenient

Include specific file locations, line numbers, and remediation steps for each finding.
