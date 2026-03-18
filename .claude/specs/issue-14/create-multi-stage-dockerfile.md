# Spec: Create multi-stage Dockerfile

> From: .claude/tasks/issue-14.md

## Objective

Create a multi-stage Dockerfile that compiles the `agent-runtime` binary as a fully static musl-linked binary and packages it in a minimal `FROM scratch` image alongside the skill file(s) and CA certificates. The goal is to produce a production-ready container image in the 5-15 MB range that can serve the HTTP API on port 8080 and make outbound HTTPS calls to LLM provider APIs.

## Current State

### Workspace structure

The project is a Cargo workspace with 6 members:

| Member | Path | Type | Has `lib.rs` | Has `main.rs` |
|--------|------|------|-------------|---------------|
| `agent-sdk` | `crates/agent-sdk` | library | yes | no |
| `skill-loader` | `crates/skill-loader` | library | yes | no |
| `tool-registry` | `crates/tool-registry` | library | yes | no |
| `agent-runtime` | `crates/agent-runtime` | binary + library | yes | yes |
| `orchestrator` | `crates/orchestrator` | binary | no | yes |
| `echo-tool` | `tools/echo-tool` | binary | no | yes (with `mod echo`) |

The workspace `Cargo.toml` at the root defines only `[workspace]` with `resolver = "2"` and the members list. A `[profile.release]` section with size optimizations (`lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`, `panic = "abort"`) is expected to be added by the prerequisite task before this Dockerfile is implemented.

### Dependency tree (TLS/crypto path)

The TLS chain is: `rig-core` -> `reqwest` -> `hyper-rustls` -> `rustls` -> `aws-lc-rs` -> `aws-lc-sys`. The `aws-lc-sys` crate (v0.38.0) has build-time dependencies on `cc`, `cmake`, `dunce`, and `fs_extra`. This means `cmake` must be installed in the builder stage. There is **no** dependency on `openssl` or `openssl-sys`, making a fully static musl build viable.

### Binary entry point

`agent-runtime` is the target binary. Its `main.rs` reads configuration via `RuntimeConfig::from_env()`, which requires:
- `SKILL_NAME` (required) -- name of the skill to load
- `SKILL_DIR` (optional, defaults to `./skills`) -- directory containing skill `.md` files
- `BIND_ADDR` (optional, defaults to `0.0.0.0:8080`) -- socket address to bind the HTTP server

### Skills directory

Skills live in `skills/` as Markdown files with YAML frontmatter:
- `echo.md`
- `cogs-analyst.md`
- `skill-writer.md`

### echo-tool module structure

The `echo-tool` binary has a non-trivial source layout: `src/main.rs` declares `mod echo;` and imports from `src/echo.rs`. The dependency-caching stub must account for this by creating both `src/main.rs` and `src/echo.rs` stubs.

## Requirements

1. The Dockerfile MUST be placed at the project root (`Dockerfile`).
2. The Dockerfile MUST use exactly two stages: a `builder` stage and a `runtime` stage.
3. The builder stage MUST use `rust:latest` as the base image.
4. The builder stage MUST install `musl-tools` and `cmake` via `apt-get`.
5. The builder stage MUST add the `x86_64-unknown-linux-musl` target via `rustup target add`.
6. The builder stage MUST implement the dependency-caching pattern: copy manifest files and create stub sources first, build to cache dependencies, then copy real sources and rebuild.
7. The dependency-caching pattern MUST create stub files for all 6 workspace members with the correct source file types (lib.rs for libraries, main.rs for binaries, plus echo.rs for echo-tool).
8. The builder stage MUST accept `SKILL_NAME` as a Docker `ARG` with a sensible default (e.g., `echo`).
9. The builder stage MUST build only the `agent-runtime` binary (not all workspace members) in release mode targeting `x86_64-unknown-linux-musl`.
10. The builder stage MUST verify the binary is statically linked using the `file` command (via `RUN file ... | grep -q 'statically linked'`).
11. The runtime stage MUST use `FROM scratch`.
12. The runtime stage MUST copy the compiled `agent-runtime` binary to `/agent-runtime`.
13. The runtime stage MUST copy the entire `skills/` directory from the build context (for flexibility at runtime).
14. The runtime stage MUST copy CA certificates from the builder stage (`/etc/ssl/certs/ca-certificates.crt`) to the same path.
15. The runtime stage MUST set `ENV SKILL_NAME` to the value of the build arg and `ENV SKILL_DIR` to `/skills`.
16. The runtime stage MUST set `ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt` so that `rustls` can locate the CA bundle.
17. The runtime stage MUST `EXPOSE 8080`.
18. The runtime stage MUST set `ENTRYPOINT ["/agent-runtime"]`.
19. The Dockerfile MUST NOT install `openssl` or `libssl-dev` -- TLS is handled entirely by `rustls`/`aws-lc-rs`.

