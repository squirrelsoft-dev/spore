# Spec: Write unit tests for config embedding fields

> From: .claude/tasks/issue-16.md

## Objective
Add unit tests to `crates/orchestrator/tests/config_test.rs` that verify the new optional embedding configuration fields (`embedding_provider`, `embedding_model`, `similarity_threshold`) parse correctly from both YAML files and environment variables. These tests ensure the config layer correctly handles the presence, absence, and invalid values of embedding settings without breaking existing agent configuration parsing.

## Current State

### Existing config structure (`crates/orchestrator/src/config.rs`)
- `OrchestratorConfig` has a single field: `agents: Vec<AgentConfig>`.
- `AgentConfig` has fields: `name`, `description`, `url` (all `String`).
- `from_file(path)` reads and deserializes YAML via `serde_yaml`.
- `from_env()` reads `AGENT_ENDPOINTS` (required) and `AGENT_DESCRIPTIONS` (optional) env vars, parsing comma-separated `key=value` pairs.

### Existing test patterns (`crates/orchestrator/tests/config_test.rs`)
- Uses a static `ENV_LOCK: Mutex<()>` to serialize env-mutating tests.
- `with_env_vars` helper sets env vars, runs a closure, then restores originals. Uses `unsafe` env set/remove under the mutex guard.
- YAML tests write temp files to `std::env::temp_dir()`, parse with `from_file()`, assert fields, then clean up.
- Env tests use `with_env_vars` wrapping `OrchestratorConfig::from_env`.
- Error cases assert on `OrchestratorError::Config { .. }` variant via `matches!`.

### Blocking dependency
The "Add embedding model configuration to `OrchestratorConfig`" task will add three new optional fields to `OrchestratorConfig`:
- `embedding_provider: Option<String>` (e.g., `"openai"`)
- `embedding_model: Option<String>` (e.g., `"text-embedding-3-small"`)
- `similarity_threshold: Option<f64>` (defaults to 0.7 when not specified)

For env-based config, these will be read from `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, and `SIMILARITY_THRESHOLD` env vars. All three are optional -- absence means `None`, not an error.

## Requirements

1. **YAML with embedding settings parses correctly**: A YAML file containing `embedding_provider`, `embedding_model`, and `similarity_threshold` alongside the `agents` list must parse into an `OrchestratorConfig` with all fields populated. Assert each embedding field has the expected `Some(value)`.

2. **YAML without embedding settings still parses**: An existing-style YAML file with only `agents` (no embedding keys) must still parse successfully. Assert all three embedding fields are `None`. This confirms backward compatibility of the YAML format.

3. **Env-based config reads `EMBEDDING_PROVIDER` and `EMBEDDING_MODEL`**: When `EMBEDDING_PROVIDER` and `EMBEDDING_MODEL` env vars are set alongside `AGENT_ENDPOINTS`, `from_env()` must return a config where `embedding_provider` and `embedding_model` are `Some(...)` with the correct values.

4. **`SIMILARITY_THRESHOLD` env var parses as f64**: When `SIMILARITY_THRESHOLD` is set to a valid float string (e.g., `"0.85"`), `from_env()` must return `similarity_threshold` as `Some(0.85_f64)`. The parsed value must be exactly the expected float.

5. **Missing embedding env vars result in `None`**: When `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, and `SIMILARITY_THRESHOLD` are all absent (unset), `from_env()` must still succeed (given valid `AGENT_ENDPOINTS`) and all three embedding fields must be `None`.

6. **Invalid `SIMILARITY_THRESHOLD` returns an error**: When `SIMILARITY_THRESHOLD` is set to a non-numeric string (e.g., `"not_a_number"`), `from_env()` must return `Err(OrchestratorError::Config { .. })`.

7. **Partial embedding env vars are valid**: Setting only `EMBEDDING_PROVIDER` without `EMBEDDING_MODEL` (or vice versa) must not cause an error. Each field is independently optional.

## Implementation Details

### File to modify
- `crates/orchestrator/tests/config_test.rs`

### New test functions to add

**`yaml_config_with_embedding_settings_parses_correctly`**
- Write a YAML string containing `agents` plus `embedding_provider: openai`, `embedding_model: text-embedding-3-small`, `similarity_threshold: 0.85`.
- Write to a temp file, parse with `from_file()`.
- Assert `config.embedding_provider` is `Some("openai".to_string())`.
- Assert `config.embedding_model` is `Some("text-embedding-3-small".to_string())`.
- Assert `config.similarity_threshold` is `Some(0.85)`.
- Also assert `config.agents` still parses correctly (at least check length).
- Clean up temp dir.

