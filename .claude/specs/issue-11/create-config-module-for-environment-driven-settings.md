# Spec: Create config module for environment-driven settings

> From: .claude/tasks/issue-11.md

## Objective

Create a `config` module for the `agent-runtime` crate that centralizes all runtime configuration into a single `RuntimeConfig` struct, read exclusively from environment variables. This replaces the hardcoded values currently scattered throughout `main.rs` (e.g., the hardcoded `"./skills"` path and `"echo"` skill name) with a clean, validated configuration layer. The module is a prerequisite for the `main.rs` refactor that wires together config, providers, and the `RuntimeAgent`.

## Current State

- `crates/agent-runtime/src/main.rs` hardcodes the skill directory (`PathBuf::from("./skills")`), skill name (`"echo"`), and has no bind address since the HTTP server does not exist yet.
- There is no configuration module; all values are inline literals.
- The crate has no `thiserror` dependency. Existing error types in the workspace (`RegistryError`, `SkillError`, `AgentError`) all follow the same pattern: a `#[derive(Debug, Clone, PartialEq)]` enum with a manual `fmt::Display` impl and a blanket `impl std::error::Error`. No crate uses `thiserror`.
- The `Cargo.toml` currently depends on `rig-core`, `rmcp`, `tool-registry`, `agent-sdk`, `skill-loader`, `tokio`, and `futures`. No additional dependencies are needed for this task.

## Requirements

1. Define a `RuntimeConfig` struct with exactly three fields:
   - `skill_name: String` -- read from the `SKILL_NAME` environment variable; required (no default).
   - `skill_dir: PathBuf` -- read from the `SKILL_DIR` environment variable; defaults to `"./skills"` if unset.
   - `bind_addr: SocketAddr` -- read from the `BIND_ADDR` environment variable; defaults to `0.0.0.0:8080` if unset.

2. Implement `RuntimeConfig::from_env() -> Result<Self, ConfigError>` as the sole constructor. It must:
   - Return `ConfigError::MissingVar { name: "SKILL_NAME".into() }` when `SKILL_NAME` is not set.
   - Return `ConfigError::InvalidValue { name, value, reason }` when `BIND_ADDR` is set but cannot be parsed as a `SocketAddr`.
   - Use `std::env::var()` directly -- no config file parsing, no third-party config crates.

3. Define a `ConfigError` enum with exactly two variants:
   - `MissingVar { name: String }` -- a required environment variable is not set.
   - `InvalidValue { name: String, value: String, reason: String }` -- an environment variable is set but its value cannot be parsed.

4. `ConfigError` must implement `Debug`, `Clone`, `PartialEq`, `fmt::Display`, and `std::error::Error`, following the same manual-impl pattern used by `RegistryError` and `SkillError` (no `thiserror`).

5. The module must not introduce any new dependencies to `Cargo.toml`.

6. The `RuntimeConfig` struct should derive `Debug` and `Clone` for diagnostics and flexibility in downstream usage.

## Implementation Details

### Files to create

- **`crates/agent-runtime/src/config.rs`** -- new module containing all types and logic described below.

### Files to modify

- **`crates/agent-runtime/src/main.rs`** -- add `mod config;` declaration (but do not change any other logic; the full refactor is a separate task).

### Types

```
ConfigError (enum)
  - MissingVar { name: String }
  - InvalidValue { name: String, value: String, reason: String }
  Derives: Debug, Clone, PartialEq
  Impls: fmt::Display, std::error::Error

RuntimeConfig (struct)
  - skill_name: String
  - skill_dir: PathBuf
  - bind_addr: SocketAddr
  Derives: Debug, Clone
```

### Key function: `RuntimeConfig::from_env()`

```
pub fn from_env() -> Result<Self, ConfigError>
```

Logic:
1. Read `SKILL_NAME` via `std::env::var("SKILL_NAME")`. If `Err(VarError::NotPresent)`, return `ConfigError::MissingVar`. If `Ok(val)`, use `val` as `skill_name`.
2. Read `SKILL_DIR` via `std::env::var("SKILL_DIR")`. If unset, default to `PathBuf::from("./skills")`. If set, use the value as-is (no validation needed; paths are opaque).
3. Read `BIND_ADDR` via `std::env::var("BIND_ADDR")`. If unset, default to `"0.0.0.0:8080".parse().unwrap()`. If set, parse via `value.parse::<SocketAddr>()`. On parse failure, return `ConfigError::InvalidValue` with the raw value and the parse error as the reason.
4. Return `Ok(RuntimeConfig { skill_name, skill_dir, bind_addr })`.

### Display messages

- `MissingVar`: `"missing required environment variable: '{name}'"`
- `InvalidValue`: `"invalid value for environment variable '{name}': '{value}' ({reason})"`

### Integration points

- `main.rs` will call `RuntimeConfig::from_env()?` early in startup to replace its hardcoded values.
- `config.skill_name` replaces the hardcoded `"echo"` passed to `loader.load()`.
- `config.skill_dir` replaces the hardcoded `PathBuf::from("./skills")` passed to `SkillLoader::new()`.
- `config.bind_addr` will be used by the HTTP server introduced in issue #12.

## Dependencies

- Blocked by: nothing (this is a Group 1 task with no prerequisites)
- Blocking: "Refactor main.rs to use config and RuntimeAgent" (Group 2)

## Risks & Edge Cases

- **Empty `SKILL_NAME`**: `std::env::var` returns `Ok("")` for a variable set to an empty string. The implementation should treat an empty string the same as a missing variable and return `MissingVar`, since an empty skill name is never valid.
- **`BIND_ADDR` with missing port**: A bare IP like `"0.0.0.0"` will fail `SocketAddr` parsing. The `InvalidValue` error's reason string (from the parse error) will explain the issue, which is sufficient.
- **Test isolation**: Tests that manipulate environment variables via `std::env::set_var`/`remove_var` are globally mutable and not thread-safe. Test files for this module (out of scope for this task, covered in Group 3) will need `#[serial]` or equivalent isolation.
- **No `thiserror`**: The project convention is manual `Display` impls. Do not introduce `thiserror` even though it would reduce boilerplate.

## Verification

1. `cargo check -p agent-runtime` passes with no errors after adding `mod config;` to `main.rs`.
2. `cargo clippy -p agent-runtime` reports no warnings for the new module.
3. `cargo test -p agent-runtime` continues to pass (no regressions; unit tests for config are a separate task).
4. The `config.rs` file contains exactly the types and function described above, with no extra dependencies added to `Cargo.toml`.
5. `ConfigError` display output matches the specified format strings.
