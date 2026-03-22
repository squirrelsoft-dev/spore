# validate-skill

An MCP tool server that validates skill file YAML frontmatter against the `SkillManifest` schema. It parses the frontmatter, checks required fields and structure, and returns a JSON report indicating whether the content is valid.

## Build

```sh
cargo build -p validate-skill
```

## Run

```sh
cargo run -p validate-skill
```

The server uses stdio transport: it reads MCP messages from stdin and writes responses to stdout. All logging output is directed to stderr so it does not interfere with the MCP protocol stream.

## Test with MCP Inspector

```sh
npx @modelcontextprotocol/inspector cargo run -p validate-skill
```

This launches the MCP Inspector, which connects to the validate-skill server and provides an interactive UI for sending requests and viewing responses. Use it to verify that the tool advertises its capabilities and handles calls correctly.

## Test

```sh
cargo test -p validate-skill
```

## Parameters

| Name      | Type   | Description                                              |
|-----------|--------|----------------------------------------------------------|
| `content` | string | Full skill file content (markdown with YAML frontmatter) |

## Output

The tool returns a JSON response with the following fields:

| Field      | Type    | Description                                         |
|------------|---------|-----------------------------------------------------|
| `valid`    | boolean | Whether the skill file content passed validation    |
| `errors`   | array   | List of error strings (empty on success)            |
| `manifest` | object  | Parsed `SkillManifest` (present only on success)    |

### Success example

```json
{
  "valid": true,
  "errors": [],
  "manifest": {
    "name": "my-skill",
    "version": "1.0.0",
    "description": "A skill description",
    ...
  }
}
```

### Failure example

```json
{
  "valid": false,
  "errors": ["name must not be empty"]
}
```

## Notes

This tool uses the `AllToolsExist` strategy for tool validation, which performs structural validation only. It checks that the YAML frontmatter conforms to the `SkillManifest` schema without verifying that referenced tools are actually registered or available at runtime.
