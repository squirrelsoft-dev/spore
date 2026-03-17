# Spec: Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml

> From: .claude/tasks/issue-9.md

## Objective

Add the `rmcp` and `tokio` crates as dependencies of the `tool-registry` crate so that downstream tasks can implement real MCP client connections over TCP and Unix socket transports. The `rmcp` crate provides `serve_client`, `Peer<RoleClient>`, `ServerSink`, and `ClientHandler` (via the `client` feature) and `IntoTransport` for async streams (via `transport-async-rw`). The `tokio` crate provides `TcpStream` and `UnixStream` (via the `net` feature). Without these dependencies, none of the transport, connection, or handle tasks in Groups 2-3 can proceed.

## Current State

`crates/tool-registry/Cargo.toml` is minimal:

```toml
[package]
name = "tool-registry"
version = "0.1.0"
edition = "2024"

[dependencies]
```

There are no dependencies, no dev-dependencies section, and no feature flags. The workspace `Cargo.toml` at the repo root uses `resolver = "2"` and does not define workspace-level dependencies. The crate currently contains only `src/lib.rs` with a stub `pub struct ToolRegistry;`.

## Requirements

1. Add `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }` to `[dependencies]`.
   - Must pin to `0.16` (not `0.16.x`, not `^0.16`, not `1.x`) because `rig-core 0.32` depends on `rmcp ^0.16`. Using rmcp 1.x would cause a Cargo version conflict when `rig-core` is added to `agent-runtime` later.
2. Add `tokio = { version = "1", features = ["net"] }` to `[dependencies]`.
   - The `net` feature provides `tokio::net::TcpStream` and `tokio::net::UnixStream`, needed by the transport module.
3. Add a `[dev-dependencies]` section with `tokio = { version = "1", features = ["macros", "rt", "net"] }` for async test support.
   - The `macros` feature enables `#[tokio::test]`.
   - The `rt` feature enables the tokio runtime required by `#[tokio::test]`.
   - The `net` feature is needed for integration tests that create in-process TCP/Unix listeners.
4. No other dependencies should be added.
5. `cargo check -p tool-registry` must succeed after the change.
6. `cargo clippy -p tool-registry` must produce no warnings.
7. Existing `cargo test` must continue to pass (currently no tests, so this is a no-regression check).

## Implementation Details

- **File to modify**: `crates/tool-registry/Cargo.toml`
- **Changes**:
  - Under `[dependencies]`, add two lines:
    ```toml
    rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }
    tokio = { version = "1", features = ["net"] }
    ```
  - Add a new `[dev-dependencies]` section:
    ```toml
    [dev-dependencies]
    tokio = { version = "1", features = ["macros", "rt", "net"] }
    ```
- **No code changes**: The `src/lib.rs` file does not need to be modified for this task. The dependencies just need to resolve and compile.

### Why these specific features

| Crate  | Feature                | Provides                                                        |
|--------|------------------------|-----------------------------------------------------------------|
| `rmcp` | `client`               | `serve_client`, `Peer<RoleClient>`, `ServerSink`, `ClientHandler` |
| `rmcp` | `transport-async-rw`   | `IntoTransport` impl for `TcpStream`, `UnixStream`              |
| `tokio`| `net`                  | `tokio::net::TcpStream`, `tokio::net::UnixStream`               |
| `tokio`| `macros` (dev)         | `#[tokio::test]` attribute macro                                 |
| `tokio`| `rt` (dev)             | Tokio runtime for `#[tokio::test]`                               |

### Version pinning rationale

`rig-core 0.32` declares `rmcp = "^0.16"` in its dependencies. Cargo's semver resolution means `^0.16` resolves to `>=0.16.0, <0.17.0`. If we used `rmcp = "1"` here, Cargo would pull in two incompatible versions of rmcp (0.16.x for rig-core and 1.x for tool-registry), and types from one would not be compatible with the other. Pinning to `version = "0.16"` ensures a single shared rmcp version across the workspace.

## Dependencies

- **Blocked by**: Nothing -- this is a Group 1 task with no prerequisites.
- **Blocking**:
  - "Create transport module with endpoint URL parsing" (Group 2) -- needs `rmcp` and `tokio` types
  - "Implement `connect()` with real MCP client logic" (Group 3) -- needs `rmcp` client APIs
  - "Add `McpHandle` field to `ToolEntry`" (Group 2) -- needs `rmcp` types for `RunningService`

## Risks & Edge Cases

1. **rmcp 0.16 not available on crates.io**: If the version is yanked or does not exist, `cargo check` will fail during verification. Mitigation: verify the version exists before starting implementation (`cargo search rmcp` or check crates.io).
2. **Feature flag names changed**: The `client` and `transport-async-rw` feature names are specific to rmcp 0.16. If they differ, compilation will fail with "unknown feature" errors. Mitigation: check the rmcp 0.16 `Cargo.toml` on crates.io to confirm feature names.
3. **Tokio version conflict**: Unlikely since tokio `1` is broadly compatible, but if another workspace crate pins a specific tokio minor version, there could be friction. Mitigation: use the broad `version = "1"` specifier, which Cargo resolves flexibly.
4. **Build time increase**: Adding rmcp and tokio increases compile times. This is expected and acceptable since these are core runtime dependencies needed for MCP protocol support.

## Verification

1. Run `cargo check -p tool-registry` -- must succeed with no errors.
2. Run `cargo clippy -p tool-registry` -- must produce no warnings.
3. Run `cargo test -p tool-registry` -- must pass (no regressions).
4. Run `cargo tree -p tool-registry` and confirm:
   - `rmcp v0.16.x` appears (not 1.x)
   - `tokio v1.x` appears with the `net` feature
5. Inspect `crates/tool-registry/Cargo.toml` and confirm:
   - `[dependencies]` contains exactly `rmcp` and `tokio` with the specified versions and features.
   - `[dev-dependencies]` contains `tokio` with `macros`, `rt`, and `net` features.