## Implementation Details

### Stage 1: builder

```
FROM rust:latest AS builder
```

**System dependencies:**

```
RUN apt-get update && apt-get install -y musl-tools cmake \
    && rm -rf /var/lib/apt/lists/*
```

- `musl-tools`: provides `musl-gcc`, the linker wrapper needed for `x86_64-unknown-linux-musl`.
- `cmake`: required by `aws-lc-sys` (which builds the aws-lc C library from source).

**Rust target:**

```
RUN rustup target add x86_64-unknown-linux-musl
```

**Build arg:**

```
ARG SKILL_NAME=echo
```

**Dependency-caching pattern:**

The key insight is that `cargo build` caches compiled dependencies as long as the `Cargo.toml`/`Cargo.lock` files have not changed. By first copying only manifests and creating minimal stub source files, the expensive dependency compilation is cached in a Docker layer. Subsequent builds that only change application source code skip dependency compilation entirely.

Step 1 -- Create directory structure and copy workspace manifests:
```
WORKDIR /app

RUN mkdir -p crates/agent-sdk/src \
             crates/skill-loader/src \
             crates/tool-registry/src \
             crates/agent-runtime/src \
             crates/orchestrator/src \
             tools/echo-tool/src

COPY Cargo.toml Cargo.lock ./
COPY crates/agent-sdk/Cargo.toml crates/agent-sdk/Cargo.toml
COPY crates/skill-loader/Cargo.toml crates/skill-loader/Cargo.toml
COPY crates/tool-registry/Cargo.toml crates/tool-registry/Cargo.toml
COPY crates/agent-runtime/Cargo.toml crates/agent-runtime/Cargo.toml
COPY crates/orchestrator/Cargo.toml crates/orchestrator/Cargo.toml
COPY tools/echo-tool/Cargo.toml tools/echo-tool/Cargo.toml
```

Step 2 -- Create stub source files:

For **library** crates (`agent-sdk`, `skill-loader`, `tool-registry`), create an empty `src/lib.rs`. For **binary** crates (`orchestrator`, `echo-tool`), create a `src/main.rs` with `fn main() {}`. For `echo-tool`, also create a stub `src/echo.rs` (because the real `main.rs` contains `mod echo;`). For `agent-runtime` (both lib and bin), create both files:

```
RUN echo "" > crates/agent-sdk/src/lib.rs && \
    echo "" > crates/skill-loader/src/lib.rs && \
    echo "" > crates/tool-registry/src/lib.rs && \
    echo "" > crates/agent-runtime/src/lib.rs && \
    echo "fn main() {}" > crates/agent-runtime/src/main.rs && \
    echo "fn main() {}" > crates/orchestrator/src/main.rs && \
    echo "fn main() {}" > tools/echo-tool/src/main.rs && \
    touch tools/echo-tool/src/echo.rs
```

Step 3 -- Build dependencies only:
```
RUN cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime
```

Step 4 -- Copy real source and rebuild:
```
COPY crates/ crates/
COPY tools/ tools/
RUN cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime
```

**Static-link verification:**

```
RUN file target/x86_64-unknown-linux-musl/release/agent-runtime | grep -q 'statically linked'
```

### Stage 2: runtime

```
FROM scratch
```

**Copy binary:**
```
COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/agent-runtime /agent-runtime
```

**Copy skills (entire directory for runtime flexibility):**
```
COPY skills/ /skills/
```

**Copy CA certificates:**
```
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt
```

**Environment and entrypoint:**
```
ARG SKILL_NAME=echo
ENV SKILL_NAME=${SKILL_NAME}
ENV SKILL_DIR=/skills
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

EXPOSE 8080

ENTRYPOINT ["/agent-runtime"]
```

The `ARG` must be re-declared in the runtime stage because Docker ARGs do not cross stage boundaries. The `SSL_CERT_FILE` environment variable is the standard way to tell `rustls-native-certs` / `openssl-probe` where to find the CA bundle.

## Dependencies

