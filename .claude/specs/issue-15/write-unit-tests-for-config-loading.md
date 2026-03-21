# Spec: Write unit tests for config loading

> From: .claude/tasks/issue-15.md

## Objective
Create a comprehensive unit test suite for the orchestrator config module (`crates/orchestrator/src/config.rs`), validating both the YAML file-based and environment variable-based configuration loading paths. This ensures the config layer is reliable before other components (like `Orchestrator::from_config`) depend on it.

## Current State

### Config module (not yet implemented — blocked by "Define registry config format and loader")

Per the task breakdown, `crates/orchestrator/src/config.rs` will define:

```rust
#[derive(Deserialize)]
pub struct OrchestratorConfig {
    pub agents: Vec<AgentConfig>,
}

#[derive(Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub description: String,
    pub url: String,
}
```

With two loading methods:
- `OrchestratorConfig::from_file(path: &str) -> Result<Self, OrchestratorError>` — reads and parses a YAML file.
- `OrchestratorConfig::from_env() -> Result<Self, OrchestratorError>` — reads `AGENT_ENDPOINTS` env var (format: `name=url,name2=url2`) and optional `AGENT_DESCRIPTIONS` env var.

### Env-testing pattern (from `crates/agent-runtime/src/config.rs`)

The project uses a `Mutex`-based pattern to serialize tests that modify environment variables:

- A static `ENV_LOCK: Mutex<()>` ensures mutual exclusion.
- A helper `with_env_vars(vars: &[(&str, Option<&str>)], f: F)` saves original values, sets/removes vars, runs the closure, then restores originals.
- Each call to `env::set_var` / `env::remove_var` is wrapped in `unsafe {}` with a comment explaining serialization via the lock.

### Test conventions in the project

- Integration test files live under `crates/<crate>/tests/<name>_test.rs`.
- Tests import from the crate's public API (e.g., `use orchestrator::config::{OrchestratorConfig, AgentConfig};`).
- Each `#[test]` function has a descriptive snake_case name indicating the scenario.
- Assertions use `assert_eq!` for value comparisons and pattern-matching for error variant checks.
- No external test framework (no `rstest`, `proptest`, etc.) — only `#[test]` and standard library assertions.

## Requirements

1. **YAML config parses correctly with multiple agents** — Construct a valid YAML string with 2+ agents, write it to a temp file (using `std::fs::write` + `tempfile` or a hardcoded path in a temp dir), call `OrchestratorConfig::from_file()`, and assert the returned `OrchestratorConfig` contains the expected `AgentConfig` entries with correct `name`, `description`, and `url` fields.

2. **Empty agents list is valid** — Parse a YAML string with `agents: []`, verify it succeeds and returns an `OrchestratorConfig` with an empty `agents` vec. This confirms the config layer does not impose a minimum agent count.

3. **Malformed YAML returns appropriate error** — Pass invalid YAML content (e.g., `"agents: [[[broken"`) to the config parser and assert the result is an error. Verify the error is the expected variant (likely mapping to an `OrchestratorError` variant that wraps the parse failure reason).

4. **Env-based config parses `AGENT_ENDPOINTS` format correctly** — Use the `with_env_vars` helper to set `AGENT_ENDPOINTS=agent1=http://localhost:8001,agent2=http://localhost:8002` and optionally `AGENT_DESCRIPTIONS=agent1=First agent,agent2=Second agent`. Call `OrchestratorConfig::from_env()` and assert the result contains the correct agent configs.

5. **Missing `AGENT_ENDPOINTS` env var returns error** — Use `with_env_vars` to ensure `AGENT_ENDPOINTS` is unset, call `from_env()`, and assert it returns an error (the env-based path requires at least the endpoints variable).

6. **Env mutex serialization** — All env-modifying tests must use the `with_env_vars` helper and `ENV_LOCK` mutex to prevent test interference, following the exact pattern from `crates/agent-runtime/src/config.rs`.

## Implementation Details

### File to create
- **`crates/orchestrator/tests/config_test.rs`** — The sole deliverable. Contains all config-loading tests.

### Structure of the test file

```
// Imports: orchestrator config types, std::env, std::sync::Mutex, std::io::Write, tempfile or std::fs

// ENV_LOCK: static Mutex<()> for serializing env-modifying tests

// with_env_vars helper (copied/adapted from agent-runtime pattern)

// --- YAML tests (no env interaction, no mutex needed) ---
// fn yaml_config_parses_multiple_agents()
// fn yaml_config_empty_agents_list_is_valid()
// fn yaml_config_malformed_returns_error()

// --- Env tests (all use with_env_vars + ENV_LOCK) ---
// fn env_config_parses_agent_endpoints()
// fn env_config_missing_agent_endpoints_returns_error()
```

