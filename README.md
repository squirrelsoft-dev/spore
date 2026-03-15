# Spore

A micro agent architecture platform. Deploy single-responsibility AI agents as lightweight containers — each one a generic runtime paired with a declarative skill file.

## Why

Most AI agents are monolithic: one massive system prompt with routing logic, business rules, and domain knowledge tangled together. Hard to debug, hard to test, fragile under load.

Spore applies the microservices principle to agents. Each agent does exactly one thing. Swap the skill file, get a different agent. The runtime never changes.

## How It Works

Every micro agent is two things:

1. **Runtime** — A statically compiled Rust binary. No domain knowledge. It loads a skill file, connects to a language model, manages tool calls, enforces constraints, and returns structured output. Ships as a 1-5MB Docker image.

2. **Skill file** — A YAML document declaring everything the agent needs: identity, system prompt, permitted tools, constraints, confidence thresholds, and output schema.

```yaml
name: cogs-analyst
version: 1.0.0
description: Handles COGS-related finance queries

model:
  provider: anthropic
  name: claude-sonnet-4-6
  temperature: 0.1

preamble: |
  You are a finance analyst specializing in Cost of Goods Sold analysis.
  Never speculate. If confidence is below threshold, escalate.

tools:
  - get_account_groups
  - execute_sql
  - query_store_lookup

constraints:
  max_turns: 5
  confidence_threshold: 0.75
  escalate_to: general-finance-agent
  allowed_actions:
    - read
    - query

output:
  format: structured_json
  schema:
    sql: string
    explanation: string
    confidence: float
    source: string
```

## Architecture

```
spore/
├── crates/
│   ├── agent-runtime/      # The deployable binary
│   ├── skill-loader/       # Parses and validates skill files
│   ├── tool-registry/      # Discovers and serves tools via MCP
│   ├── orchestrator/       # Routes requests to agents
│   └── agent-sdk/          # Shared traits and types
├── skills/                 # Skill file definitions (YAML)
├── tools/                  # Tool implementations
└── Cargo.toml
```

### Crates

**`agent-sdk`** — Shared contract. Defines `SkillManifest`, `MicroAgent` trait, and the `AgentRequest`/`AgentResponse` envelope. Everything else depends on this.

**`skill-loader`** — Parses YAML skill files into typed `SkillManifest` structs. Validation is strict and happens at startup — if a skill references a tool that doesn't exist, the agent fails to start, not to respond.

**`tool-registry`** — Tools run as MCP servers. The registry maps tool names to MCP endpoints and hands back handles the runtime can use. Tools are independently deployable and versioned. They have no knowledge of which agent calls them.

**`agent-runtime`** — The deployable binary. Loads the skill file, resolves tools, builds a `rig-core` agent, and serves an HTTP API via `axum`. The Docker image is `FROM scratch` + the binary + the skill file.

**`orchestrator`** — A micro agent itself, with a routing skill. Reads incoming intent, dispatches to the right agent, and handles escalation when a downstream agent returns below its confidence threshold.

### Agent Tiers

Agents organize into tiers, all built from the same primitive (runtime + skill file):

- **Orchestrator** — routing only, no business logic
- **Domain agents** — each owns a specific capability slice
- **Utility agents** — validators, formatters, query lookups — invoked as sub-tasks

The orchestrator is optional. Agents are independently addressable and can be fronted by an API gateway, wired into workflow tools (Make, n8n), or invoked from webhooks, cron jobs, or other agents.

## Self-Bootstrapping Factory

The system builds itself from two seed agents:

| Seed Agent | Skill | What It Does |
|---|---|---|
| **Skill Writer** | `skill-writer.yaml` | Takes a plain-language capability description, produces a validated YAML skill file |
| **Tool Coder** | `tool-coder.yaml` | Reads a skill file, identifies missing tools, implements them in Rust |

The third agent they produce together is the **Deploy Agent** (`deploy-agent.yaml`), which packages runtime + skill file into a Docker image and registers the endpoint. From that point the system is fully self-extending — new agents are described, written, tooled, and deployed entirely within the platform.

## Tech Stack

| Layer | Choice | Why |
|---|---|---|
| Agent engine | [`rig-core`](https://github.com/0xPlaygrounds/rig) | Trait-based, async-first, tool servers via message passing |
| Tool protocol | [`rmcp`](https://github.com/anthropics/rust-mcp) | Official Rust MCP SDK for tool interoperability |
| HTTP surface | `axum` | Lightweight, Tokio-native |
| Async runtime | `tokio` | Standard Rust async runtime |
| Serialization | `serde` / `schemars` | Skill file parsing and JSON schema generation |

## Key Properties

**Single-responsibility agents.** Each agent does one thing. Debug, test, and version them independently.

**Tiny containers.** Static Rust binaries with no dependencies. 1-5MB Docker images vs. gigabyte-scale typical AI containers.

**Skill files are text.** Diff them, review them in PRs, roll them back independently, test them against known datasets before promoting to production.

**Compile-time safety.** Tool contracts are enforced at the type level. If a tool implementation doesn't match what the skill file expects, it doesn't compile. Constraints on which tools an agent can call are structural, not runtime checks.

**Horizontal scaling.** Agents are stateless process managers that call a model API. Spin up more pods when demand increases, tear them down when it doesn't. No minimum GPU requirement for the agents themselves.

## Analogy

Serverless, but for agents. The runtime is the execution environment, the skill file is the function code, the container is the deployment unit. Except instead of executing code, it's managing an LLM conversation scoped to a single capability.

## License

TBD
