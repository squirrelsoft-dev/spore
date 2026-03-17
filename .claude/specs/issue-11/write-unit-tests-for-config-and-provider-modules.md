# Spec: Write unit tests for config and provider modules

> From: .claude/tasks/issue-11.md

## Objective

Add integration-style unit tests for the `config` and `provider` modules of the `agent-runtime` crate. These tests verify that environment-variable-driven configuration parsing works correctly (happy path, missing required vars, default values) and that the provider module rejects unsupported providers and missing API keys with clear errors. Because these tests manipulate process-wide environment variables, they must run serially to avoid interference.

## Current State

- **Config module does not exist yet.** Per the task breakdown, `crates/agent-runtime/src/config.rs` will define:
  - `RuntimeConfig` struct with fields: `skill_name: String` (from `SKILL_NAME`, required), `skill_dir: PathBuf` (from `SKILL_DIR`, default `./skills`), `bind_addr: SocketAddr` (from `BIND_ADDR`, default `0.0.0.0:8080`).
  - `RuntimeConfig::from_env()` constructor returning `Result<Self, ConfigError>`.
  - `ConfigError` enum with variants `MissingVar { name: String }` and `InvalidValue { name: String, value: String, reason: String }`.

- **Provider module does not exist yet.** Per the task breakdown, `crates/agent-runtime/src/provider.rs` will define:
  - A `build_agent()` function that takes `&SkillManifest` and `Vec<McpTool>`, reads `ModelConfig.provider` to select the rig-core provider (initially only `"openai"` supported), reads the corresponding API key from environment (e.g., `OPENAI_API_KEY`), and returns a built rig-core `Agent` or an error.
  - An error type (or reuse of `ConfigError` / a new `ProviderError`) for unsupported providers and missing API keys.

- **Workspace test conventions (observed from existing crates):**
  - Integration tests live in `crates/<crate>/tests/<name>_test.rs` files.
  - Tests use `#[test]` for sync tests and `#[tokio::test]` for async tests.
  - Test functions construct types directly and assert on return values using `assert_eq!`, `assert!`, and `.contains()` for error message substring checks.
  - No mocking frameworks are used. Helper functions like `make_manifest()` and `valid_manifest()` construct test fixtures inline.
  - Tests import crate types directly (e.g., `use agent_runtime::config::{RuntimeConfig, ConfigError};`).

- **Current `Cargo.toml` dev-dependencies:** The `agent-runtime` crate currently has no `[dev-dependencies]` section. `tokio` is already a regular dependency with `features = ["full"]`.

## Requirements

1. **Add `serial_test` as a dev-dependency** in `crates/agent-runtime/Cargo.toml` under `[dev-dependencies]` to provide the `#[serial]` attribute macro for env-var-manipulating tests. This prevents parallel test execution from causing flaky failures due to shared process-wide environment state.

2. **Create `crates/agent-runtime/tests/config_test.rs`** with the following test cases, each annotated with `#[serial]`:

   | # | Test name | Setup | Assertion |
   |---|-----------|-------|-----------|
   | 1 | `missing_skill_name_returns_error` | Remove `SKILL_NAME` from env (ensure it is unset). | `RuntimeConfig::from_env()` returns `Err(ConfigError::MissingVar { name })` where `name` contains `"SKILL_NAME"`. |
   | 2 | `valid_env_produces_correct_config` | Set `SKILL_NAME=echo`, `SKILL_DIR=/tmp/skills`, `BIND_ADDR=127.0.0.1:9090`. | `RuntimeConfig::from_env()` returns `Ok(config)` with `config.skill_name == "echo"`, `config.skill_dir == PathBuf::from("/tmp/skills")`, `config.bind_addr` parsed as `127.0.0.1:9090`. |
   | 3 | `default_skill_dir_when_unset` | Set `SKILL_NAME=test`, remove `SKILL_DIR`. | `config.skill_dir == PathBuf::from("./skills")`. |
   | 4 | `default_bind_addr_when_unset` | Set `SKILL_NAME=test`, remove `BIND_ADDR`. | `config.bind_addr` equals `"0.0.0.0:8080".parse::<SocketAddr>()`. |
   | 5 | `invalid_bind_addr_returns_error` | Set `SKILL_NAME=test`, `BIND_ADDR=not-an-address`. | `RuntimeConfig::from_env()` returns `Err(ConfigError::InvalidValue { .. })` with `name` containing `"BIND_ADDR"`. |
   | 6 | `all_defaults_with_only_required_vars` | Set only `SKILL_NAME=minimal`, remove `SKILL_DIR` and `BIND_ADDR`. | `Ok(config)` with `skill_name == "minimal"`, `skill_dir == "./skills"`, `bind_addr == "0.0.0.0:8080"`. |

