# Spec: Define registry config format and loader

> From: .claude/tasks/issue-15.md

## Objective

Create a `config` module for the `orchestrator` crate that defines a YAML-deserializable configuration structure for agent registry entries, with two loading paths: a primary environment-variable-based loader (`from_env`) and a secondary YAML-file-based loader (`from_file`). This module provides the data needed by the `Orchestrator` struct (Group 3) to construct its `AgentEndpoint` registry at startup. The env-based approach follows the `TOOL_ENDPOINTS` comma-separated `name=url` pattern already established in `crates/agent-runtime/src/main.rs` (lines 86-112), while the YAML path offers a richer config format for local development.

## Current State

- **`crates/orchestrator/`** is currently a stub binary crate with a 3-line `main.rs` printing "Hello, world!". It has no dependencies beyond the defaults. A prerequisite task will convert it to a library crate with `lib.rs` declaring `pub mod config;` and another task will add the required dependencies to `Cargo.toml` (including `serde`, `serde_yaml`).

- **`crates/agent-runtime/src/config.rs`** defines the project's established config pattern:
  - A `RuntimeConfig` struct with `#[derive(Debug, Clone)]`.
  - A `ConfigError` enum with two variants (`MissingVar`, `InvalidValue`), each with `#[derive(Debug, Clone, PartialEq)]`, manual `fmt::Display` impl, and `impl std::error::Error`.
  - A `from_env()` constructor that uses helper functions `read_required_var(name)` and `read_optional_var_or(name, default)` to read `std::env::var()` values.
  - Empty strings are treated as missing (the `Ok(val) if !val.is_empty()` guard pattern).
  - Tests use a `with_env_vars` helper and a `static ENV_LOCK: Mutex<()>` to safely serialize env-mutating tests.

- **`crates/agent-runtime/src/main.rs` lines 86-112** define the `TOOL_ENDPOINTS` parsing pattern:
  - Read `TOOL_ENDPOINTS` env var (with a default fallback).
  - Split by comma, trim each pair, skip empty entries.
  - Split each pair on `=` via `split_once('=')`, error if no `=` found.
  - Trim name and endpoint individually.
  - This pattern will be reused for `AGENT_ENDPOINTS`.

- **Error convention**: The workspace does not use `thiserror`. All error types use manual `Display + Error` impls. The orchestrator will define its own `OrchestratorError` in a separate `error.rs` module (a sibling task). The config module should use `OrchestratorError` as its error type, not define its own.

- **`OrchestratorError`** (defined in the sibling task) will have an `HttpError { url, reason }` variant. The config module needs error variants for config-loading failures. Since `OrchestratorError` does not have config-specific variants, the config functions should return `Result<Self, OrchestratorError>` by mapping config failures to a suitable variant. The task description specifies the return type as `Result<Self, OrchestratorError>`. Looking at the `OrchestratorError` enum, the most appropriate approach is to add a config-related mapping -- but since `OrchestratorError` is defined in a sibling task and we should not modify it, the config module should define a local `ConfigError` enum (following the `agent-runtime` pattern) and provide a `From<ConfigError> for OrchestratorError` conversion, or the `OrchestratorError` task should include a `ConfigError` variant. **Decision**: Follow the task description literally -- the functions return `Result<Self, OrchestratorError>`. The `OrchestratorError` enum (sibling task) should include a variant like `Config { reason: String }` for this purpose, or the config module should construct `OrchestratorError::HttpError` (which is a poor semantic fit). The cleanest approach: define a local `ConfigError` in config.rs and have the public-facing functions map it to `OrchestratorError`. See the Implementation Details section for the recommended approach.

## Requirements

1. **Define `OrchestratorConfig` struct** with a single field:
   - `agents: Vec<AgentConfig>` -- the list of agent registry entries.
   - Derive `Debug`, `Clone`, `serde::Deserialize`.

2. **Define `AgentConfig` struct** with three fields:
   - `name: String` -- unique agent identifier (e.g., `"cogs-analyst"`).
   - `description: String` -- human-readable description used for routing heuristics.
   - `url: String` -- the HTTP base URL of the agent endpoint (e.g., `"http://localhost:8081"`).
   - Derive `Debug`, `Clone`, `serde::Deserialize`.

