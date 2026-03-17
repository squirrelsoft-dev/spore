# Spec: Write transport unit tests

> From: .claude/tasks/issue-9.md

## Objective

Add inline unit tests for the `parse_endpoint` function in `crates/tool-registry/src/transport.rs`. These tests verify that the endpoint URL parser correctly distinguishes TCP (`mcp://`) and Unix socket (`mcp+unix://`) transports, extracts host/port/path values, and rejects malformed inputs with appropriate errors. Catching parsing bugs here prevents connection failures from surfacing only at runtime when the agent-runtime tries to contact tool servers.

## Current State

- `crates/tool-registry/src/transport.rs` does not yet exist. It will be created by the "Create transport module with endpoint URL parsing" task, which defines:
  - `pub(crate) enum TransportTarget { Tcp { host: String, port: u16 }, Unix { path: PathBuf } }` -- the parsed result type.
  - `pub(crate) fn parse_endpoint(endpoint: &str) -> Result<TransportTarget, RegistryError>` -- the function under test.
  - `pub(crate) async fn connect_transport(endpoint: &str) -> Result<RunningService<RoleClient, ()>, RegistryError>` -- uses `parse_endpoint` internally but is NOT the focus of these unit tests (it requires a running server).
- `RegistryError` will be defined in `crates/tool-registry/src/registry_error.rs` (per issue-8 specs) with a `ConnectionFailed { endpoint: String, reason: String }` variant used for parse failures.
- The crate uses `#[cfg(test)] mod tests` for inline unit tests in some modules (e.g., `crates/skill-loader/src/frontmatter.rs`, `crates/skill-loader/src/validation.rs`) and external test files in `tests/` for integration-style tests. Since `parse_endpoint` is `pub(crate)`, it is NOT accessible from external test files -- inline tests are required.
- Existing test patterns:
  - `crates/skill-loader/src/frontmatter.rs` uses `#[cfg(test)] mod tests { use super::*; ... }` with synchronous `#[test]` functions.
  - `crates/skill-loader/src/validation.rs` follows the same pattern.
  - Tests use `assert_eq!`, `assert!(matches!(...))`, `assert!(result.is_err())`, and destructuring with `if let` for error inspection.
  - Test function names are descriptive snake_case (e.g., `parses_standard_frontmatter`, `returns_error_for_missing_opening_delimiter`).

## Requirements

### Valid endpoint tests

1. **Valid TCP with hostname and port**: `parse_endpoint("mcp://localhost:7001")` returns `Ok(TransportTarget::Tcp { host: "localhost", port: 7001 })`.
2. **Valid TCP with IP address and port**: `parse_endpoint("mcp://127.0.0.1:8080")` returns `Ok(TransportTarget::Tcp { host: "127.0.0.1", port: 8080 })`.
3. **Valid Unix socket with absolute path**: `parse_endpoint("mcp+unix:///var/run/tool.sock")` returns `Ok(TransportTarget::Unix { path: PathBuf::from("/var/run/tool.sock") })`.

### Invalid endpoint tests

4. **Missing scheme**: `parse_endpoint("localhost:7001")` returns `Err(RegistryError::ConnectionFailed { .. })`. The error reason should indicate an unrecognized or missing scheme.
5. **Unknown scheme**: `parse_endpoint("http://localhost:7001")` returns `Err(RegistryError::ConnectionFailed { .. })`. The error reason should indicate an unrecognized scheme.
6. **Missing port on TCP**: `parse_endpoint("mcp://localhost")` returns `Err(RegistryError::ConnectionFailed { .. })`. The error reason should indicate a missing or invalid port.
7. **Invalid port on TCP**: `parse_endpoint("mcp://localhost:notaport")` returns `Err(RegistryError::ConnectionFailed { .. })`. The error reason should indicate an invalid port value.
8. **Unix with no path**: `parse_endpoint("mcp+unix://")` returns `Err(RegistryError::ConnectionFailed { .. })`. The error reason should indicate a missing socket path.

### General requirements

9. All tests are synchronous `#[test]` functions (no `#[tokio::test]` needed since `parse_endpoint` is not async).
10. Tests use `use super::*;` to access `parse_endpoint`, `TransportTarget`, and any needed imports.
11. Error assertions should verify the correct `RegistryError` variant (`ConnectionFailed`) is returned, and should check that the error reason contains a relevant substring (not an exact string match, to tolerate minor wording changes).
12. Success assertions should destructure the `TransportTarget` enum to verify individual field values rather than relying on `PartialEq` (since `TransportTarget` may not derive `PartialEq`). Alternatively, if `TransportTarget` does derive `PartialEq`, direct equality assertions are acceptable.