3. **Create `crates/agent-runtime/tests/provider_test.rs`** with the following test cases, each annotated with `#[serial]`:

   | # | Test name | Setup | Assertion |
   |---|-----------|-------|-----------|
   | 1 | `unsupported_provider_returns_error` | Construct a `SkillManifest` with `model.provider = "unsupported-llm"`. Call `build_agent()` with an empty tools vec. | Returns an error whose display/message contains `"unsupported"` (case-insensitive) or the provider name `"unsupported-llm"`. |
   | 2 | `missing_api_key_returns_error` | Construct a `SkillManifest` with `model.provider = "openai"`. Remove `OPENAI_API_KEY` from env. Call `build_agent()` with an empty tools vec. | Returns an error whose display/message mentions the missing key (e.g., contains `"OPENAI_API_KEY"` or `"api key"`). |
   | 3 | `openai_provider_recognized` | Construct a `SkillManifest` with `model.provider = "openai"`. Set `OPENAI_API_KEY=test-key-placeholder`. Call `build_agent()` with an empty tools vec. | Returns `Ok(agent)` -- the agent is constructed successfully (we do not invoke it, just verify construction does not error). |

4. **Env var cleanup:** Each test must restore the environment to its pre-test state. The `#[serial]` attribute ensures tests do not overlap, but each test should still clean up after itself (remove vars it set, restore vars it removed) using a helper or explicit `std::env::remove_var` / `std::env::set_var` in a cleanup block or via the `serial_test` crate's facilities. Alternatively, use `unsafe { std::env::set_var(...) }` / `unsafe { std::env::remove_var(...) }` as required by Rust 2024 edition (edition = "2024" in the Cargo.toml), wrapping in an `unsafe` block since environment mutation is considered unsafe in the 2024 edition.

5. **Manifest helper:** Both test files (especially `provider_test.rs`) need a `SkillManifest` fixture. Define a `make_manifest()` helper function within each test file (following the pattern from `crates/agent-sdk/tests/micro_agent_test.rs` and `crates/skill-loader/tests/validation_test.rs`). The helper should return a valid `SkillManifest` with sensible defaults (e.g., `provider: "openai"`, `name: "gpt-4o"`, `temperature: 0.7`, `tools: vec![]`, etc.).

6. **No network calls:** The provider tests that construct an OpenAI agent with a placeholder key must not make any network calls. They only verify that the `build_agent()` function successfully constructs the agent struct. No `.invoke()` or `.prompt()` calls.

## Implementation Details

### Files to modify

- **`crates/agent-runtime/Cargo.toml`**: Add `[dev-dependencies]` section with `serial_test = "3"`. Also add `agent-sdk = { path = "../agent-sdk" }` under dev-dependencies if not already a regular dependency (it is already a regular dependency, so the tests can import it directly).

### Files to create

- **`crates/agent-runtime/tests/config_test.rs`**:
  - Imports: `use agent_runtime::config::{RuntimeConfig, ConfigError};` (or `use agent_runtime::{RuntimeConfig, ConfigError};` depending on re-exports), `use std::net::SocketAddr;`, `use std::path::PathBuf;`, `use serial_test::serial;`.
  - A helper function `clear_config_env()` that removes `SKILL_NAME`, `SKILL_DIR`, and `BIND_ADDR` from the environment to establish a clean baseline before each test.
  - 6 test functions as described in Requirements section 2, each with `#[test]` and `#[serial]`.
  - Note: `std::env::set_var` and `std::env::remove_var` are `unsafe` in Rust 2024 edition. All calls must be wrapped in `unsafe { }` blocks.

- **`crates/agent-runtime/tests/provider_test.rs`**:
  - Imports: `use agent_runtime::provider::build_agent;` (or however the function is re-exported), `use agent_sdk::{...};` for `SkillManifest` and related types, `use serial_test::serial;`.
  - A `make_manifest()` helper returning a valid `SkillManifest` (following existing patterns).
  - 3 test functions as described in Requirements section 3. If `build_agent()` is async, use `#[tokio::test]` instead of `#[test]`, combined with `#[serial]`.
  - Note: Same `unsafe` requirement for env var manipulation applies.

### Key types/interfaces the tests depend on

