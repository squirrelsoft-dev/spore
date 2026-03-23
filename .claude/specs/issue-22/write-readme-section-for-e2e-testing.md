# Spec: Write README section for E2E testing

> From: .claude/tasks/issue-22.md

## Objective

Add a new section to `README.md` that documents how to run the end-to-end self-bootstrapping pipeline test. The section should cover prerequisites, commands, expected runtime, debugging guidance, the `--no-cleanup` flag, and cost considerations.

## Current State

### README.md

The README currently has the following top-level sections in order:

1. Why
2. How It Works
3. Architecture
4. Self-Bootstrapping Factory
5. Tech Stack
6. Docker (with subsections: Build, Run, Image Size, Environment Variables, Debugging)
7. Key Properties
8. Analogy
9. License

There is no mention of testing, E2E validation, or the self-bootstrapping pipeline test anywhere in the README.

### E2E infrastructure (created by prior tasks)

- `scripts/e2e-test.sh` -- top-level shell script test driver with `--no-cleanup` and `--timeout` flags
- `docker-compose.e2e.yml` -- multi-container test environment definition
- `tests/e2e/SCENARIO.md` -- test scenario document (temperature-conversion agent)
- `tests/e2e/validate_step{1-4}_*.sh` -- per-step validators
- `tests/e2e_bootstrap_test.rs` -- Rust integration test wrapper gated behind `#[ignore]` and `#[cfg(feature = "e2e")]`

## Requirements

- Add a new `## E2E Testing` section to `README.md`.
- Insert it after the `Docker` section (after the `Debugging` subsection, before `Key Properties`). This placement groups all operational/runtime concerns together.
- The section must cover the following subsections:

### Prerequisites subsection

Document what is needed before running the E2E test:
- Docker and Docker Compose installed and running
- A valid `ANTHROPIC_API_KEY` environment variable (the seed agents use Anthropic models)
- Sufficient API credits (see Cost Considerations below)
- All project crates building successfully (`cargo build`)

### Running the test subsection

Document two ways to run the E2E test:

1. **Shell script directly**: `./scripts/e2e-test.sh`
2. **Via cargo**: `cargo test --features e2e -- --ignored e2e_bootstrap_test`

Note that the cargo approach requires the `e2e` feature flag and the `--ignored` flag since the test is marked `#[ignore]`.

### Expected runtime subsection

State that the full pipeline takes approximately 5-10 minutes. Factors that affect runtime: Docker image build time, LLM response latency, and tool compilation. The `--timeout` flag on the shell script controls the overall timeout (default: 10 minutes).

### Debugging failures subsection

Document the debugging workflow:
- All intermediate artifacts are saved to `tests/e2e/artifacts/` (generated skill file, tool code, step responses, Docker build logs)
- On failure, the script automatically dumps container logs
- Use the `--no-cleanup` flag (`./scripts/e2e-test.sh --no-cleanup`) to keep Docker containers running after the test finishes, enabling manual inspection with `docker compose -f docker-compose.e2e.yml logs <service>` and direct `curl` calls against the running services
- Each step's output can be examined independently in the artifacts directory

### Cost considerations subsection

Document that each E2E run makes multiple LLM API calls:
- Minimum 4 calls: skill-writer, tool-coder, deploy-agent, and the generated temperature-agent
- With retries (e.g., tool-coder may retry up to 3 times if generated code fails to compile), this could be 8-12 calls per run
- Advise running the E2E test deliberately rather than as part of routine development iteration

## Implementation Details

### Files to modify

**`README.md`**

Insert a new `## E2E Testing` section between the existing `### Debugging` subsection (line 184, end of Docker section) and the `## Key Properties` section (line 186). The new section should use the following structure:

```
## E2E Testing

<brief intro paragraph explaining what the E2E test validates: the full self-bootstrapping pipeline from natural language description to a running, routable agent>

### Prerequisites

<bulleted list>

### Running the Test

<two approaches with code blocks>

### Expected Runtime

<paragraph>

### Debugging Failures

<bulleted list with code blocks for commands>

### Cost Considerations

<paragraph with bullet points>
```

### Style guidance

- Match the existing README tone: concise, technical, no filler
- Use code blocks for all commands
- Use the same heading hierarchy as the Docker section (h2 for the section, h3 for subsections)
- Keep the entire section under 60 lines of markdown to avoid bloating the README

## Dependencies

- Blocked by: "Add Rust integration test wrapper" (the cargo command and feature flag must exist before documenting them)
- Blocking: None

## Risks & Edge Cases

- **Premature documentation**: Since this task is blocked by the Rust integration test wrapper, the documented cargo command (`cargo test --features e2e -- --ignored e2e_bootstrap_test`) must match whatever the wrapper task actually implements. If the feature name or test name changes, the README must be updated accordingly.
- **Cost estimates may drift**: The "8-12 calls" estimate is based on the current pipeline design. If steps are added or retry logic changes, the cost section should be updated.
- **No CI integration yet**: The README should not claim CI support since there is no `.github/workflows/` directory. Avoid mentioning CI pipelines.

## Verification

- `README.md` parses as valid markdown (no broken links, no unclosed code blocks).
- The new section appears between `Docker` and `Key Properties`.
- All six topics are covered: prerequisites, commands, runtime, debugging, `--no-cleanup` flag, cost considerations.
- No new files are created -- this task only modifies `README.md`.
- The section uses h2 (`##`) for the top-level heading and h3 (`###`) for subsections, consistent with the Docker section pattern.
- The section is under 60 lines of markdown.
