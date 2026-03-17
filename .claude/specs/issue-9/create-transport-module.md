# Spec: Create transport module with endpoint URL parsing

> From: .claude/tasks/issue-9.md

## Objective

Create `crates/tool-registry/src/transport.rs` with an async function that establishes MCP client connections over TCP or Unix socket transports, plus a URL-parsing helper that determines which transport to use based on the endpoint scheme. This module is the low-level connection plumbing that the `ToolRegistry::connect()` method (Group 3) will delegate to, keeping transport concerns isolated from registry logic.

## Current State

- `crates/tool-registry/src/lib.rs` contains only `pub struct ToolRegistry;` (placeholder). Issue #8 tasks will replace this with module declarations, `ToolEntry`, `RegistryError`, and `ToolRegistry` with core methods.
- `RegistryError` (defined by issue #8 in `crates/tool-registry/src/registry_error.rs`) has a `ConnectionFailed { endpoint: String, reason: String }` variant specifically designed for transport failures.
- `crates/tool-registry/Cargo.toml` currently has no dependencies. The sibling task "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" (Group 1, issue #9) will add:
  - `rmcp = { version = "0.16", features = ["client", "transport-async-rw"] }`
  - `tokio = { version = "1", features = ["net"] }`
- The `rmcp` crate (pinned to 0.16 for `rig-core 0.32` compatibility) provides:
  - `rmcp::service::RunningService<RoleClient, ()>` -- the handle returned after a successful MCP handshake.
  - `rmcp::model::RoleClient` -- the client role marker type.
  - `rmcp::handler::client::ClientHandler` -- trait implemented by `()` with default no-op behavior.
  - The `transport-async-rw` feature enables `IntoTransport` for any `AsyncRead + AsyncWrite` type, including `tokio::net::TcpStream` and `tokio::net::unix::UnixStream`.
  - `().serve(stream).await` initiates the MCP client handshake on the given transport stream.
- The project follows a one-type-per-file convention, manual error `Display` implementations (no `thiserror`), and a 50-line function limit.
- The codebase uses `std::sync::RwLock` for synchronous operations, but this module will use `tokio::net` async I/O since transport connections are inherently async.

## Requirements

1. Create a new file `crates/tool-registry/src/transport.rs`.
2. Define a `pub(crate)` enum `TransportTarget` with exactly two variants:
   - `Tcp { host: String, port: u16 }` -- for `mcp://host:port` endpoints.
   - `Unix { path: PathBuf }` -- for `mcp+unix:///path` endpoints.
3. Implement `pub(crate) fn parse_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError>` that:
   - Strips the scheme prefix and dispatches based on it.
   - For `mcp://`: extracts host and port, returning `TransportTarget::Tcp`. Port is required (no default port). Host can be a hostname or IP address.
   - For `mcp+unix://`: extracts the path component, returning `TransportTarget::Unix`. The path is the portion after `mcp+unix://` (e.g., `mcp+unix:///var/run/tool.sock` yields `/var/run/tool.sock`).
   - For any other scheme or malformed input: returns `Err(RegistryError::ConnectionFailed)` with a descriptive reason.
4. Implement `pub(crate) async fn connect_transport(endpoint: &str) -> Result<RunningService<RoleClient, ()>, RegistryError>` that:
   - Calls `parse_endpoint(endpoint)` to determine the transport target.
   - For `TransportTarget::Tcp`: calls `TcpStream::connect(format!("{host}:{port}")).await`, then `().serve(stream).await`.
   - For `TransportTarget::Unix`: calls `UnixStream::connect(path).await`, then `().serve(stream).await`.
   - Maps all I/O errors and MCP handshake errors to `RegistryError::ConnectionFailed { endpoint, reason }` where `reason` is the error's `Display` output.
5. Keep every function under 50 lines.
6. Include `#[cfg(test)] mod tests` with unit tests for `parse_endpoint` (see Verification section). Do NOT include integration tests for `connect_transport` -- those require real MCP servers and are handled by the "Write MCP connection integration tests" task in Group 5.
7. The `TransportTarget` enum must derive `Debug` and `PartialEq` (for test assertions). It does NOT need `Clone`, `Serialize`, or `Deserialize`.

## Implementation Details

### File to create

**`crates/tool-registry/src/transport.rs`**

### Imports

```rust
use std::path::PathBuf;

use rmcp::model::RoleClient;
use rmcp::service::RunningService;
use rmcp::ServiceExt;
use tokio::net::TcpStream;
#[cfg(unix)]
use tokio::net::UnixStream;

use crate::registry_error::RegistryError;
```

Note: `UnixStream` is only available on Unix platforms. Gate its import and the Unix connection path behind `#[cfg(unix)]`.

### `TransportTarget` enum

```rust
#[derive(Debug, PartialEq)]
pub(crate) enum TransportTarget {
    Tcp { host: String, port: u16 },
    #[cfg(unix)]
    Unix { path: PathBuf },
}
```

Crate-internal only (`pub(crate)`) -- not part of the crate's public API. Downstream code interacts with `connect_transport`, not `TransportTarget`.

### `parse_endpoint` function

```rust
pub(crate) fn parse_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError> {
    // ...
}
```

Parsing logic:

1. Check if `endpoint` starts with `mcp+unix://` -- if so, extract the path after the scheme prefix. If the path is empty, return an error. Otherwise return `TransportTarget::Unix { path }`.
2. Check if `endpoint` starts with `mcp://` -- if so, extract the `host:port` portion after the scheme prefix. Split on `:` to separate host and port. Parse port as `u16`. If host is empty, port is missing, or port is not a valid `u16`, return an error.
3. Otherwise, return `Err(RegistryError::ConnectionFailed)` with reason indicating an unsupported or missing scheme.

The order matters: check `mcp+unix://` before `mcp://` since `mcp+unix://` starts with `mcp` too, and using `strip_prefix` ensures correct matching.

Error construction for all parse failures:
```rust
RegistryError::ConnectionFailed {
    endpoint: endpoint.to_string(),
    reason: "<descriptive message>".to_string(),
}
```

### `connect_transport` function

```rust
pub(crate) async fn connect_transport(
    endpoint: &str,
) -> Result<RunningService<RoleClient, ()>, RegistryError> {
    // ...
}
```

Implementation:

1. Call `parse_endpoint(endpoint)?`.
2. Match on the `TransportTarget`:
   - `Tcp { host, port }`: Connect via `TcpStream::connect(format!("{host}:{port}")).await`, map errors, then `().serve(stream).await`, map errors.
   - `Unix { path }`: Connect via `UnixStream::connect(&path).await`, map errors, then `().serve(stream).await`, map errors.
3. Error mapping helper -- use a closure or inline `.map_err` to convert `std::io::Error` and rmcp errors to `RegistryError::ConnectionFailed`:
   ```rust
   .map_err(|e| RegistryError::ConnectionFailed {
       endpoint: endpoint.to_string(),
       reason: e.to_string(),
   })
   ```

### `serve` call details

The rmcp `ServiceExt::serve` method (from the `client` feature) takes a transport and initiates the MCP client handshake. The call pattern is:

```rust
let service = ().serve(stream).await.map_err(|e| RegistryError::ConnectionFailed {
    endpoint: endpoint.to_string(),
    reason: e.to_string(),
})?;
```

Here `()` is the `ClientHandler` implementation (unit type provides default no-op behavior) and `stream` is either a `TcpStream` or `UnixStream` that implements `AsyncRead + AsyncWrite`.

### Unit tests

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_tcp_endpoint_with_hostname() { ... }

    #[test]
    fn parse_tcp_endpoint_with_ip_address() { ... }

    #[test]
    fn parse_unix_endpoint() { ... }

    #[test]
    fn parse_endpoint_missing_scheme_returns_error() { ... }

    #[test]
    fn parse_endpoint_unknown_scheme_returns_error() { ... }

    #[test]
    fn parse_tcp_endpoint_missing_port_returns_error() { ... }

    #[test]
    fn parse_tcp_endpoint_invalid_port_returns_error() { ... }

    #[test]
    fn parse_unix_endpoint_empty_path_returns_error() { ... }
}
```

Test expectations:
- `parse_endpoint("mcp://localhost:7001")` returns `Ok(TransportTarget::Tcp { host: "localhost".into(), port: 7001 })`.
- `parse_endpoint("mcp://192.168.1.10:8080")` returns `Ok(TransportTarget::Tcp { host: "192.168.1.10".into(), port: 8080 })`.
- `parse_endpoint("mcp+unix:///var/run/tool.sock")` returns `Ok(TransportTarget::Unix { path: PathBuf::from("/var/run/tool.sock") })`.
- `parse_endpoint("localhost:7001")` returns `Err(RegistryError::ConnectionFailed { ... })`.
- `parse_endpoint("http://localhost:7001")` returns `Err(RegistryError::ConnectionFailed { ... })`.
- `parse_endpoint("mcp://localhost")` returns `Err(RegistryError::ConnectionFailed { ... })`.
- `parse_endpoint("mcp://localhost:abc")` returns `Err(RegistryError::ConnectionFailed { ... })`.
- `parse_endpoint("mcp+unix://")` returns `Err(RegistryError::ConnectionFailed { ... })`.

All error assertions use `assert!(result.is_err())` and optionally match on the `RegistryError::ConnectionFailed` variant to verify the `endpoint` field equals the input and the `reason` field is non-empty. Use `PartialEq`-based matching where practical.

Unix-specific tests (`parse_unix_endpoint`, `parse_unix_endpoint_empty_path_returns_error`) should be gated with `#[cfg(unix)]` since the `Unix` variant itself is `#[cfg(unix)]`.

### Integration with `lib.rs`

This file will be declared as `mod transport;` in `lib.rs` (no `pub use` needed since everything is `pub(crate)`). This wiring is handled either by this task or by a Group 3 task -- the implementer should add the `mod transport;` declaration when creating the file so the module is visible to sibling modules (specifically `tool_registry.rs` which calls `transport::connect_transport` in the Group 3 "Implement `connect()`" task).

## Dependencies

- **Blocked by:**
  - "Add `rmcp` and `tokio` dependencies to tool-registry Cargo.toml" (Group 1, issue #9) -- `rmcp` and `tokio` must be in `Cargo.toml` for imports to resolve.
  - "Define `RegistryError` enum" (Group 1, issue #8) -- `RegistryError::ConnectionFailed` must exist in `crates/tool-registry/src/registry_error.rs`.
- **Blocking:**
  - "Implement `connect()` with real MCP client logic" (Group 3, issue #9) -- the `tool_registry.rs` `connect` method calls `transport::connect_transport`.
  - "Write transport unit tests" (Group 5, issue #9) -- although this spec includes inline `#[cfg(test)]` unit tests for `parse_endpoint`, the Group 5 task may add additional tests or validate the existing ones.

## Risks & Edge Cases

1. **`#[cfg(unix)]` gating for Unix sockets.** `tokio::net::UnixStream` is only available on Unix platforms. The `Unix` variant of `TransportTarget` and all Unix-related code paths must be gated with `#[cfg(unix)]`. On non-Unix platforms, a `mcp+unix://` endpoint should return `RegistryError::ConnectionFailed` with a reason indicating Unix sockets are not supported on this platform. This ensures the crate compiles on all targets.

2. **IPv6 addresses.** IPv6 addresses in URLs use bracket notation: `mcp://[::1]:7001`. The naive `split(':')` approach for extracting host and port will fail for IPv6. The simplest mitigation: detect `[` in the host portion and handle bracket-enclosed addresses separately, splitting on `]:` to find the port. Alternatively, document that IPv6 bracket notation is supported. If full URL parsing is deemed too complex, document the limitation and defer IPv6 support -- plain hostnames and IPv4 addresses cover the initial use cases.

3. **Port 0.** Port 0 is technically valid (OS-assigned), but meaningless for a client connecting to a known server. The spec does not prohibit it -- `u16` parsing will accept it. If desired, a port > 0 check can be added, but this is low priority.

4. **Relative Unix socket paths.** `mcp+unix://relative/path.sock` (no leading `/`) is ambiguous. The parser extracts the path after `mcp+unix://`, which would yield `relative/path.sock`. This is a valid `PathBuf` but may fail at runtime if the working directory is unexpected. The spec does not prohibit relative paths -- document that absolute paths are recommended.

5. **rmcp API stability.** `rmcp` is pinned to 0.16. The `().serve(stream).await` call pattern and `RunningService<RoleClient, ()>` type may change in later versions. Pinning mitigates this, but verify the exact API when implementing. Check `rmcp 0.16` docs/source for the correct import paths (`rmcp::ServiceExt` for the `serve` method, `rmcp::service::RunningService`, `rmcp::model::RoleClient`).

6. **Error message consistency.** The `reason` field in `RegistryError::ConnectionFailed` should use the error's `Display` output (via `.to_string()`), not `Debug`. This ensures human-readable error messages that match the `RegistryError::Display` format.

7. **No URL crate dependency.** The spec intentionally avoids adding a `url` crate dependency for parsing. The scheme-based prefix stripping (`strip_prefix`) is sufficient for the two supported schemes and keeps dependencies minimal, following the project rule of not adding dependencies without strong reason.

## Verification

After implementation (and after all blocking tasks are complete), run:

```bash
cargo check -p tool-registry
cargo clippy -p tool-registry
cargo test -p tool-registry
```

All must pass with no errors and no warnings. Additionally verify:

- The file `crates/tool-registry/src/transport.rs` exists.
- `TransportTarget` has exactly two variants: `Tcp { host: String, port: u16 }` and `Unix { path: PathBuf }` (the latter gated with `#[cfg(unix)]`).
- `parse_endpoint("mcp://localhost:7001")` returns `Ok(TransportTarget::Tcp { host: "localhost".into(), port: 7001 })`.
- `parse_endpoint("mcp://192.168.1.10:8080")` returns `Ok(TransportTarget::Tcp { host: "192.168.1.10".into(), port: 8080 })`.
- `parse_endpoint("mcp+unix:///var/run/tool.sock")` returns `Ok(TransportTarget::Unix { path: PathBuf::from("/var/run/tool.sock") })`.
- `parse_endpoint` returns `Err(RegistryError::ConnectionFailed { .. })` for: missing scheme, unknown scheme, missing port, invalid port, empty Unix path.
- `connect_transport` compiles and has the correct signature: `async fn connect_transport(endpoint: &str) -> Result<RunningService<RoleClient, ()>, RegistryError>`.
- All functions are under 50 lines.
- No commented-out code, no debug statements.
- All `#[cfg(test)]` unit tests pass.