## Implementation Details

### File to modify

**`crates/tool-registry/src/transport.rs`** -- append a `#[cfg(test)] mod tests` block at the end of the file.

### Test module structure

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // --- Valid endpoints ---

    #[test]
    fn parses_valid_tcp_endpoint() { ... }

    #[test]
    fn parses_tcp_endpoint_with_ip_address() { ... }

    #[test]
    fn parses_valid_unix_endpoint() { ... }

    // --- Invalid endpoints ---

    #[test]
    fn rejects_missing_scheme() { ... }

    #[test]
    fn rejects_unknown_scheme() { ... }

    #[test]
    fn rejects_tcp_missing_port() { ... }

    #[test]
    fn rejects_tcp_invalid_port() { ... }

    #[test]
    fn rejects_unix_with_no_path() { ... }
}
```

### Test naming convention

Follow the codebase pattern from `frontmatter.rs` and `validation.rs`: descriptive verb-first names (`parses_...`, `rejects_...`, `handles_...`).

### Assertion patterns

For success cases, use destructuring to check fields:
```rust
let result = parse_endpoint("mcp://localhost:7001");
let target = result.expect("should parse valid TCP endpoint");
match target {
    TransportTarget::Tcp { host, port } => {
        assert_eq!(host, "localhost");
        assert_eq!(port, 7001);
    }
    other => panic!("expected Tcp, got {other:?}"),
}
```

If `TransportTarget` derives `PartialEq`, a simpler form is acceptable:
```rust
assert_eq!(
    parse_endpoint("mcp://localhost:7001").unwrap(),
    TransportTarget::Tcp { host: "localhost".to_string(), port: 7001 }
);
```

For error cases, use `assert!(matches!(...))` combined with substring checks:
```rust
let err = parse_endpoint("localhost:7001").unwrap_err();
assert!(
    matches!(err, RegistryError::ConnectionFailed { .. }),
    "expected ConnectionFailed, got {err:?}"
);
let display = err.to_string();
assert!(
    display.contains("scheme") || display.contains("unrecognized"),
    "expected mention of scheme in: {display}"
);
```

### No Cargo.toml changes needed

The tests are synchronous and inline. No additional dev-dependencies are required beyond what is already specified in the `add-rmcp-tokio-dependencies` spec.

## Dependencies

- **Blocked by**: "Create transport module with endpoint URL parsing" -- the `parse_endpoint` function, `TransportTarget` enum, and the `transport.rs` file itself must exist before tests can be added.
- **Blocking**: "Run verification suite" -- the verification task will run `cargo test` across the workspace and needs these tests to pass.

## Risks & Edge Cases

1. **`TransportTarget` field types or names differ from spec**: If the transport module task uses different field names (e.g., `addr` instead of `host`) or different types (e.g., `SocketAddr` instead of separate `host`/`port`), the test assertions will need to match. The implementer should read the actual `TransportTarget` definition before writing assertions.
2. **`parse_endpoint` error messages differ**: The tests use substring-based error assertions (not exact matches) to tolerate minor wording differences. However, if the error messages are fundamentally different (e.g., using `RegistryError::InvalidEndpoint` instead of `ConnectionFailed`), the variant assertions will need updating.
3. **`TransportTarget` does not derive `Debug`**: The `panic!("expected Tcp, got {other:?}")` pattern requires `Debug`. If `TransportTarget` does not derive `Debug`, the implementer should either add it or adjust the panic message to not use `{:?}` formatting.
4. **Platform-specific Unix socket paths**: The Unix socket test uses `/var/run/tool.sock`, which is a valid path on Linux/macOS. This is fine since the test only exercises string parsing, not actual filesystem access. The path does not need to exist.
5. **Edge case: mcp+unix with relative path**: The task description does not require testing relative Unix socket paths (e.g., `mcp+unix://tool.sock`). This is out of scope but could be added later if the transport module supports it.

## Verification

1. `cargo test -p tool-registry` passes with all 8 new tests green.
2. `cargo test -p tool-registry -- --list` shows all expected test function names under `transport::tests::`.
3. `cargo clippy -p tool-registry -- -D warnings` produces no warnings in the test code.
4. Each valid-endpoint test confirms correct variant and field values.
5. Each invalid-endpoint test confirms `ConnectionFailed` variant and a meaningful error reason substring.
