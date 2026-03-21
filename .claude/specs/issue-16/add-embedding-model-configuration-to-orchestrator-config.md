# Spec: Add embedding model configuration to `OrchestratorConfig`

> From: .claude/tasks/issue-16.md

## Objective

Extend `OrchestratorConfig` with optional embedding model settings so the orchestrator can be configured to use semantic routing. When these fields are absent, the orchestrator falls back to exact-match-only routing (no semantic fallback). This provides the configuration plumbing that `Orchestrator::from_config()` will use to decide whether to construct a `SemanticRouter`.

## Current State

`OrchestratorConfig` in `crates/orchestrator/src/config.rs` currently has a single field:

```rust
pub struct OrchestratorConfig {
    pub agents: Vec<AgentConfig>,
}
```

It supports two construction paths:
- `from_file(path)` -- deserializes from a YAML file via `serde_yaml`.
- `from_env()` -- reads `AGENT_ENDPOINTS` (required) and `AGENT_DESCRIPTIONS` (optional) environment variables, parsing comma-separated `key=value` pairs.

Both `OrchestratorConfig` and `AgentConfig` derive `Debug`, `Clone`, and `Deserialize`.

The existing test file (`crates/orchestrator/tests/config_test.rs`) covers YAML parsing, empty agents, malformed YAML, env-based parsing, and missing required env vars. Tests that modify env vars use a shared `ENV_LOCK` mutex and a `with_env_vars` helper for safe setup/teardown.

## Requirements

- Add three new optional fields to `OrchestratorConfig`:
  - `embedding_provider: Option<String>` -- embedding service provider identifier (e.g., `"openai"`).
  - `embedding_model: Option<String>` -- specific model name (e.g., `"text-embedding-3-small"`).
  - `similarity_threshold: Option<f64>` -- cosine similarity threshold for semantic routing; defaults to `0.7` when semantic routing is used but no threshold is explicitly configured.
- All three fields must be optional. Omitting them is not an error; it means semantic routing is disabled.
- `from_env()` must read these from environment variables: `EMBEDDING_PROVIDER`, `EMBEDDING_MODEL`, and `SIMILARITY_THRESHOLD`.
- `from_file()` must deserialize these from YAML (serde handles this automatically given correct field names and `Option` types).
- `SIMILARITY_THRESHOLD` must be parsed as `f64`. An invalid (non-numeric) value must return `OrchestratorError::Config` with a descriptive reason.
- Existing behavior must be fully preserved: all current tests must continue to pass without modification. YAML files and env configurations that omit the new fields must parse successfully with `None` values.
- The `similarity_threshold` field stores the raw `Option<f64>` from configuration. The default of `0.7` is applied downstream by the consumer (e.g., `Orchestrator::from_config()`), not inside `OrchestratorConfig` itself. This keeps the config struct a faithful representation of what the user provided.

## Implementation Details

### File to modify: `crates/orchestrator/src/config.rs`

**Struct changes:**

