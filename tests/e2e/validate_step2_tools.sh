#!/usr/bin/env bash
set -euo pipefail

# === Step 2: Tool-coder invocation validator ===
# Sends a generated skill file to the tool-coder agent and validates
# that it produces compiled tools with valid implementation paths.

ARTIFACTS_DIR="${ARTIFACTS_DIR:-tests/e2e/artifacts}"
SKILL_FILE="${ARTIFACTS_DIR}/generated-skill.md"
RESPONSE_FILE="${ARTIFACTS_DIR}/step2-response.json"
TOOL_CODER_URL="${TOOL_CODER_URL:-http://tool-coder:8080}"
MAX_RETRIES="${MAX_RETRIES:-3}"
REQUEST_TIMEOUT="${REQUEST_TIMEOUT:-120}"
RETRY_DELAY=5

generate_uuid() {
    if command -v uuidgen &>/dev/null; then
        uuidgen | tr '[:upper:]' '[:lower:]'
    elif [[ -f /proc/sys/kernel/random/uuid ]]; then
        cat /proc/sys/kernel/random/uuid
    else
        printf '%04x%04x-%04x-%04x-%04x-%04x%04x%04x' \
            $RANDOM $RANDOM $RANDOM $(( (RANDOM & 0x0FFF) | 0x4000 )) \
            $(( (RANDOM & 0x3FFF) | 0x8000 )) $RANDOM $RANDOM $RANDOM
    fi
}

validate_response() {
    local response_file="$1"

    # Check compilation_result contains "success" (case-insensitive)
    local compilation_result
    compilation_result=$(jq -r '.output.compilation_result // empty' "$response_file" 2>/dev/null)
    if [[ -z "$compilation_result" ]]; then
        echo "FAIL: output.compilation_result is missing or null"
        echo "  Raw output: $(jq -c '.output' "$response_file" 2>/dev/null)"
        return 1
    fi
    if ! echo "$compilation_result" | grep -qi "success"; then
        echo "FAIL: output.compilation_result does not contain 'success'"
        echo "  Actual value: $compilation_result"
        return 1
    fi

    # Check tools_generated is non-empty
    local tools_generated
    tools_generated=$(jq -r '.output.tools_generated // empty' "$response_file" 2>/dev/null)
    if [[ -z "$tools_generated" ]]; then
        echo "FAIL: output.tools_generated is missing, null, or empty"
        echo "  Raw output: $(jq -c '.output' "$response_file" 2>/dev/null)"
        return 1
    fi

    # Check implementation_paths is non-empty and entries start with tools/
    local implementation_paths
    implementation_paths=$(jq -r '.output.implementation_paths // empty' "$response_file" 2>/dev/null)
    if [[ -z "$implementation_paths" ]]; then
        echo "FAIL: output.implementation_paths is missing, null, or empty"
        echo "  Raw output: $(jq -c '.output' "$response_file" 2>/dev/null)"
        return 1
    fi

    # Validate each comma-separated path starts with tools/
    local IFS=','
    for path_entry in $implementation_paths; do
        local trimmed
        trimmed=$(echo "$path_entry" | xargs)
        if [[ "$trimmed" != tools/* ]]; then
            echo "FAIL: implementation_path entry does not start with 'tools/': '$trimmed'"
            return 1
        fi
    done

    return 0
}

# --- Main ---
echo "=== Step 2: Tool-coder invocation ==="

# Validate input artifact exists
if [[ ! -f "$SKILL_FILE" ]]; then
    echo "ERROR: Skill file not found at $SKILL_FILE"
    echo "  Step 1 must run first to generate this artifact."
    exit 1
fi

if [[ ! -s "$SKILL_FILE" ]]; then
    echo "ERROR: Skill file is empty at $SKILL_FILE"
    exit 1
fi

# Ensure artifacts directory exists
mkdir -p "$ARTIFACTS_DIR"

skill_content=$(cat "$SKILL_FILE")
success=false

for attempt in $(seq 1 "$MAX_RETRIES"); do
    echo "Attempt ${attempt}/${MAX_RETRIES}..."

    request_id=$(generate_uuid)
    payload=$(jq -n \
        --arg id "$request_id" \
        --arg input "$skill_content" \
        '{id: $id, input: $input, context: null, caller: "e2e-test"}')

    # Send request; capture body and HTTP status code
    http_response=$(curl -s -w '\n%{http_code}' \
        --max-time "$REQUEST_TIMEOUT" \
        -X POST \
        -H "Content-Type: application/json" \
        -d "$payload" \
        "${TOOL_CODER_URL}/invoke" 2>&1) || {
        echo "  curl failed (connection error or timeout)"
        if [[ "$attempt" -lt "$MAX_RETRIES" ]]; then
            echo "  Retrying in ${RETRY_DELAY}s..."
            sleep "$RETRY_DELAY"
        fi
        continue
    }

    # Split response body and status code
    http_status=$(echo "$http_response" | tail -n1)
    response_body=$(echo "$http_response" | sed '$d')

    if [[ "$http_status" != "200" ]]; then
        echo "  HTTP status $http_status (expected 200)"
        echo "  Response body: $response_body"
        if [[ "$attempt" -lt "$MAX_RETRIES" ]]; then
            echo "  Retrying in ${RETRY_DELAY}s..."
            sleep "$RETRY_DELAY"
        fi
        continue
    fi

    # Save response artifact
    printf '%s\n' "$response_body" > "$RESPONSE_FILE"

    if validate_response "$RESPONSE_FILE"; then
        tools_generated=$(jq -r '.output.tools_generated' "$RESPONSE_FILE")
        echo "Step 2 PASSED: tools generated and compiled successfully"
        echo "  Tools generated: $tools_generated"
        success=true
        break
    else
        echo "  Validation failed on attempt ${attempt}/${MAX_RETRIES}"
        if [[ "$attempt" -lt "$MAX_RETRIES" ]]; then
            echo "  Retrying in ${RETRY_DELAY}s..."
            sleep "$RETRY_DELAY"
        fi
    fi
done

if [[ "$success" != "true" ]]; then
    echo "Step 2 FAILED: tool-coder did not produce valid output after ${MAX_RETRIES} attempts"
    exit 1
fi
