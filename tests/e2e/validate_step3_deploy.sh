#!/usr/bin/env bash
set -euo pipefail

# Step 3 Validator: deploy-agent invocation
# Sends combined artifacts from steps 1 & 2 to the deploy-agent,
# then validates the structured JSON response.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib.sh"

DEPLOY_AGENT_URL="${DEPLOY_AGENT_URL:-http://deploy-agent:8080}"
INVOKE_URL="${DEPLOY_AGENT_URL}/invoke"
CONNECT_TIMEOUT=30
MAX_TIME=600

check_step1_artifact() {
    local skill_file="${ARTIFACTS_DIR}/generated-skill.md"
    if [[ ! -f "$skill_file" ]]; then
        echo "Step 3 FAILED: generated-skill.md not found -- step 1 must complete first"
        exit 1
    fi
    SKILL_PATH="$skill_file"
    log_info "Loaded step 1 artifact: skill_path=${SKILL_PATH}"
}

check_step2_artifact() {
    local step2_response="${ARTIFACTS_DIR}/step2-response.json"
    if [[ ! -f "$step2_response" ]]; then
        echo "Step 3 FAILED: step2-response.json not found -- step 2 must complete first"
        exit 1
    fi
    TOOL_PATHS="$(jq -r '.output.implementation_paths // [] | join(", ")' "$step2_response")"
    log_info "Loaded step 2 artifact: tool_paths=${TOOL_PATHS}"
}

build_request_payload() {
    local request_id
    request_id="$(generate_uuid)"
    REQUEST_PAYLOAD="$(jq -n \
        --arg id "$request_id" \
        --arg skill_path "$SKILL_PATH" \
        --arg tool_paths "$TOOL_PATHS" \
        --arg caller "$E2E_CALLER" \
        '{
            id: $id,
            input: ("Deploy the agent defined by skill file at " + $skill_path + " with tools at " + $tool_paths),
            context: {
                skill_path: $skill_path,
                tool_paths: $tool_paths
            },
            caller: $caller
        }'
    )"
    log_info "Built request payload with id=${request_id}"
}

send_invoke_request() {
    log_info "Sending POST to ${INVOKE_URL} (timeout: ${MAX_TIME}s for Docker builds)"
    HTTP_CODE=$(curl -s -o "${ARTIFACTS_DIR}/step3-response.json" \
        -w "%{http_code}" \
        -X POST \
        -H "Content-Type: application/json" \
        -d "$REQUEST_PAYLOAD" \
        --max-time "${MAX_TIME}" \
        --connect-timeout "${CONNECT_TIMEOUT}" \
        "$INVOKE_URL")
    if [[ "$HTTP_CODE" != "200" ]]; then
        echo "Step 3 FAILED: deploy-agent returned HTTP ${HTTP_CODE}"
        cat "${ARTIFACTS_DIR}/step3-response.json" >&2 || true
        exit 1
    fi
    log_info "Received HTTP ${HTTP_CODE}"
}

validate_image_uri() {
    local image_uri
    image_uri="$(jq -r '.output.image_uri // empty' "${ARTIFACTS_DIR}/step3-response.json")"
    if [[ -z "$image_uri" ]]; then
        echo "Step 3 FAILED: output.image_uri is missing or empty"
        return 1
    fi
    if [[ ! "$image_uri" =~ spore-[a-z][a-z0-9-]*:[0-9]+(\.[0-9]+)* ]]; then
        echo "Step 3 FAILED: output.image_uri does not match spore-{name}:{version} pattern (got: ${image_uri})"
        return 1
    fi
    IMAGE_URI="$image_uri"
    log_info "image_uri is valid: ${image_uri}"
}

validate_endpoint_url() {
    local endpoint_url
    endpoint_url="$(jq -r '.output.endpoint_url // empty' "${ARTIFACTS_DIR}/step3-response.json")"
    if [[ -z "$endpoint_url" ]]; then
        echo "Step 3 FAILED: output.endpoint_url is missing or empty"
        return 1
    fi
    if [[ "$endpoint_url" != http://* && "$endpoint_url" != https://* ]]; then
        echo "Step 3 FAILED: output.endpoint_url is not a valid HTTP URL (got: ${endpoint_url})"
        return 1
    fi
    ENDPOINT_URL="$endpoint_url"
    log_info "endpoint_url is valid: ${endpoint_url}"
}

validate_health_check() {
    local health_check
    health_check="$(jq -r '.output.health_check // empty' "${ARTIFACTS_DIR}/step3-response.json")"
    if [[ "$health_check" != "healthy" ]]; then
        echo "Step 3 FAILED: output.health_check expected 'healthy' (got: '${health_check}')"
        return 1
    fi
    log_info "health_check is valid: ${health_check}"
}

validate_output_structure() {
    local output_type
    output_type="$(jq -r '.output | type' "${ARTIFACTS_DIR}/step3-response.json")"
    if [[ "$output_type" != "object" ]]; then
        echo "Step 3 FAILED: response .output is not a JSON object (got: ${output_type})"
        exit 1
    fi
}

validate_response() {
    validate_output_structure
    local failures=0
    validate_image_uri || failures=$((failures + 1))
    validate_endpoint_url || failures=$((failures + 1))
    validate_health_check || failures=$((failures + 1))
    if [[ "$failures" -gt 0 ]]; then
        echo "Step 3 FAILED: ${failures} validation(s) failed"
        jq '.' "${ARTIFACTS_DIR}/step3-response.json" >&2 || true
        exit 1
    fi
}

main() {
    log_info "Starting step 3 validation against ${INVOKE_URL}"
    check_dependencies curl jq
    check_step1_artifact
    check_step2_artifact
    build_request_payload
    send_invoke_request
    validate_response
    echo "Step 3 PASSED: deploy-agent produced image_uri=${IMAGE_URI}, endpoint_url=${ENDPOINT_URL}, health_check=healthy"
    log_info "Response saved to ${ARTIFACTS_DIR}/step3-response.json"
}

main "$@"