Add three fields to `OrchestratorConfig`. Use `#[serde(default)]` on each new field so that YAML files without these keys deserialize correctly (serde defaults `Option<T>` to `None`, but the explicit attribute makes the intent clear and guards against future refactors):

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct OrchestratorConfig {
    pub agents: Vec<AgentConfig>,
    #[serde(default)]
    pub embedding_provider: Option<String>,
    #[serde(default)]
    pub embedding_model: Option<String>,
    #[serde(default)]
    pub similarity_threshold: Option<f64>,
}
```

**`from_env()` changes:**

After the existing agent-building logic and before the final `Ok(OrchestratorConfig { agents })`, read the three new environment variables:

1. `EMBEDDING_PROVIDER` -- read via `std::env::var`. If the var is missing or empty, set to `None`. Otherwise `Some(value.trim().to_string())`.
2. `EMBEDDING_MODEL` -- same approach as `EMBEDDING_PROVIDER`.
3. `SIMILARITY_THRESHOLD` -- read via `std::env::var`. If missing or empty, set to `None`. If present, parse with `value.trim().parse::<f64>()`. On parse failure, return `OrchestratorError::Config { reason: "SIMILARITY_THRESHOLD must be a valid floating-point number, got '...'" }`.

Add a helper function to keep `from_env()` clean:

```rust
fn read_optional_env(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.trim().to_string())
}
```

And a parsing helper for the threshold:

```rust
fn parse_optional_f64_env(name: &str) -> Result<Option<f64>, OrchestratorError> {
    match read_optional_env(name) {
        None => Ok(None),
        Some(val) => val.parse::<f64>().map(Some).map_err(|_| OrchestratorError::Config {
            reason: format!("{name} must be a valid floating-point number, got '{val}'"),
        }),
    }
}
```

Update the return statement in `from_env()`:

```rust
Ok(OrchestratorConfig {
    agents,
    embedding_provider: read_optional_env("EMBEDDING_PROVIDER"),
    embedding_model: read_optional_env("EMBEDDING_MODEL"),
    similarity_threshold: parse_optional_f64_env("SIMILARITY_THRESHOLD")?,
})
```

**No changes needed to `from_file()`** -- serde_yaml handles the new `Option` fields automatically (absent keys become `None`).

### Integration points

- `Orchestrator::from_config()` (in `crates/orchestrator/src/orchestrator.rs`, modified by the "Integrate `SemanticRouter` into `Orchestrator`" task) will read these fields to decide whether to construct a `SemanticRouter`. If `embedding_provider` is `Some`, it will initialize the embedding model and build the router; otherwise, semantic routing is skipped.
- The `similarity_threshold` value (or its `0.7` default) will be passed to `SemanticRouter::new()`.

## Dependencies

- Blocked by: "Implement `SemanticRouter` struct with two-phase routing" (the config fields are designed to match the `SemanticRouter` constructor API)
- Blocking: "Write integration tests for semantic routing in `Orchestrator`", "Write unit tests for config embedding fields"

## Risks & Edge Cases

- **Whitespace-only env values**: `EMBEDDING_PROVIDER="  "` should be treated as unset (`None`), not as `Some("  ")`. The `read_optional_env` helper handles this by trimming and filtering empty strings.
- **Threshold out of range**: A `SIMILARITY_THRESHOLD` of `-1.0` or `5.0` is technically a valid `f64` and will parse successfully. Range validation is the responsibility of the consumer (`SemanticRouter` or `Orchestrator::from_config()`), not the config parser, since the config layer should be a faithful representation of user input.
- **YAML type coercion**: In YAML, `similarity_threshold: 0.7` deserializes as `f64` natively. However, `similarity_threshold: "0.7"` (quoted string) will fail serde deserialization. This is acceptable and consistent with YAML conventions.
- **Partial configuration**: Setting `EMBEDDING_MODEL` without `EMBEDDING_PROVIDER` is allowed at the config level (both are independently optional). Validation of logical consistency (e.g., "model requires provider") belongs in the consumer, not in config parsing.
- **Backward compatibility**: All existing YAML configs and env-based configs will continue to work because the new fields are `Option` with `#[serde(default)]`. No existing tests need modification.

## Verification

- `cargo check -p orchestrator` compiles without errors or warnings.
- `cargo clippy -p orchestrator` passes with no warnings.
- `cargo test -p orchestrator` passes -- all existing tests continue to pass unchanged.
- The following behaviors are verified by the "Write unit tests for config embedding fields" task:
  1. YAML with all three embedding fields parses correctly, values are `Some(...)`.
  2. YAML without any embedding fields parses correctly, values are all `None`.
  3. `from_env()` reads `EMBEDDING_PROVIDER` and `EMBEDDING_MODEL` as `Option<String>`.
  4. `from_env()` parses `SIMILARITY_THRESHOLD` as `Option<f64>`.
  5. `from_env()` with invalid `SIMILARITY_THRESHOLD` (e.g., `"notanumber"`) returns `OrchestratorError::Config`.
  6. `from_env()` with missing embedding env vars produces `None` values (not errors).
  7. `from_env()` with whitespace-only embedding env vars produces `None` values.