**`yaml_config_without_embedding_settings_parses`**
- Use an existing-format YAML with only `agents: []` or a populated agents list.
- Parse with `from_file()`, assert it succeeds.
- Assert `config.embedding_provider.is_none()`.
- Assert `config.embedding_model.is_none()`.
- Assert `config.similarity_threshold.is_none()`.
- Note: The existing `empty_agents_list_is_valid` test partially covers this, but this test explicitly asserts the embedding fields are `None`.

**`env_config_reads_embedding_provider_and_model`**
- Use `with_env_vars` to set `AGENT_ENDPOINTS`, `AGENT_DESCRIPTIONS`, `EMBEDDING_PROVIDER` (e.g., `"openai"`), and `EMBEDDING_MODEL` (e.g., `"text-embedding-3-small"`).
- Clear `SIMILARITY_THRESHOLD` (set to `None`).
- Call `OrchestratorConfig::from_env()`.
- Assert `config.embedding_provider` is `Some("openai".to_string())`.
- Assert `config.embedding_model` is `Some("text-embedding-3-small".to_string())`.
- Assert `config.similarity_threshold.is_none()`.

**`env_config_parses_similarity_threshold_as_f64`**
- Use `with_env_vars` to set `AGENT_ENDPOINTS`, `SIMILARITY_THRESHOLD` to `"0.85"`.
- Clear `EMBEDDING_PROVIDER` and `EMBEDDING_MODEL` (set to `None`).
- Call `OrchestratorConfig::from_env()`.
- Assert `config.similarity_threshold` is `Some(0.85_f64)`.
- Use `assert!((val - 0.85).abs() < f64::EPSILON)` or `assert_eq!` depending on parsing exactness.

**`missing_embedding_env_vars_result_in_none`**
- Use `with_env_vars` to set only `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS`. Explicitly clear `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, and `SIMILARITY_THRESHOLD` (all `None`).
- Call `OrchestratorConfig::from_env()`.
- Assert all three embedding fields are `None`.
- Assert agents still parse correctly.

**`invalid_similarity_threshold_returns_error`**
- Use `with_env_vars` to set `AGENT_ENDPOINTS` and `SIMILARITY_THRESHOLD` to `"not_a_number"`.
- Call `OrchestratorConfig::from_env()`.
- Assert result is `Err`.
- Assert error matches `OrchestratorError::Config { .. }`.

**`partial_embedding_env_vars_are_valid`**
- Use `with_env_vars` to set `AGENT_ENDPOINTS` and `EMBEDDING_PROVIDER` (e.g., `"openai"`), but clear `EMBEDDING_MODEL` and `SIMILARITY_THRESHOLD`.
- Call `OrchestratorConfig::from_env()`.
- Assert `config.embedding_provider` is `Some("openai".to_string())`.
- Assert `config.embedding_model.is_none()`.
- Assert `config.similarity_threshold.is_none()`.

### Patterns to follow
- Each YAML test: create a unique temp dir (e.g., `orchestrator_test_<test_name>`), write YAML, parse, assert, clean up.
- Each env test: wrap in `with_env_vars`, explicitly list all relevant env vars (including clearing the ones not under test), call `from_env()`.
- Error assertions: use `matches!(err, OrchestratorError::Config { .. })` pattern.
- No new dependencies or helper functions needed beyond the existing `with_env_vars`.

## Dependencies
- Blocked by: "Add embedding model configuration to `OrchestratorConfig`" (the fields being tested do not exist yet)
- Blocking: Nothing (non-blocking)

## Risks & Edge Cases
- **Float precision**: Parsing `"0.85"` from a string should yield exactly `0.85_f64` since `f64::from_str("0.85")` is deterministic, but tests should be written to tolerate minor floating-point issues if needed.
- **Env var leakage between tests**: The existing `ENV_LOCK` mutex and `with_env_vars` restore mechanism prevent this. New tests must include all embedding env vars in their `with_env_vars` call (setting unused ones to `None`) to avoid inheriting state from a prior test that crashed before cleanup.
- **serde default behavior**: The `OrchestratorConfig` fields must use `#[serde(default)]` or be `Option<T>` for YAML backward compatibility. Tests in this spec validate that serde handles the missing-field case correctly.
- **`SIMILARITY_THRESHOLD` as empty string**: An empty string for a float env var could parse as `None` or error depending on implementation. The spec does not require a test for this edge case, but implementers should be aware.

## Verification
- `cargo test --package orchestrator --test config_test` passes with all new and existing tests green.
- `cargo clippy --package orchestrator` reports no new warnings in the test file.
- All seven new test functions appear in the test output and pass.
- Existing tests (`yaml_config_parses_correctly`, `empty_agents_list_is_valid`, `malformed_yaml_returns_error`, `env_config_parses_agent_endpoints`, `missing_env_var_returns_error`) continue to pass unchanged.
