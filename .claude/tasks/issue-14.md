# Task Breakdown: Create minimal Dockerfile for static binary deployment

> Build a multi-stage Dockerfile that compiles `agent-runtime` as a fully static musl binary and packages it in a `FROM scratch` image with just the binary and skill files, targeting 1-5 MB total image size.

## Group 1 — Build configuration

_Tasks in this group can be done in parallel._

- [x] **Create `.dockerignore` file** `[S]`
      Create a `.dockerignore` at the project root to exclude `target/`, `.git/`, `.worktrees/`, `.claude/`, `*.md` (except skill files in `skills/`), and editor/IDE files from the Docker build context. This keeps the build context small and avoids sending gigabytes of compiled artifacts to the Docker daemon.
      Files: `.dockerignore`
      Blocking: None

- [x] **Add release profile optimizations to workspace `Cargo.toml`** `[S]`
      Add a `[profile.release]` section to the workspace-level `Cargo.toml` with size-optimizing settings: `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`. These are critical for hitting the 1-5 MB image size target. Without these, a Rust binary with `tokio`, `axum`, `serde`, `rig-core`, and `reqwest` will be 10-20 MB. With these settings it should come down to 5-8 MB stripped. The `panic = "abort"` is safe for a server binary and further reduces size by removing unwinding infrastructure.
      Files: `Cargo.toml`
      Blocking: "Create multi-stage Dockerfile"

## Group 2 — Dockerfile

_Depends on: Group 1._

- [x] **Create multi-stage Dockerfile** `[M]`
      Create a `Dockerfile` at the project root with two stages:

      **Stage 1 (builder):** Use `rust:latest` as base. Install `musl-tools` and `cmake` (needed for `aws-lc-sys` which is in the `rustls` dependency tree). Add the `x86_64-unknown-linux-musl` target via `rustup`. Use the dependency-caching pattern: copy all `Cargo.toml` and `Cargo.lock` files first with stub `src/` files, run `cargo build` to cache dependency compilation, then copy actual source and rebuild. Accept `SKILL_NAME` as a `ARG`. Verify the binary is statically linked with `file` command.

      **Stage 2 (runtime):** Use `FROM scratch`. Copy just the compiled binary from the builder stage. Copy `skills/${SKILL_NAME}.md` (or alternatively the entire `skills/` directory for flexibility). Copy CA certificates from the builder stage (`/etc/ssl/certs/ca-certificates.crt`) since the binary needs to make HTTPS calls to LLM provider APIs and `FROM scratch` has no CA bundle. Expose port 8080. Set `SKILL_NAME` and `SKILL_DIR` environment variables. Set entrypoint to `/agent-runtime`.

      The dependency-caching pattern requires creating stub files for all 6 workspace members: `crates/agent-sdk`, `crates/skill-loader`, `crates/tool-registry`, `crates/agent-runtime`, `crates/orchestrator`, and `tools/echo-tool`. This ensures `cargo build` caches dependency compilation separately from application code compilation, making iterative builds much faster.
      Files: `Dockerfile`
      Blocked by: "Add release profile optimizations to workspace `Cargo.toml`"
      Blocking: "Add build and run documentation", "Verify image builds and meets size target"

## Group 3 — Documentation and verification

_Depends on: Group 2._

- [x] **Add build and run documentation** `[S]`
      Add a "Docker" section to the project `README.md` documenting: (1) how to build the image (`docker build --build-arg SKILL_NAME=echo -t spore-echo .`), (2) how to run the container (`docker run -p 8080:8080 -e ANTHROPIC_API_KEY=... spore-echo`), (3) how to check the image size (`docker images spore-echo`), (4) the key environment variables (`SKILL_NAME`, `SKILL_DIR`, `ANTHROPIC_API_KEY`, `TOOL_ENDPOINTS`, `BIND_ADDR`), (5) a note about the `FROM scratch` limitation (no shell, debugging requires `docker cp` or temporarily switching to `FROM alpine`).
      Files: `README.md`
      Blocked by: "Create multi-stage Dockerfile"
      Blocking: None

- [x] **Verify image builds and meets size target** `[S]`
      Build the Docker image with `docker build --build-arg SKILL_NAME=echo -t spore-echo .`. Verify: (1) the build completes without errors, (2) `docker images spore-echo` shows the image is under 15 MB (ideally under 5 MB), (3) `docker run --rm spore-echo` starts without crashing (it will fail to connect to LLM APIs without keys, but should at least begin the startup sequence), (4) the binary inside is statically linked. If the image exceeds 5 MB, document the actual size and note which dependencies contribute most.
      Files: (none — command-line verification only)
      Blocked by: "Create multi-stage Dockerfile"
      Blocking: None

## Implementation Notes

1. **TLS via `rustls`, not `openssl`**: The dependency tree uses `rustls` with `aws-lc-rs` as the crypto backend. This avoids linking against `libssl`/`libcrypto` and makes `FROM scratch` viable. However, `aws-lc-sys` requires `cmake` in the builder stage.

2. **CA certificates are required**: Since `rig-core` uses `reqwest` to call LLM APIs over HTTPS, the final image must include CA certificates. Copy `/etc/ssl/certs/ca-certificates.crt` from the builder stage.

3. **`SKILL_NAME` as build arg vs runtime env**: The issue specifies `SKILL_NAME` as a build arg. The recommended approach is to use it as a build arg that selects which skill file to copy into the image, and also set it as a default `ENV` that the runtime reads. This allows the same image to be overridden at runtime with `-e SKILL_NAME=other` if multiple skills are copied.

4. **No CI/CD yet**: There are no GitHub Actions workflows. Docker image CI could be a follow-up issue.

5. **Depends on completed issues #11 and #12**: The HTTP server (issue #12, merged in commit `3bc7d6b`) and skill loading (issue #11) are already implemented. The Dockerfile should produce a working image.

6. **Image size reality check**: With `tokio`, `axum`, `serde`, `rig-core`, `reqwest`, `rustls`, and `aws-lc-rs`, the stripped binary will likely be 5-10 MB. The 1 MB target from the issue is aspirational; 5-10 MB is realistic and still a massive improvement over typical AI container images.
