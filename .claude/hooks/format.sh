#!/usr/bin/env bash
# PostToolUse format hook — detects the project formatter and formats the changed file

INPUT=$(cat)
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty')

# Nothing to format if no file path
[[ -z "$FILE_PATH" ]] && exit 0

# Skip non-existent files (e.g. deleted)
[[ ! -f "$FILE_PATH" ]] && exit 0

# --- Node/JS/TS projects ---
if [[ -f "$CLAUDE_PROJECT_DIR/package.json" ]]; then
  # Biome: check package.json deps or biome.json config
  if [[ -f "$CLAUDE_PROJECT_DIR/biome.json" ]] || [[ -f "$CLAUDE_PROJECT_DIR/biome.jsonc" ]] || grep -q '"@biomejs/biome"' "$CLAUDE_PROJECT_DIR/package.json" 2>/dev/null; then
    npx @biomejs/biome format --write "$FILE_PATH" 2>/dev/null || true
    exit 0
  fi

  # Prettier: check package.json deps or config files
  if [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.json" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.js" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.cjs" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.mjs" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.yaml" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.yml" ]] || [[ -f "$CLAUDE_PROJECT_DIR/.prettierrc.toml" ]] || [[ -f "$CLAUDE_PROJECT_DIR/prettier.config.js" ]] || [[ -f "$CLAUDE_PROJECT_DIR/prettier.config.cjs" ]] || [[ -f "$CLAUDE_PROJECT_DIR/prettier.config.mjs" ]] || grep -q '"prettier"' "$CLAUDE_PROJECT_DIR/package.json" 2>/dev/null; then
    npx prettier --write "$FILE_PATH" 2>/dev/null || true
    exit 0
  fi

  # Fallback for Node projects: try prettier then biome
  npx prettier --write "$FILE_PATH" 2>/dev/null || npx @biomejs/biome format --write "$FILE_PATH" 2>/dev/null || true
  exit 0
fi

# --- Python projects ---
if [[ -f "$CLAUDE_PROJECT_DIR/pyproject.toml" ]] || [[ -f "$CLAUDE_PROJECT_DIR/requirements.txt" ]] || [[ -f "$CLAUDE_PROJECT_DIR/setup.py" ]]; then
  ruff format "$FILE_PATH" 2>/dev/null || black "$FILE_PATH" 2>/dev/null || true
  exit 0
fi

# --- Rust projects ---
if [[ -f "$CLAUDE_PROJECT_DIR/Cargo.toml" ]]; then
  rustfmt "$FILE_PATH" 2>/dev/null || true
  exit 0
fi

# --- Go projects ---
if [[ -f "$CLAUDE_PROJECT_DIR/go.mod" ]]; then
  gofmt -w "$FILE_PATH" 2>/dev/null || true
  exit 0
fi