- **Blocked by:** "Add release profile optimizations to workspace `Cargo.toml`" -- The `[profile.release]` with `lto = true`, `opt-level = "z"`, `codegen-units = 1`, `strip = true`, `panic = "abort"` must be present in the workspace `Cargo.toml` before building the Dockerfile, otherwise the binary will be 10-20 MB instead of 5-10 MB.
- **Blocking:** "Add build and run documentation" (needs to document `docker build` / `docker run` commands), "Verify image builds and meets size target" (needs the Dockerfile to exist).

## Risks & Edge Cases

1. **`aws-lc-sys` + musl build failure:** `aws-lc-sys` builds a C library (`aws-lc`) from source using CMake. Cross-compiling this for musl can fail if the musl toolchain is not properly configured. The `musl-tools` package on Debian provides `musl-gcc` which should work, but the CMake build may need the `CC` environment variable set explicitly to `musl-gcc`, or `CARGO_TARGET_X86_64_UNKNOWN_LINUX_MUSL_LINKER=musl-gcc` may need to be set. If this fails, add these environment variables to the Dockerfile.

2. **Architecture assumption:** The Dockerfile hardcodes `x86_64-unknown-linux-musl`. It will not work on ARM64 hosts without modification. Multi-arch support via `--platform` or `cross` could be a follow-up, but is out of scope.

3. **echo-tool stub compilation:** The `echo-tool` uses `rmcp` v1 while other crates use v0.16. The workspace allows this because they are separate dependencies. However, the stub must create both `src/main.rs` and `src/echo.rs` to satisfy the `mod echo;` declaration in the real source. If any workspace member adds new `mod` declarations in the future, the stubs will need updating.

4. **Cargo workspace resolution with stubs:** The stub `lib.rs` files are empty, which means any inter-crate `use` statements will fail during the stub build. This is expected -- the stub build may produce warnings or partial compilation errors for workspace crates, but the external dependency crates will still be compiled and cached.

5. **CA certificate path:** Different Linux distributions place CA certificates in different locations. `rust:latest` is Debian-based, so `/etc/ssl/certs/ca-certificates.crt` is correct. The `SSL_CERT_FILE` environment variable ensures `rustls-native-certs` and `openssl-probe` find the bundle regardless of compile-time assumptions.

6. **`FROM scratch` debugging limitations:** A scratch image has no shell, no `ls`, no debugging tools. If the binary crashes, the only diagnostic is the container exit code and any logs written to stdout/stderr. Temporarily switching to `FROM alpine` is useful for debugging.

7. **Skills directory vs single file:** The spec copies the entire `skills/` directory rather than just `skills/${SKILL_NAME}.md`. This is a deliberate tradeoff: slightly larger image (~4 KB for all 3 current skill files) in exchange for runtime flexibility (can override `SKILL_NAME` without rebuilding).

8. **Build context size:** Without a `.dockerignore` file, `docker build` will send the entire project directory (including `target/`, `.git/`, etc.) as context, which can be multiple gigabytes. The `.dockerignore` task in Group 1 should be completed first or simultaneously.

9. **Layer ordering for cache efficiency:** The `apt-get` and `rustup` steps should come before any `COPY` commands, since system dependencies change less frequently than Rust dependencies. This maximizes layer cache hits.

## Verification

1. **Build succeeds:** `docker build --build-arg SKILL_NAME=echo -t spore-echo .` completes without errors.
2. **Binary is statically linked:** The `file` command check in the builder stage serves as an in-build assertion.
3. **Image size is reasonable:** `docker images spore-echo --format '{{.Size}}'` should report under 15 MB. Target is 5-10 MB with release optimizations.
4. **Skills are present:** Runtime successfully loads the skill (logs `[4/7] Loading skill manifest` before failing on missing API keys).
5. **CA certificates work:** When run with a valid `ANTHROPIC_API_KEY`, the container can make HTTPS calls without TLS certificate errors.
6. **Environment variables are set:** `docker inspect spore-echo --format '{{.Config.Env}}'` should show `SKILL_NAME=echo`, `SKILL_DIR=/skills`, and `SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt`.
7. **Port is exposed:** `docker inspect spore-echo --format '{{.Config.ExposedPorts}}'` should show `8080/tcp`.
8. **Entrypoint is correct:** `docker inspect spore-echo --format '{{.Config.Entrypoint}}'` should show `[/agent-runtime]`.
9. **Dependency caching works:** Run `docker build` twice with only a source file change. The second build should skip the dependency compilation layer and complete significantly faster.
