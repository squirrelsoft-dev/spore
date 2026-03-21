#!/usr/bin/env bash
# Shared helpers for E2E validator scripts.
# Source this file: source "$(dirname "${BASH_SOURCE[0]}")/lib.sh"

ARTIFACTS_DIR="${ARTIFACTS_DIR:-$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/artifacts}"
E2E_CALLER="e2e-test"

fail() {
    echo "FAIL: $*" >&2
    exit 1
}

log_info() {
    echo "INFO: $*"
}

log_pass() {
    echo "PASS: $*"
}

generate_uuid() {
    if command -v uuidgen &>/dev/null; then
        uuidgen | tr '[:upper:]' '[:lower:]'
    elif [[ -f /proc/sys/kernel/random/uuid ]]; then
        cat /proc/sys/kernel/random/uuid
    else
        python3 -c 'import uuid; print(uuid.uuid4())'
    fi
}

check_dependencies() {
    for cmd in "$@"; do
        if ! command -v "$cmd" &>/dev/null; then
            fail "Required tool '${cmd}' is not installed"
        fi
    done
}

# invoke_agent URL INPUT OUTPUT_FILE [MAX_TIME]
# POSTs an AgentRequest to the given URL, saves the response body to
# OUTPUT_FILE, and prints the HTTP status code to stdout.
invoke_agent() {
    local url="$1"
    local input="$2"
    local output_file="$3"
    local max_time="${4:-120}"
    local request_id
    request_id="$(generate_uuid)"

    local payload
    payload=$(jq -n \
        --arg id "$request_id" \
        --arg input "$input" \
        --arg caller "$E2E_CALLER" \
        '{id: $id, input: $input, context: null, caller: $caller}')

    curl -s -o "$output_file" -w '%{http_code}' \
        -X POST \
        -H "Content-Type: application/json" \
        -d "$payload" \
        --max-time "$max_time" \
        "$url"
}

assert_http_ok() {
    local label="$1" status="$2" artifact="$3"
    if [[ "$status" != "200" ]]; then
        echo "--- Response body ---" >&2
        cat "$artifact" >&2 || true
        echo "" >&2
        fail "${label}: expected HTTP 200, got ${status}"
    fi
    log_pass "${label}: HTTP 200 OK"
}

assert_agent_response_fields() {
    local label="$1" artifact="$2"
    shift 2
    local fields=("$@")
    for field in "${fields[@]}"; do
        if [[ "$(jq "has(\"${field}\")" "$artifact")" != "true" ]]; then
            fail "${label}: missing required field '${field}'"
        fi
    done
}