### Key types/interfaces to use
- `orchestrator::config::OrchestratorConfig` — the main config struct
- `orchestrator::config::AgentConfig` — individual agent entry
- `orchestrator::error::OrchestratorError` — error type returned by config loading methods
- `OrchestratorConfig::from_file(path)` — YAML file loading
- `OrchestratorConfig::from_env()` — env var loading

### Temp file handling for YAML tests
- Use `std::env::temp_dir()` to get a temp directory, write YAML content to a unique file (e.g., using a UUID or test-specific name in the path), call `from_file()` with that path, then clean up with `std::fs::remove_file()`. Alternatively, if `tempfile` is available as a dev-dependency, use `NamedTempFile`.
- If neither is suitable, the config module may also expose a `from_yaml_str()` or similar method that can be tested without file I/O. The spec should prefer testing through the public API (`from_file`), but the implementation task may choose to expose a string-parsing helper for testability.

### YAML content for tests

Multiple agents:
```yaml
agents:
  - name: summarizer
    description: Summarizes text
    url: http://localhost:8001
  - name: translator
    description: Translates text
    url: http://localhost:8002
```

Empty list:
```yaml
agents: []
```

Malformed:
```yaml
agents: [[[not valid yaml
```

### Env var format

The `AGENT_ENDPOINTS` format follows the `TOOL_ENDPOINTS` pattern from `crates/agent-runtime/src/main.rs` lines 86-112:
- Comma-separated pairs: `name=url,name2=url2`
- Each pair split on `=` into name and URL
- Whitespace around entries is trimmed
- Empty entries between commas are skipped

The `AGENT_DESCRIPTIONS` env var (optional) follows the same format: `name=description,name2=description2`.

## Dependencies
- **Blocked by:** "Define registry config format and loader" — the config module (`crates/orchestrator/src/config.rs`) must exist with `OrchestratorConfig`, `AgentConfig`, `from_file()`, and `from_env()` before tests can be written against them.
- **Blocking:** Nothing — this is a leaf task.

## Risks & Edge Cases

- **Config API not yet finalized:** The exact method signatures and error types for `from_file` and `from_env` are specified in the task breakdown but may evolve during implementation. The test file must be written against the actual public API of the config module once it exists. If `from_file` takes a `&Path` instead of `&str`, or returns a different error type, tests must adapt.
- **`tempfile` crate availability:** The project avoids unnecessary dependencies. YAML tests that need temp files should use `std::fs` + `std::env::temp_dir()` rather than adding a `tempfile` dev-dependency. Use a unique subdirectory or filename to avoid collisions with parallel test runs.
- **Env var pollution between tests:** The `with_env_vars` helper must restore original values even if the test closure panics. The current pattern in `agent-runtime` does NOT use `catch_unwind` (it relies on the mutex preventing concurrent access). This is acceptable but means a panicking test could leave env vars dirty. This is a known minor risk, consistent with the existing codebase approach.
- **`AGENT_DESCRIPTIONS` optionality:** The task breakdown says `AGENT_DESCRIPTIONS` is optional. Tests should verify that `from_env` works both with and without this variable set. When omitted, agents should get an empty or default description.
- **Malformed `AGENT_ENDPOINTS` entries:** Consider edge cases like `name_only_no_equals`, `=url_without_name`, or an empty string. These may or may not be in scope for this test file depending on the config module's validation logic, but the spec should note them as potential additional tests.
- **`unsafe` env operations:** The `env::set_var` and `env::remove_var` functions are `unsafe` in Rust 2024 edition (which this crate uses per `edition = "2024"` in Cargo.toml). Tests must wrap these calls in `unsafe {}` blocks with appropriate safety comments, matching the existing pattern.

## Verification
- `cargo test -p orchestrator --test config_test` passes with all tests green.
- `cargo test` across the full workspace shows no regressions.
- `cargo clippy -p orchestrator` reports no warnings in the test file.
- Each of the 5 specified test scenarios has a corresponding `#[test]` function that exercises the stated behavior.
- Env-modifying tests use the mutex-based `with_env_vars` pattern and do not interfere with each other or with other test files.
