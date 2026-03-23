# Spec: Define the test scenario document

> From: issue #22

## Objective

Create `tests/e2e/SCENARIO.md`, a test scenario document that defines the temperature-conversion agent as the canonical end-to-end test case for the Spore pipeline. The document specifies the natural-language input, the expected outputs at each pipeline stage (skill-writer, tool-coder, deploy-agent, orchestrator), and the success criteria validators must check. This file is the single source of truth that step validators and the E2E test script depend on.

## Current State

- The `tests/e2e/` directory does not exist yet; it must be created along with the scenario file.
- The Spore pipeline consists of four agents executed in sequence:
  1. **skill-writer** -- accepts a plain-language description and produces a validated skill file (markdown with YAML frontmatter). Output: `{ skill_yaml: string, validation_result: string }`.
  2. **tool-coder** -- accepts a skill file or tool list and generates Rust MCP tool crate(s). Output: `{ tools_generated: string, compilation_result: string, implementation_paths: string }`.
  3. **deploy-agent** -- packages the runtime binary and skill file into a Docker image, pushes to a registry, and registers with the orchestrator. Output: `{ image_uri: string, endpoint_url: string, health_check: string }`.
  4. **orchestrator** -- routes incoming requests to the best-matching agent. Output: `{ target_agent: string, reasoning: string }`.
- The `/invoke` API (`POST /invoke`) accepts `AgentRequest { id: Uuid, input: String, context: Option<Value>, caller: Option<String> }` and returns `AgentResponse { id: Uuid, output: Value, confidence: f32, escalated: bool, escalate_to: Option<String>, tool_calls: Vec<ToolCallRecord> }`.
- The `SkillManifest` struct has 8 fields: `name`, `version`, `description`, `model` (ModelConfig), `preamble`, `tools` (Vec<String>), `constraints` (Constraints), `output` (OutputSchema).

## Requirements

- The scenario document must be a markdown file at `tests/e2e/SCENARIO.md`.
- It must define the **seed input**: a natural-language prompt describing a temperature-conversion agent (e.g., "Create an agent that converts temperatures between Celsius, Fahrenheit, and Kelvin").
- It must specify expected outputs and success criteria for each of the four pipeline stages, structured so that automated validators can reference specific field names and value patterns.
- It must define the expected **skill file structure**: the SkillManifest fields and their expected values or value patterns for a temperature-conversion skill.
- It must define the expected **MCP tool**: the tool name, its request schema (input parameters), and its output behavior.
- It must be purely a specification document -- no executable code, no implementation.
- The document must be usable as a reference by both human reviewers and automated test scripts.

## Implementation Details

### File to create

**`tests/e2e/SCENARIO.md`**

The document should contain the following sections:

#### 1. Overview

Brief description of the test scenario: a single natural-language request flows through the full Spore pipeline to produce a deployed temperature-conversion agent.

#### 2. Seed Input

The exact `AgentRequest.input` string to feed into the pipeline:

```
Create an agent that converts temperatures between Celsius, Fahrenheit, and Kelvin
```

The full `AgentRequest` payload:
- `id`: auto-generated UUID
- `input`: the seed string above
- `context`: `null`
- `caller`: `null`

#### 3. Stage 1: skill-writer

**Input**: The seed input string.

**Expected skill file structure** (SkillManifest fields):
- `name`: `"temperature-converter"` (or close variant like `"temp-converter"`)
- `version`: a quoted string, e.g., `"0.1"`
- `description`: contains the words "temperature" and "convert" (case-insensitive)
- `model.provider`: `"anthropic"`
- `model.name`: a valid model identifier (non-empty string)
- `model.temperature`: a float between 0.0 and 1.0
- `tools`: list containing exactly one tool name: `"convert_temperature"`
- `constraints.max_turns`: integer > 0
- `constraints.confidence_threshold`: float in [0.0, 1.0]
- `constraints.allowed_actions`: non-empty list
- `output.format`: one of `"json"`, `"structured_json"`, or `"text"`
- `output.schema`: non-empty map with at least a result field
- `preamble`: non-empty string containing behavioral instructions

**Success criteria**:
- `validation_result` indicates no errors
- The skill YAML parses into a valid `SkillManifest`
- All 9 validation rules from the skill-writer spec pass (required non-empty strings, confidence range, max turns positive, tool existence, output format validity, version quoting, no standalone triple-dash in preamble)

#### 4. Stage 2: tool-coder

**Input**: The skill file produced by Stage 1 (specifically the `tools` list: `["convert_temperature"]`).

**Expected tool crate structure**:
- Crate directory: `tools/convert-temperature/`
- Files: `Cargo.toml`, `src/main.rs`, `src/convert_temperature.rs`, `README.md`
- `Cargo.toml` dependencies: `rmcp`, `tokio`, `serde`, `serde_json`, `tracing`, `tracing-subscriber`
- Request struct fields: at minimum `value: f64`, `from_unit: String`, `to_unit: String` (or equivalent)
- Tool method: `#[tool(description = "...")]` annotated method that performs the conversion
- The tool implements the MCP `ServerHandler` trait