3. **Implement `OrchestratorConfig::from_env()`** as the primary config path:
   - Signature: `pub fn from_env() -> Result<Self, OrchestratorError>`
   - Read `AGENT_ENDPOINTS` env var. It is **required** (no default). If missing or empty, return an error.
   - Format: comma-separated `name=url` pairs, e.g. `analyst=http://localhost:8081,writer=http://localhost:8082`.
   - Parsing rules (matching `TOOL_ENDPOINTS` pattern exactly):
     - Split on `,`.
     - Trim each segment; skip empty segments.
     - Split each segment on `=` via `split_once('=')`. Error if no `=` found.
     - Trim both name and url.
     - Validate that name is non-empty and url is non-empty.
   - Read `AGENT_DESCRIPTIONS` env var. It is **optional**. Format: comma-separated `name=description` pairs, e.g. `analyst=Financial analysis agent,writer=Content writer`. If unset, each agent gets an empty string as its description.
   - For each entry parsed from `AGENT_ENDPOINTS`, look up a matching description from `AGENT_DESCRIPTIONS` (by name). If no match, use `""`.
   - Construct `OrchestratorConfig { agents }` from the merged data.

4. **Implement `OrchestratorConfig::from_file()`** as the secondary config path:
   - Signature: `pub fn from_file(path: &str) -> Result<Self, OrchestratorError>`
   - Read the file at `path` using `std::fs::read_to_string`.
   - Parse as YAML using `serde_yaml::from_str`.
   - Map I/O errors and YAML parse errors to `OrchestratorError`.
   - The expected YAML format:
     ```yaml
     agents:
       - name: analyst
         description: Financial analysis agent
         url: http://localhost:8081
       - name: writer
         description: Content writer
         url: http://localhost:8082
     ```

5. **Error handling**: Functions return `Result<Self, OrchestratorError>`. Config-specific errors (missing env var, invalid format, file I/O, YAML parse) should be mapped to an appropriate `OrchestratorError` variant. If `OrchestratorError` does not have a dedicated config variant, use a reasonable mapping (e.g., construct an error with a descriptive message). The recommended approach is to coordinate with the `OrchestratorError` sibling task to include a `Config { reason: String }` variant.

6. **No new dependencies beyond what the sibling Cargo.toml task provides**: The Cargo.toml task adds `serde` (with `derive`), `serde_yaml`, and others. The config module uses `serde::Deserialize` and `serde_yaml`. No additional crates are needed.

7. **Empty strings treated as missing**: Following the `agent-runtime` pattern, an env var set to `""` should be treated the same as unset.

8. **Functions must be 50 lines or fewer**: Break parsing logic into helpers as needed to respect the project's function-length rule.

## Implementation Details

### Files to create

- **`crates/orchestrator/src/config.rs`** -- new module containing `OrchestratorConfig`, `AgentConfig`, and their constructors.

### Files that must exist first (created by sibling tasks)

- **`crates/orchestrator/src/lib.rs`** -- must declare `pub mod config;` (created by the "Convert orchestrator from binary to library crate" task).
- **`crates/orchestrator/Cargo.toml`** -- must have `serde` and `serde_yaml` dependencies (created by the "Update orchestrator Cargo.toml with dependencies" task).
- **`crates/orchestrator/src/error.rs`** -- must define `OrchestratorError` (created by the "Define OrchestratorError enum" task). Note: `config.rs` imports `OrchestratorError` from `crate::error`.

### Types

```
OrchestratorConfig (struct)
  - agents: Vec<AgentConfig>
  Derives: Debug, Clone, serde::Deserialize

AgentConfig (struct)
  - name: String
  - description: String
  - url: String
  Derives: Debug, Clone, serde::Deserialize
```

### Key functions

```
OrchestratorConfig::from_env() -> Result<Self, OrchestratorError>
```

Logic:
1. Read `AGENT_ENDPOINTS` via a helper (see below). If missing/empty, return error.
2. Parse into a `Vec<(String, String)>` of `(name, url)` pairs via `parse_comma_pairs`.
3. Read `AGENT_DESCRIPTIONS` via `std::env::var`. If present and non-empty, parse into a `HashMap<String, String>` of `(name, description)` via the same `parse_comma_pairs` helper.
4. For each `(name, url)`, look up description from the map (defaulting to `""`), construct `AgentConfig`.
5. Return `Ok(OrchestratorConfig { agents })`.

```
OrchestratorConfig::from_file(path: &str) -> Result<Self, OrchestratorError>
```

Logic:
1. `std::fs::read_to_string(path)` -- map `io::Error` to `OrchestratorError`.
2. `serde_yaml::from_str(&contents)` -- map `serde_yaml::Error` to `OrchestratorError`.
3. Return the deserialized `OrchestratorConfig`.

### Helper functions (private)

```
fn read_required_env(name: &str) -> Result<String, OrchestratorError>
```
Read `std::env::var(name)`. Return error if `Err` or if value is empty. Mirrors `read_required_var` from `agent-runtime/src/config.rs`.

```
fn parse_comma_pairs(input: &str, var_name: &str) -> Result<Vec<(String, String)>, OrchestratorError>
```
Split `input` on `,`, trim each segment, skip empty segments, split each on `=` via `split_once('=')`, error if no `=` found (include the offending segment and `var_name` in the error message), trim both halves, validate non-empty, collect into `Vec<(String, String)>`.

