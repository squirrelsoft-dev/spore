# Spec: Write `README.md`
> From: .claude/tasks/issue-51.md

## Objective
Create `tools/list-agents/README.md` that documents the list-agents MCP tool server, following the established pattern from `tools/register-agent/README.md`.

## Current State
No `tools/list-agents/` directory or README exists yet. The `tools/register-agent/README.md` serves as the canonical template for tool documentation, covering description, build/run/test commands, MCP Inspector usage, parameters, output format, environment variables, and security considerations.

## Requirements
1. **Description**: One-line summary explaining that list-agents is an MCP tool server that reads agent registrations from environment variables and returns them as a filtered JSON array.
2. **Build section**: `cargo build -p list-agents`
3. **Run section**: `cargo run -p list-agents` with note about stdio transport (stdin/stdout for MCP, stderr for logging).
4. **MCP Inspector section**: `npx @modelcontextprotocol/inspector cargo run -p list-agents` with brief explanation.
5. **Test section**: `cargo test -p list-agents`
6. **Parameters table**: Single optional parameter `filter` (type `string`, required `no`) for case-insensitive substring matching against agent name or description.
7. **Output section**: Document the JSON response shape `{"agents": [{name, url, description}, ...]}` with optional `error` field. Include a success example with multiple agents, a filtered example, an empty-result example, and an error example.
8. **Environment Variables table**: Document `AGENT_ENDPOINTS` (comma-separated `name=url` pairs, required for any agents to appear) and `AGENT_DESCRIPTIONS` (comma-separated `name=description` pairs, optional — agents without a matching description get an empty string).
9. **Usage examples**: Show how to set env vars and invoke the tool, including filter usage.

## Implementation Details
- Mirror the exact markdown structure of `tools/register-agent/README.md`: heading hierarchy, table formatting, code block language tags, section ordering.
- Sections in order: description, Build, Run, Test with MCP Inspector, Test, Parameters, Output (with sub-examples), Environment Variables.
- Since list-agents makes no HTTP calls and only reads env vars, omit the Security Considerations section. This is a key difference from register-agent — there is no shell execution, no URL validation needed, and no network requests.
- The `AGENT_ENDPOINTS` format is `name=url,name2=url2`. The `AGENT_DESCRIPTIONS` format is `name=description,name2=description2`. Document both with examples.
- Output examples to include:
  - **Success with agents**: Two agents returned, both with name/url/description populated.
  - **Empty result**: `{"agents": []}` when no env vars are set or no agents match the filter.
  - **Error example**: `{"agents": [], "error": "..."}` when env var parsing fails.

## Dependencies
- `tools/register-agent/README.md` — pattern to follow (already exists)
- `.claude/tasks/issue-51.md` — task definition with parameter and output details

## Risks & Edge Cases
- The README must stay consistent with the actual implementation once built. If parameter names or output fields change during implementation, the README must be updated accordingly.
- The env var format documentation must exactly match the parsing logic in `crates/orchestrator/src/config.rs` (`parse_comma_pairs`).

## Verification
- Confirm the README follows the same structure as `tools/register-agent/README.md` (heading order, table format, code block style).
- Confirm all sections from the task description are present: description, build/run/test commands, MCP Inspector command, input parameters, output format, env var configuration, usage examples.
- Confirm the parameter table lists `filter` as optional with type `string`.
- Confirm the output format documents the `{"agents": [...]}` shape with `name`, `url`, and `description` fields per agent.
- Confirm both `AGENT_ENDPOINTS` and `AGENT_DESCRIPTIONS` env vars are documented with their format.
