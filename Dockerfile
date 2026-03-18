FROM rust:latest AS builder

RUN apt-get update && apt-get install -y musl-tools cmake \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

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

RUN echo "" > crates/agent-sdk/src/lib.rs && \
    echo "" > crates/skill-loader/src/lib.rs && \
    echo "" > crates/tool-registry/src/lib.rs && \
    echo "" > crates/agent-runtime/src/lib.rs && \
    echo "fn main() {}" > crates/agent-runtime/src/main.rs && \
    echo "fn main() {}" > crates/orchestrator/src/main.rs && \
    echo "fn main() {}" > tools/echo-tool/src/main.rs && \
    touch tools/echo-tool/src/echo.rs

RUN cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime

COPY crates/ crates/
COPY tools/ tools/
# Touch .rs files to invalidate cargo's mtime-based cache after COPY overwrites stubs
RUN find crates/ tools/ -name '*.rs' -exec touch {} + && \
    cargo build --release --target x86_64-unknown-linux-musl -p agent-runtime

RUN file target/x86_64-unknown-linux-musl/release/agent-runtime | grep -qE 'static(-pie)? linked'

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/agent-runtime /agent-runtime
COPY skills/ /skills/
COPY --from=builder /etc/ssl/certs/ca-certificates.crt /etc/ssl/certs/ca-certificates.crt

ARG SKILL_NAME=echo
ENV SKILL_NAME=${SKILL_NAME}
ENV SKILL_DIR=/skills
ENV SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt

EXPOSE 8080

USER 1000

ENTRYPOINT ["/agent-runtime"]
