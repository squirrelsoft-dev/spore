# Spec: Verify image builds and meets size target

> From: .claude/tasks/issue-14.md

## Objective

Validate that the multi-stage Dockerfile produces a working, correctly-sized Docker image. This is a verification-only task -- no code is written. The goal is to confirm four properties: (1) the Docker build completes without errors, (2) the final image meets the size target, (3) the container starts and begins its startup sequence, and (4) the binary inside is statically linked. If the image exceeds expectations, document the actual size and identify the contributing dependencies.

## Current State

- **No CI/CD infrastructure**: There are no GitHub Actions workflows or automated Docker build pipelines. All verification is manual.
- **No Dockerfile yet**: This task is blocked by "Create multi-stage Dockerfile". The Dockerfile does not exist on `main` at this time.
- **No `.dockerignore` yet**: Also blocked by Group 1 tasks (`.dockerignore` creation and release profile optimizations).
- **Existing test infrastructure**: `cargo test` runs unit and integration tests across the workspace. There are no Docker-specific tests.
- **Echo skill available**: `skills/echo.md` exists and uses the `anthropic` provider with `claude-haiku-4-5-20251001`. It has no tools (`tools: []`), making it the simplest skill for testing.

## Requirements

### R1: Docker build completes without errors

- **Command**: `docker build --build-arg SKILL_NAME=echo -t spore-echo .`
- **Pass criteria**: Exit code 0, no error messages in build output.
- **Fail criteria**: Any non-zero exit code, or errors during compilation, musl linking, or `COPY` steps.

### R2: Image size is under 15 MB (hard limit), ideally under 5 MB

- **Command**: `docker images spore-echo --format "{{.Size}}"`
- **Hard pass**: Image size < 15 MB.
- **Ideal pass**: Image size < 5 MB.
- **Acceptable**: Image size 5-15 MB. Document the actual size in a comment or PR description.
- **Fail**: Image size >= 15 MB.
- **Note**: With `tokio`, `axum`, `serde`, `rig-core`, `reqwest`, `rustls`, and `aws-lc-rs`, a realistic stripped static binary is 5-10 MB. The `[profile.release]` settings (`lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`) should bring it toward the lower end. The 1 MB target from the original issue is aspirational and not expected.

### R3: Container starts without crashing

- **Command**: `docker run --rm spore-echo` (with a timeout, since it will block waiting for connections)
- **Pass criteria**: The container produces log output showing at least the first startup steps before failing. Expected log sequence:
  1. `[1/7] Loading configuration` -- config loads (`SKILL_NAME=echo` is set via Dockerfile `ENV`)
  2. `[2/7] Registering tool entries` -- tool endpoints registered (defaults to `echo-tool=mcp://localhost:7001`)
  3. `[3/7] Connecting to tool servers` -- will attempt MCP connection to `localhost:7001`
- **Expected failure point**: The container will fail at step 3 (connecting to tool servers) because there is no MCP tool server running at `localhost:7001` inside the container. This is acceptable. Alternatively, if `TOOL_ENDPOINTS` is overridden to empty, it will fail at step 5 when building the agent due to missing `ANTHROPIC_API_KEY`.
- **Fail criteria**: The container exits immediately with a signal (SIGSEGV, SIGILL, SIGABRT) or produces no output at all, indicating the binary cannot execute (e.g., dynamic linker missing in scratch image).
- **Verification command**: `timeout 5 docker run --rm spore-echo 2>&1 || true` -- capture stderr (tracing writes to stderr) and allow the non-zero exit.
- **Alternative for cleaner startup**: `timeout 5 docker run --rm -e TOOL_ENDPOINTS="" spore-echo 2>&1 || true` -- skips tool registration, will fail at step 5 (missing API key) but confirms more startup steps succeed.

### R4: Binary is statically linked

- **Command**: Use a helper container to inspect the binary:
  ```
  docker create --name spore-echo-inspect spore-echo
  docker cp spore-echo-inspect:/agent-runtime /tmp/agent-runtime-check
  docker rm spore-echo-inspect
  file /tmp/agent-runtime-check
  ```
- **Pass criteria**: `file` output contains `statically linked`.
- **Fail criteria**: `file` output shows `dynamically linked` or references `ld-linux` / `ld-musl`.
- **Alternative** (if `file` is not available on the host): Build a small alpine container to inspect:
  ```
  docker run --rm -v /tmp/agent-runtime-check:/binary alpine file /binary
  ```

### R5: Document size breakdown if image exceeds 5 MB

- If the image exceeds 5 MB, the verifier should note:
  - The actual total image size.
  - The binary size (from `ls -lh /tmp/agent-runtime-check`).
  - The skill file size (check on the host: `ls -lh skills/echo.md`).
  - The CA certificate bundle size (typically ~200 KB, negligible).
  - A note that the primary contributor is the Rust binary itself, and which dependency families (TLS/crypto via `aws-lc-rs`, HTTP via `hyper`/`reqwest`, async runtime via `tokio`, LLM client via `rig-core`) account for the bulk.

## Implementation Details

