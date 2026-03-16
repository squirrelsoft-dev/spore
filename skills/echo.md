---
name: echo
version: "1.0"
description: Echoes input back for testing
model:
  provider: anthropic
  name: claude-haiku-4-5-20251001
  temperature: 0.0
tools: []
constraints:
  max_turns: 1
  confidence_threshold: 1.0
  allowed_actions: []
output:
  format: text
  schema: {}
---
Echo back the input exactly as received. Do not modify, summarize, or interpret.
