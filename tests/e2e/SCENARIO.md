# E2E Test Scenario: Temperature-Conversion Agent

This document describes the end-to-end test scenario for the Spore self-bootstrapping pipeline. The scenario exercises all four pipeline stages by creating a temperature-conversion agent from a single natural-language prompt.

## Seed Input

The pipeline is kicked off with one plain-language description:

```
Create an agent that converts temperatures between Celsius, Fahrenheit, and Kelvin
```

This input is fed to Stage 1 (Skill Writer), and each subsequent stage consumes the output of the previous one.

---

## Stage 1 -- Skill Writer

The Skill Writer agent receives the seed input and produces a skill file in markdown-with-frontmatter format. The generated file must parse into a valid `SkillManifest` struct.

### Expected SkillManifest Fields

| Field | Expected Value / Pattern |
|---|---|
| `name` | `temperature-converter` (or similar descriptive identifier) |
| `version` | `"0.1"` (quoted string in YAML) |
| `description` | One-line summary mentioning temperature conversion between Celsius, Fahrenheit, and Kelvin |
| `model.provider` | `anthropic` |
| `model.name` | A valid model identifier (e.g., `claude-sonnet-4-6`) |
| `model.temperature` | Low value (e.g., `0.1` or `0.2`) since conversion is deterministic |
| `tools` | List containing `convert_temperature` |
| `constraints.max_turns` | Positive integer (e.g., `5` or `10`) |
| `constraints.confidence_threshold` | Float in `[0.0, 1.0]` (e.g., `0.9`) |
| `constraints.allowed_actions` | Subset of `[read, write, query, route, discover]` |
| `output.format` | One of `json`, `structured_json`, or `text` |
| `output.schema` | Key-value map describing response fields (e.g., `converted_value: string`, `from_unit: string`, `to_unit: string`) |
| `preamble` | Non-empty markdown body with behavioral instructions for the temperature-conversion agent |

### Validation Rules That Must Pass

- All required non-empty string fields are present and non-empty.
- `version` is quoted to avoid YAML float coercion.
- `confidence_threshold` is in `[0.0, 1.0]`.
- `max_turns` is greater than 0.
- `output.format` is one of the three allowed values.
- Preamble contains no standalone `---` lines.

---

## Stage 2 -- Tool Coder

The Tool Coder agent receives the skill file from Stage 1, identifies that `convert_temperature` is missing from the tool registry, and generates a complete Rust MCP tool crate.

### Expected File Structure

```
tools/convert-temperature/
  Cargo.toml
  src/
    main.rs
    convert_temperature.rs
  README.md
```

### Cargo.toml

- Package name: `convert-temperature`
- Edition: `2024`
- Dependencies: `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber` (matching the standard dependency block from the tool-coder skill)

### Request Struct

The `ConvertTemperatureRequest` struct (in `src/convert_temperature.rs`) must have the following fields:

| Field | Type | Description |
|---|---|---|
| `value` | `f64` | The numeric temperature value to convert |
| `from_unit` | `String` | Source unit: `"Celsius"`, `"Fahrenheit"`, or `"Kelvin"` |
| `to_unit` | `String` | Target unit: `"Celsius"`, `"Fahrenheit"`, or `"Kelvin"` |

The struct derives `Debug`, `serde::Deserialize`, and `schemars::JsonSchema`.

### Conversion Formulas

The tool must implement the following conversions:

| From | To | Formula |
|---|---|---|
| Celsius | Fahrenheit | F = C * 9/5 + 32 |
| Celsius | Kelvin | K = C + 273.15 |
| Fahrenheit | Celsius | C = (F - 32) * 5/9 |
| Fahrenheit | Kelvin | K = (F - 32) * 5/9 + 273.15 |
| Kelvin | Celsius | C = K - 273.15 |
| Kelvin | Fahrenheit | F = (K - 273.15) * 9/5 + 32 |

### main.rs

- Initializes tracing to stderr.
- Creates a `ConvertTemperatureTool` and serves it over stdio transport via `rmcp::ServiceExt`.
- Uses `#[tokio::main(flavor = "current_thread")]`.

### README.md

