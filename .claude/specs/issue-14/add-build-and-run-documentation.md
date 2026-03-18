# Spec: Add build and run documentation

> From: .claude/tasks/issue-14.md

## Objective

Add a "Docker" section to the project `README.md` that documents how to build, run, inspect, and debug Spore agent containers. This is the user-facing documentation for the Dockerfile created by the "Create multi-stage Dockerfile" task. Without this section, users would need to reverse-engineer the Dockerfile and source code to understand the build arguments, environment variables, and `FROM scratch` limitations.

## Current State

### README structure (`README.md`)

The README currently has these top-level sections in order:

1. **Why** (line 5) — motivation for the microservices approach
2. **How It Works** (line 11) — runtime + skill file explanation with example
3. **Architecture** (line 72) — directory tree, crate descriptions, agent tiers
4. **Self-Bootstrapping Factory** (line 109) — seed agents table
5. **Tech Stack** (line 122) — dependency table
6. **Key Properties** (line 131) — bullet points about design qualities
7. **Analogy** (line 142) — "serverless for agents" framing
8. **License** (line 147) — TBD placeholder

There is no Docker, deployment, or "getting started" section. The README mentions Docker images in several places (line 15: "Ships as a 1-5MB Docker image"; line 95: "The Docker image is `FROM scratch` + the binary + the skill file"; line 135: "1-5MB Docker images vs. gigabyte-scale typical AI containers") but never explains how to actually build or run one.

### Environment variables consumed by the runtime

From `crates/agent-runtime/src/config.rs` and `crates/agent-runtime/src/main.rs`:

| Variable | Required | Default | Source |
|---|---|---|---|
| `SKILL_NAME` | Yes | (none — errors if missing) | `config.rs` line 48, `read_required_var` |
| `SKILL_DIR` | No | `./skills` | `config.rs` line 49, `read_optional_var_or` |
| `BIND_ADDR` | No | `0.0.0.0:8080` | `config.rs` line 74-86, `parse_bind_addr` |
| `TOOL_ENDPOINTS` | No | `echo-tool=mcp://localhost:7001` | `main.rs` line 89-90, `register_tool_endpoints` |
| `ANTHROPIC_API_KEY` | Yes (if provider is `anthropic`) | (none — errors if missing) | `provider.rs` line 188, `read_api_key` |
| `OPENAI_API_KEY` | Yes (if provider is `openai`) | (none — errors if missing) | `provider.rs` line 166, `read_api_key` |
| `RUST_LOG` | No | (tracing default: INFO) | `main.rs` line 25-28, `EnvFilter::from_default_env` |

### HTTP endpoints

From `crates/agent-runtime/src/http.rs`:

- `POST /invoke` — sends a request to the agent (accepts `AgentRequest` JSON)
- `GET /health` — returns agent name, version, and health status

### Dockerfile (not yet created)

The Dockerfile is being created by the prerequisite task "Create multi-stage Dockerfile". Per the task description in `.claude/tasks/issue-14.md` (lines 23-33), it will:

- Accept `SKILL_NAME` as a build arg to select which skill file to copy
- Use `FROM scratch` for the runtime stage
- Set `SKILL_NAME` and `SKILL_DIR` as default ENV values
- Expose port 8080
- Set entrypoint to `/agent-runtime`

## Requirements

- Add a `## Docker` section to `README.md` positioned between "Tech Stack" (line 129) and "Key Properties" (line 131). This placement groups all technical/operational content together before the higher-level "Key Properties" and "Analogy" sections.
- The Docker section must include exactly five subsections documented below. Each subsection must use `###` heading level.
- **Build the image** (`### Build`): Show the `docker build` command with `--build-arg SKILL_NAME=echo -t spore-echo .`. Explain that `SKILL_NAME` selects which skill file from the `skills/` directory gets baked into the image.
- **Run the container** (`### Run`): Show the `docker run` command with `-p 8080:8080 -e ANTHROPIC_API_KEY=... spore-echo`. Include a curl example hitting `GET /health` to verify the container is running. Mention that the port maps to the default `BIND_ADDR` of `0.0.0.0:8080`.
- **Check image size** (`### Image Size`): Show `docker images spore-echo` and note the expected size range (realistically 5-10 MB, aspirationally under 5 MB). Frame the size as a key advantage over typical AI containers.
- **Environment variables** (`### Environment Variables`): Document all seven environment variables in a markdown table with columns: Variable, Required, Default, Description. Include `SKILL_NAME`, `SKILL_DIR`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `TOOL_ENDPOINTS`, `BIND_ADDR`, and `RUST_LOG`. Mark `SKILL_NAME` as set at build time (baked into the image as a default) but overridable at runtime. Mark the API key variables as conditionally required based on the provider specified in the skill file.
- **`FROM scratch` limitations** (`### Debugging`): Explain that `FROM scratch` images contain no shell, no package manager, and no debugging utilities. Document two workarounds: (1) `docker cp <container>:/agent-runtime ./agent-runtime` to extract the binary for local inspection, and (2) temporarily switching the Dockerfile's `FROM scratch` to `FROM alpine` to get a shell for interactive debugging. Note that `docker exec` will not work with `FROM scratch`.
- All code examples must use fenced code blocks with the `sh` or `bash` language identifier.
- Do not modify any existing sections of the README. Only insert the new Docker section.
- The documentation must be accurate relative to the actual source code (environment variable names, defaults, and behavior as documented in "Current State" above).

