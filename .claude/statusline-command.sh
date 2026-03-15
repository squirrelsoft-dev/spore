#!/bin/sh
# Claude Code statusline — devcontainer-style prompt
# Reads JSON context from stdin, outputs a single ANSI-formatted line

set -e

# Colors
GREEN='\033[0;32m'
BOLD_BLUE='\033[1;34m'
CYAN='\033[0;36m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
DIM='\033[2m'
RESET='\033[0m'

# Read JSON from stdin
input=$(cat)
cwd=$(echo "$input" | jq -r '.workspace.current_dir // empty')
model=$(echo "$input" | jq -r '.model.display_name // empty')
used_pct=$(echo "$input" | jq -r '.context_window.used_percentage // empty')

# Username
user=$(whoami)

# Shorten home directory to ~
if [ -n "$cwd" ]; then
    display_dir=$(echo "$cwd" | sed "s|^$HOME|~|")
else
    display_dir=$(pwd | sed "s|^$HOME|~|")
fi

# Git branch
branch=""
if git_branch=$(git --no-optional-locks symbolic-ref --short HEAD 2>/dev/null); then
    branch="$git_branch"
fi

# Context usage color
ctx_color="$GREEN"
if [ -n "$used_pct" ]; then
    pct_int=$(printf '%.0f' "$used_pct" 2>/dev/null || echo "0")
    if [ "$pct_int" -ge 80 ]; then
        ctx_color="$RED"
    elif [ "$pct_int" -ge 50 ]; then
        ctx_color="$YELLOW"
    fi
fi

# Build output
out=""
out="${out}${GREEN}${user}${RESET} "
out="${out}${BOLD_BLUE}${display_dir}${RESET}"

if [ -n "$branch" ]; then
    out="${out} ${CYAN}(${RED}${branch}${CYAN})${RESET}"
fi

if [ -n "$model" ]; then
    out="${out} ${DIM}[${model}]${RESET}"
fi

if [ -n "$used_pct" ]; then
    pct_fmt=$(printf '%.0f' "$used_pct" 2>/dev/null || echo "$used_pct")
    out="${out} ${ctx_color}ctx:${pct_fmt}%%${RESET}"
fi

printf '%b' "$out"