- Documents the tool purpose, build command (`cargo build -p convert-temperature`), run command, and MCP Inspector usage.

### Compilation

The crate must be added to the root `Cargo.toml` workspace members and must compile successfully with `cargo build -p convert-temperature`.

---

## Stage 3 -- Deploy Agent

The Deploy Agent packages the compiled agent runtime and skill file into a Docker container, pushes it to a registry, and registers the agent with the orchestrator.

### Expected Docker Image

| Property | Expected Value |
|---|---|
| Image tag | `spore-temperature-converter:0.1` |
| Base | `FROM scratch` (final stage) |
| Contents | Static `agent-runtime` binary, `/skills/` directory, CA certificates |
| Port | `8080` |
| User | Non-root (UID `1000`) |
| Image size | 1-5 MB |

### Expected Endpoint

| Property | Expected Value |
|---|---|
| Endpoint URL | `http://temperature-converter:8080` |
| Health check URL | `http://temperature-converter:8080/health` |
| Health check response | `200 OK` |

### Orchestrator Registration

The deploy agent registers the new agent with the orchestrator, providing:
- Agent name: `temperature-converter`
- Endpoint URL: `http://temperature-converter:8080`
- Capabilities: description and tools list from the skill manifest

---

## Stage 4 -- Orchestrator Routing

The Orchestrator receives user queries, matches them to the temperature-converter agent based on declared capabilities, and routes the request.

### Test Queries

| Query | Expected Agent | Expected Result |
|---|---|---|
| `"Convert 100F to Celsius"` | `temperature-converter` | Approximately `37.78` C |
| `"Convert 0K to Celsius"` | `temperature-converter` | Approximately `-273.15` C |
| `"What is 100C in Fahrenheit?"` | `temperature-converter` | Approximately `212.0` F |
| `"Convert 32F to Kelvin"` | `temperature-converter` | Approximately `273.15` K |

The orchestrator must:
1. Analyze the query intent.
2. Call `list_agents` to discover available agents.
3. Select `temperature-converter` based on capability matching.
4. Route the request via `route_to_agent`.
5. Return the converted value.

---

## Success Criteria

| Stage | Validator Script | Key Assertions |
|---|---|---|
| Stage 1 -- Skill Writer | `validate_skill_output.sh` | Skill file parses into a valid `SkillManifest`; all 8 fields present; `tools` contains `convert_temperature`; `version` is a quoted string; `output.format` is an allowed value; preamble is non-empty |
| Stage 2 -- Tool Coder | `validate_tool_output.sh` | Crate directory exists at `tools/convert-temperature/`; `Cargo.toml`, `src/main.rs`, `src/convert_temperature.rs`, `README.md` all present; request struct has `value`, `from_unit`, `to_unit` fields; crate compiles with `cargo build -p convert-temperature` |
| Stage 3 -- Deploy Agent | `validate_deploy_output.sh` | Docker image tagged `spore-temperature-converter:0.1` exists; image size under 10 MB; container starts and exposes port 8080; health endpoint returns 200 OK; agent registered with orchestrator |
| Stage 4 -- Orchestrator | `validate_routing_output.sh` | Orchestrator routes temperature queries to `temperature-converter`; returned values are within +/- 0.1 of expected results; non-temperature queries are not routed to this agent |

---

## Non-determinism Note

Because each pipeline stage is driven by an LLM, outputs are non-deterministic. Validator scripts check **structure and constraints**, not exact content. Specifically:

- **Field presence over exact values**: Validators confirm that required fields exist and have the correct types, but do not assert exact string content for fields like `description` or `preamble`.
- **Numeric tolerance**: Conversion results are compared within a tolerance of +/- 0.1 to account for floating-point representation and rounding differences.
- **Name flexibility**: The agent name may vary slightly (e.g., `temperature-converter` vs `temp-converter`). Validators match against a pattern rather than a fixed string.
- **Schema keys**: The `output.schema` map must contain keys related to conversion results, but exact key names may vary.
- **Preamble content**: Validators check that the preamble is non-empty and contains temperature-related keywords, not that it matches a specific paragraph.

This approach ensures the E2E test is robust against LLM variability while still verifying that the pipeline produces correct, functional output at every stage.