### Integration points

- The `Orchestrator::from_config(config: OrchestratorConfig)` method (Group 3 task) consumes this config to build `AgentEndpoint` instances.
- The orchestrator's startup path (when wired into `agent-runtime`) will call `OrchestratorConfig::from_env()` or `OrchestratorConfig::from_file()` early in initialization.
- `AgentConfig` fields map 1:1 to `AgentEndpoint::new(name, description, url)` parameters.

### YAML example file format

```yaml
agents:
  - name: cogs-analyst
    description: Financial analysis and reporting agent
    url: http://localhost:8081
  - name: skill-writer
    description: Writes and refines skill manifests
    url: http://localhost:8082
```

### Env var example

```bash
AGENT_ENDPOINTS="cogs-analyst=http://localhost:8081,skill-writer=http://localhost:8082"
AGENT_DESCRIPTIONS="cogs-analyst=Financial analysis and reporting agent,skill-writer=Writes and refines skill manifests"
```

## Dependencies

- Blocked by: "Convert orchestrator from binary to library crate" (provides `lib.rs` with `pub mod config;`), "Update orchestrator Cargo.toml with dependencies" (provides `serde`, `serde_yaml`), "Define OrchestratorError enum" (provides the error type used in return values)
- Blocking: "Implement Orchestrator struct with dispatch logic" (consumes `OrchestratorConfig` via `Orchestrator::from_config`)

## Risks & Edge Cases

- **`OrchestratorError` may lack a config variant**: If the sibling `error.rs` task does not include a `Config { reason: String }` variant (or equivalent), the config module will need to shoehorn config errors into an ill-fitting variant. Mitigation: coordinate with the error task to include a suitable variant, or use a generic variant pattern like `OrchestratorError::Config { reason: String }`. If neither is available, wrapping config errors in `OrchestratorError::HttpError { url: "config".into(), reason }` is a last resort (semantically poor but functional).

- **Duplicate agent names in `AGENT_ENDPOINTS`**: The current design does not explicitly reject duplicates. The downstream `Orchestrator` uses a `HashMap<String, AgentEndpoint>` keyed by name, so the last entry wins silently. This is acceptable for the initial implementation but should be documented. A future enhancement could reject duplicates at config-parse time.

- **Descriptions without matching endpoints**: If `AGENT_DESCRIPTIONS` contains a name not present in `AGENT_ENDPOINTS`, the extra description is silently ignored. This is the expected behavior -- descriptions are supplementary metadata.

- **URL validation**: The config module does NOT validate that `url` values are well-formed URLs. Validation happens implicitly when `AgentEndpoint` attempts to connect. This keeps the config module simple and avoids adding a URL-parsing dependency.

- **Empty `AGENT_ENDPOINTS` after trimming**: If the env var is set to `","` or `"  "`, all segments are empty after trimming and skipping, resulting in an empty `agents` list. This should be treated as an error (at least one agent is required for the orchestrator to be useful). Alternatively, allow zero agents and let the `Orchestrator` handle the empty-registry case. **Decision**: Allow an empty list -- the config module's job is parsing, not business-rule validation. The `Orchestrator` can decide whether zero agents is an error.

- **File not found for `from_file`**: `std::fs::read_to_string` returns `io::Error` with `ErrorKind::NotFound`. This should be surfaced clearly in the error message, including the file path.

- **YAML with extra fields**: `serde_yaml` by default ignores extra fields (unless `#[serde(deny_unknown_fields)]` is used). The initial implementation should allow extra fields for forward compatibility.

- **Test isolation for env-var tests**: Tests that set/unset `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` must use the `ENV_LOCK: Mutex<()>` + `with_env_vars` pattern from `agent-runtime/src/config.rs` to avoid flaky parallel test failures. The test file is a separate task ("Write unit tests for config loading"), but the config module's design should be test-friendly (pure functions that take parsed strings, not just env-reading functions).

- **`=` in URL values**: URLs may contain `=` characters (e.g., query parameters). Since `split_once('=')` splits on the first `=` only, the URL portion correctly retains any subsequent `=` characters. Example: `name=http://host?foo=bar` parses as name=`name`, url=`http://host?foo=bar`.

## Verification

1. `cargo check -p orchestrator` passes with no errors (requires sibling tasks to be complete first).
2. `cargo clippy -p orchestrator` reports no warnings for the new module.
3. `cargo test -p orchestrator` continues to pass (no regressions; dedicated config tests are a separate task).
4. The `config.rs` file contains exactly the types and functions described above.
5. No new dependencies are added beyond those specified in the sibling Cargo.toml task.
6. All functions are 50 lines or fewer.
7. No commented-out code or debug statements remain in the file.