**Expected tool behavior**:
- Converts between Celsius (C), Fahrenheit (F), and Kelvin (K)
- Formulas: C-to-F = C * 9/5 + 32, F-to-C = (F - 32) * 5/9, C-to-K = C + 273.15, K-to-C = K - 273.15
- Returns a string representation of the converted value

**Success criteria**:
- `tools_generated` contains `"convert_temperature"`
- `compilation_result` is `"success"` (the crate compiles with `cargo build -p convert-temperature`)
- `implementation_paths` contains `"tools/convert-temperature"`
- The generated crate follows the MCP tool implementation pattern (two-file structure with tool_router and tool_handler macros)

#### 5. Stage 3: deploy-agent

**Input**: The skill name `"temperature-converter"` and version from Stage 1.

**Expected outputs**:
- `image_uri`: matches pattern `{REGISTRY_URL}/spore-temperature-converter:{version}`
- `endpoint_url`: a valid HTTP URL (e.g., `http://temperature-converter:8080`)
- `health_check`: `"healthy"`

**Success criteria**:
- Docker image builds successfully (two-stage build, final image FROM scratch)
- Image is tagged following the `spore-{name}:{version}` convention
- Image is pushed to the registry
- Agent is registered with the orchestrator via `register_agent`
- Health endpoint (`GET /health`) returns 200 OK with `status: "Healthy"`

#### 6. Stage 4: orchestrator

**Input**: A user request routed through the orchestrator, e.g.:
```
Convert 100 degrees Celsius to Fahrenheit
```

**Expected outputs**:
- `target_agent`: `"temperature-converter"`
- `reasoning`: non-empty string explaining why this agent was selected

**Success criteria**:
- The orchestrator identifies the temperature-converter agent via `list_agents`
- Routes the request to the correct agent with confidence >= 0.9
- The routed request reaches the deployed agent and returns the correct conversion result (212 degrees Fahrenheit)

#### 7. End-to-End Success Criteria

The full pipeline passes if and only if all four stages pass their individual success criteria AND:
- The pipeline completes without any escalation (`escalated: false` at each stage)
- No `AgentError` variants are returned at any stage
- The final output contains the correct temperature conversion result
- Total pipeline latency is recorded (no hard threshold, but logged for regression tracking)

#### 8. Validator Reference Table

A summary table mapping each stage to its validator script (to be implemented) and the key assertions:

| Stage | Validator | Key Assertions |
|---|---|---|
| skill-writer | `validate_skill_output.rs` | valid SkillManifest, tool name = `convert_temperature`, no validation errors |
| tool-coder | `validate_tool_output.rs` | crate compiles, correct file structure, MCP pattern followed |
| deploy-agent | `validate_deploy_output.rs` | image built, health check healthy, agent registered |
| orchestrator | `validate_routing.rs` | correct target agent, confidence >= threshold, correct final answer |

## Dependencies

- Blocked by: nothing (this is a specification document with no code dependencies)
- Blocking: step validators (`validate_skill_output.rs`, `validate_tool_output.rs`, `validate_deploy_output.rs`, `validate_routing.rs`) and the E2E test script -- these will reference the scenario document for expected values and success criteria

## Risks & Edge Cases

- **Skill name variability**: The skill-writer LLM may produce a name like `temp-converter` instead of `temperature-converter`. The scenario document should specify an acceptable pattern (contains "temp" and "convert") rather than requiring an exact string, while noting that downstream stages depend on the actual name produced.
- **Tool name variability**: Similarly, the tool could be named `temperature_converter` instead of `convert_temperature`. The document should specify the expected name but note that validators should accept reasonable variants.
- **Formula precision**: Floating-point conversion results may differ slightly. Validators should use approximate equality (epsilon = 0.01) rather than exact comparison.
- **Deployment environment**: Stage 3 (deploy-agent) requires Docker. The scenario document should note that the E2E test may skip Stage 3 in environments without Docker access, falling back to a mock deployment validator.
- **Orchestrator agent discovery**: Stage 4 depends on the temperature-converter agent being registered. If Stage 3 is skipped, the orchestrator test must use a pre-seeded agent registry.

## Verification

- The file `tests/e2e/SCENARIO.md` exists and is valid markdown.
- The document covers all four pipeline stages with explicit success criteria.
- The seed input is a concrete string (not a placeholder).
- Expected SkillManifest field values are specified with enough precision for automated validation.
- Expected tool crate structure matches the MCP tool implementation pattern from `tool-coder.md`.
- The validator reference table lists all four validators with their key assertions.
- A human reviewer can read the document and understand exactly what the E2E test should verify at each stage.