## Implementation Details

### Insertion point

Insert the new `## Docker` section after the "Tech Stack" table (after line 129, which is the last row of the tech stack table) and before `## Key Properties` (currently line 131). Add a blank line before and after the new section.

### Section content outline

```
## Docker

### Build

<paragraph explaining build args>
<code block: docker build command>

### Run

<paragraph explaining runtime config>
<code block: docker run command>
<code block: curl health check>

### Image Size

<code block: docker images command>
<paragraph about expected size>

### Environment Variables

<table with all 7 variables>

### Debugging

<paragraph about FROM scratch limitations>
<code block: docker cp workaround>
<paragraph about alpine workaround>
```

### Variable table format

Use this exact column layout for consistency with the existing "Tech Stack" table style:

```
| Variable | Required | Default | Description |
|---|---|---|---|
```

### Command examples

- Build: `docker build --build-arg SKILL_NAME=echo -t spore-echo .`
- Run: `docker run -p 8080:8080 -e ANTHROPIC_API_KEY=sk-... spore-echo`
- Health check: `curl http://localhost:8080/health`
- Image size: `docker images spore-echo`
- Copy binary out: `docker cp <container_id>:/agent-runtime ./agent-runtime`

## Dependencies

- Blocked by: "Create multi-stage Dockerfile" — the Docker section documents the Dockerfile, so the Dockerfile must exist first to ensure the documentation is accurate (build arg names, exposed ports, entrypoint, default ENV values).
- Blocking: None

## Risks & Edge Cases

- **Dockerfile not yet finalized**: Since this task is blocked by the Dockerfile creation task, the exact build arg names, default ENV values, and exposed ports documented here are based on the task description in `.claude/tasks/issue-14.md`. If the Dockerfile implementation deviates from the task description, this documentation must be updated to match. Mitigation: the spec captures the source-of-truth environment variables from the actual Rust source code, which will not change.
- **API key in command examples**: The `docker run` example includes `-e ANTHROPIC_API_KEY=...`. Using a placeholder like `sk-...` or `your-key-here` avoids accidentally encouraging users to paste real keys into shell history. The example must use a clearly-fake placeholder value.
- **Image size claims**: The README already claims "1-5MB Docker images" in multiple places (lines 15, 135). The realistic size with the full dependency tree (`tokio`, `axum`, `rig-core`, `reqwest`, `rustls`, `aws-lc-rs`) is 5-10 MB per the task file's implementation notes. The Docker section should state the realistic range and avoid contradicting the existing claims by framing it as "typically under 10 MB" rather than making a specific promise.
- **Provider-conditional API keys**: `ANTHROPIC_API_KEY` and `OPENAI_API_KEY` are only required when the skill file specifies the corresponding provider. The documentation must make this conditionality clear to avoid confusion (e.g., a user running an Anthropic-backed skill should not think `OPENAI_API_KEY` is also needed).
- **`TOOL_ENDPOINTS` format**: The format is `name=endpoint,name=endpoint` (comma-separated key=value pairs). This non-obvious format must be documented with an example value to prevent user confusion.

## Verification

- The `## Docker` section appears in `README.md` between `## Tech Stack` and `## Key Properties`.
- The section contains exactly five subsections: Build, Run, Image Size, Environment Variables, Debugging.
- All seven environment variables (`SKILL_NAME`, `SKILL_DIR`, `ANTHROPIC_API_KEY`, `OPENAI_API_KEY`, `TOOL_ENDPOINTS`, `BIND_ADDR`, `RUST_LOG`) are documented in the table.
- Variable names, defaults, and required/optional status match the source code in `config.rs`, `main.rs`, and `provider.rs`.
- All code examples use fenced code blocks with a language identifier.
- The `docker build` command includes `--build-arg SKILL_NAME=echo -t spore-echo .`.
- The `docker run` command includes `-p 8080:8080 -e ANTHROPIC_API_KEY=...`.
- The `docker images` command is present for size checking.
- The `FROM scratch` limitation is explained with both workarounds (`docker cp` and `FROM alpine`).
- No existing README sections are modified or removed.
- API key placeholders use clearly-fake values (not real key patterns).
- `cargo test` still passes (no code changes, but ensures nothing is broken by stale state).