The full verification sequence, intended to be run manually by the implementer:

```bash
# Step 1: Build the image
docker build --build-arg SKILL_NAME=echo -t spore-echo .
echo "Build exit code: $?"

# Step 2: Check image size
docker images spore-echo --format "table {{.Repository}}\t{{.Tag}}\t{{.Size}}"
SIZE_BYTES=$(docker inspect spore-echo --format='{{.Size}}')
SIZE_MB=$((SIZE_BYTES / 1048576))
echo "Image size: ${SIZE_MB} MB (${SIZE_BYTES} bytes)"

# Step 3: Run the container (expect failure, but should show startup logs)
echo "--- Container startup output (expect failure at tool connect or API key) ---"
timeout 5 docker run --rm -e TOOL_ENDPOINTS="" spore-echo 2>&1 || true
echo "--- End container output ---"

# Step 4: Check static linking
docker create --name spore-echo-inspect spore-echo
docker cp spore-echo-inspect:/agent-runtime /tmp/agent-runtime-check
docker rm spore-echo-inspect
file /tmp/agent-runtime-check
ls -lh /tmp/agent-runtime-check
rm -f /tmp/agent-runtime-check
```

### Expected output patterns

- **Build**: Final line should be something like `Successfully tagged spore-echo:latest`.
- **Image size**: A line showing `spore-echo latest <SIZE>` where SIZE is under 15 MB.
- **Container logs**: Lines containing `[1/7]`, `[2/7]`, and possibly `[3/7]` or up to `[5/7]` on stderr before an error about missing API key or connection failure.
- **Static linking**: `file` output should include `ELF 64-bit LSB executable, x86-64, ... statically linked, ... stripped`.

### Pass/fail summary

| Check | Pass | Fail |
|-------|------|------|
| Build completes | Exit code 0 | Non-zero exit code |
| Image size (hard) | < 15 MB | >= 15 MB |
| Image size (ideal) | < 5 MB | 5-15 MB (acceptable, document) |
| Container starts | Shows `[1/7]` log line | No output or signal crash |
| Static linking | `statically linked` in file output | `dynamically linked` in file output |

## Dependencies

- Blocked by: "Create multi-stage Dockerfile" (the Dockerfile must exist before it can be verified)
- Blocked by (transitively): "Add release profile optimizations to workspace `Cargo.toml`" and "Create `.dockerignore` file" (Group 1 tasks that the Dockerfile task depends on)
- Blocking: None

## Risks & Edge Cases

1. **Docker not available in the build environment**: The verification requires Docker to be installed and the Docker daemon to be running. In some CI or codespace environments, Docker-in-Docker may not be available or may require special configuration (`--privileged` mode, DinD sidecar).

2. **Architecture mismatch**: The Dockerfile targets `x86_64-unknown-linux-musl`. If the verifier is on an ARM machine (e.g., Apple Silicon Mac), the build will either fail or produce an emulated x86_64 build via QEMU, which will be extremely slow (30+ minutes) and may time out. In that case, use `docker buildx build --platform linux/amd64` explicitly.

3. **Build time**: A clean Docker build with no cache compiles the entire Rust dependency tree for the musl target. With `aws-lc-sys` (which requires cmake and a C compilation step), this can take 10-20 minutes even on a fast machine. Subsequent builds with the dependency cache layer intact should be 1-2 minutes.

4. **`FROM scratch` limitations**: The `FROM scratch` image has no shell, no `ls`, no utilities. You cannot `docker exec` into it for debugging. To inspect the image contents, use `docker create` + `docker cp` (as shown in the verification commands) or temporarily switch the Dockerfile to `FROM alpine` for debugging.

5. **Disk space**: The builder stage image can be large (1-2 GB with Rust toolchain + compiled artifacts). Ensure the Docker host has sufficient disk space. The final image is small, but the build cache is not.

6. **`SKILL_NAME` not set as env var**: The Dockerfile should set `ENV SKILL_NAME=echo` (or whatever the build arg was). If this is not done, the container will fail at step 1 with `missing required environment variable: 'SKILL_NAME'` rather than at step 3 or 5. This would indicate a Dockerfile bug, not a verification failure -- but it should be noted.

7. **Non-deterministic binary size**: The exact binary size may vary slightly between Rust compiler versions, dependency updates, or link-time optimization decisions. The size thresholds should be treated as approximate guidelines, not exact byte counts.

## Verification

To verify that the verification itself is correct:

1. **Positive control**: If the build succeeds and all checks pass, confirm by running `docker run --rm -p 8080:8080 -e ANTHROPIC_API_KEY=test-key -e TOOL_ENDPOINTS="" spore-echo` and sending `curl http://localhost:8080/health` -- the health endpoint should respond (even if the agent cannot actually call the LLM with a fake key, the HTTP server should be listening).

2. **Negative control**: Deliberately break the Dockerfile (e.g., remove the `COPY` for CA certificates) and verify that the build or runtime check catches the regression.

3. **Size regression detection**: Record the exact image size (in bytes) in the PR description. Future PRs that add dependencies can compare against this baseline to detect size regressions.