- `RuntimeConfig::from_env() -> Result<RuntimeConfig, ConfigError>` (from `config.rs`)
- `ConfigError::MissingVar { name: String }` and `ConfigError::InvalidValue { name: String, value: String, reason: String }` (from `config.rs`)
- `RuntimeConfig` fields: `skill_name: String`, `skill_dir: PathBuf`, `bind_addr: SocketAddr`
- `build_agent(&SkillManifest, Vec<McpTool>) -> Result<..., ...>` (from `provider.rs`) -- the exact return type and error type depend on the provider module implementation
- `SkillManifest`, `ModelConfig`, `Constraints`, `OutputSchema` (from `agent-sdk`)

### Integration points

- Tests import from `agent_runtime` as an external crate (integration test style), so the config and provider modules must be `pub` in `agent_runtime`'s `lib.rs` (or re-exported from it). The `agent-runtime` crate currently only has `main.rs` and `tool_bridge.rs` with no `lib.rs`. A `lib.rs` will need to be created by the preceding tasks to expose these modules for testing.

## Dependencies

- **Blocked by:**
  - "Create config module for environment-driven settings" (Group 1) -- `RuntimeConfig`, `ConfigError`, and `from_env()` must exist.
  - "Create provider module for LLM client construction" (Group 1) -- `build_agent()` and its error types must exist.
  - "Refactor main.rs to use config and RuntimeAgent" (Group 2) -- the modules must be wired into `lib.rs` with `pub` visibility so integration tests can import them.
- **Blocking:**
  - "Run verification suite" (Group 3) -- the verification task runs `cargo test -p agent-runtime` and depends on these tests existing and passing.

## Risks & Edge Cases

1. **Rust 2024 edition `unsafe` requirement for env vars:** The `agent-runtime` crate uses `edition = "2024"`, which makes `std::env::set_var` and `std::env::remove_var` unsafe functions (since they can cause undefined behavior in multithreaded programs). All env var mutations in tests must be wrapped in `unsafe { }` blocks. The `#[serial]` attribute mitigates the actual safety concern by preventing concurrent execution, but the `unsafe` annotation is still syntactically required.

2. **Module visibility / re-export structure:** The tests assume `agent_runtime::config::RuntimeConfig` and `agent_runtime::provider::build_agent` are publicly accessible. If the implementation uses a different re-export structure (e.g., flat re-exports from `lib.rs`), the import paths in the tests must be adjusted. The implementer should check the actual `lib.rs` exports.

3. **Provider error type uncertainty:** The task breakdown does not specify whether `build_agent()` returns a `ConfigError`, a dedicated `ProviderError`, or a generic `Box<dyn Error>`. The tests should assert on the error's `Display` output (using `.to_string().contains(...)`) rather than matching on specific enum variants, unless the error type is clearly defined. If the error type does have specific variants (e.g., `ProviderError::UnsupportedProvider { provider }` and `ProviderError::MissingApiKey { key_name }`), the tests can match on those for stronger assertions.

4. **`build_agent()` may be async:** If `build_agent()` performs any async initialization (e.g., validating the API key), the provider tests will need `#[tokio::test]` instead of `#[test]`. The implementer should check the actual function signature. Given that rig-core client construction is typically synchronous but the function signature may be async for flexibility, both attributes should be considered.

5. **OpenAI client construction with invalid key:** The test `openai_provider_recognized` sets `OPENAI_API_KEY=test-key-placeholder` and expects `build_agent()` to succeed (return `Ok`). This works only if rig-core's `openai::Client::new(key)` does not validate the key at construction time (it should not -- validation happens at request time). If rig-core does eagerly validate, this test would need to be marked `#[ignore]` or restructured.

6. **`serial_test` version compatibility:** The spec specifies `serial_test = "3"`. If version 3 is not available or has breaking changes, fall back to `serial_test = "2"`. The API (`#[serial]` attribute) is the same across major versions.

7. **Env var leakage between test files:** The `#[serial]` attribute from `serial_test` serializes tests within the same test binary. Since `config_test.rs` and `provider_test.rs` are separate integration test files, they compile into separate binaries and cannot interfere with each other. However, if they are combined into a single binary (unlikely for integration tests), `#[serial]` handles it correctly.

## Verification

1. `cargo test -p agent-runtime --test config_test` compiles and all 6 config tests pass.
2. `cargo test -p agent-runtime --test provider_test` compiles and all 3 provider tests pass.
3. `cargo clippy -p agent-runtime --tests` reports no warnings on the test files.
4. Tests do not make network calls or require external services.
5. Each test cleans up its environment variables, leaving no side effects for subsequent tests.
6. `cargo test -p agent-runtime` runs all tests (including these and any others) without failures.
