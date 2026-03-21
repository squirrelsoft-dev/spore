#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# validate_step1_skill.sh
#
# Validates the skill-writer pipeline stage (Stage 1) by invoking the
# skill-writer agent and checking that it produces a valid skill file
# in markdown-with-frontmatter format.
#
# Usage:
#   SKILL_WRITER_URL=http://localhost:8080 ./tests/e2e/validate_step1_skill.sh
#
# Requires: curl, jq
# =============================================================================

SKILL_WRITER_URL="${SKILL_WRITER_URL:-http://skill-writer:8080}"
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ARTIFACT_DIR="${SCRIPT_DIR}/artifacts"
ARTIFACT_FILE="${ARTIFACT_DIR}/generated-skill.md"
RESPONSE_FILE="${ARTIFACT_DIR}/step1-response.json"

# Seed text from the E2E scenario
SEED_INPUT="An agent that converts temperatures between Fahrenheit, Celsius, and Kelvin"

# Helpers -------------------------------------------------------------------

fail() {
    echo "FAIL: $1" >&2
    exit 1
}

info() {
    echo "INFO: $1"
}

check_dependency() {
    local cmd="$1"
    command -v "${cmd}" > /dev/null 2>&1 \
        || fail "Required tool '${cmd}' is not installed. Please install it and retry."
}

# Extract a top-level YAML frontmatter value by key name.
# Strips surrounding quotes. Works for simple scalar values only.
get_frontmatter_value() {
    local key="$1"
    echo "${FRONTMATTER}" | grep -E "^${key}:" | head -1 \
        | sed "s/^${key}:[[:space:]]*//" | tr -d '"' | tr -d "'"
}

# Dependency checks ---------------------------------------------------------

check_dependency "curl"
check_dependency "jq"

# Step 1: Ensure artifacts directory exists ----------------------------------

mkdir -p "${ARTIFACT_DIR}"

# Step 2: Generate UUID and build AgentRequest --------------------------------

REQUEST_ID="$(cat /proc/sys/kernel/random/uuid 2>/dev/null \
    || uuidgen 2>/dev/null \
    || python3 -c 'import uuid; print(uuid.uuid4())')"

REQUEST_BODY=$(jq -n \
    --arg id "${REQUEST_ID}" \
    --arg input "${SEED_INPUT}" \
    '{id: $id, input: $input, context: null, caller: "e2e-test"}')

info "Sending POST to ${SKILL_WRITER_URL}/invoke"
info "Request body: ${REQUEST_BODY}"

HTTP_RESPONSE=$(curl -s -w "\n%{http_code}" \
    --max-time 120 \
    -X POST \
    -H "Content-Type: application/json" \
    -d "${REQUEST_BODY}" \
    "${SKILL_WRITER_URL}/invoke") || fail "curl request to ${SKILL_WRITER_URL}/invoke failed (timeout or connection error)"

HTTP_BODY=$(echo "${HTTP_RESPONSE}" | sed '$d')
HTTP_STATUS=$(echo "${HTTP_RESPONSE}" | tail -n1)

# Save raw response for debugging
echo "${HTTP_BODY}" > "${RESPONSE_FILE}"
info "Saved raw response to ${RESPONSE_FILE}"

# Step 3: Validate HTTP 200 and valid JSON AgentResponse ---------------------

if [ "${HTTP_STATUS}" != "200" ]; then
    echo "Response body: ${HTTP_BODY}" >&2
    fail "Expected HTTP 200, got ${HTTP_STATUS}"
fi
info "Received HTTP 200"

echo "${HTTP_BODY}" | jq . > /dev/null 2>&1 || fail "Response is not valid JSON"
info "Response is valid JSON"

# Validate AgentResponse required fields
for field in id output confidence escalated tool_calls; do
    echo "${HTTP_BODY}" | jq -e ".${field}" > /dev/null 2>&1 \
        || fail "AgentResponse missing required field: ${field}"
done
info "AgentResponse has all required fields (id, output, confidence, escalated, tool_calls)"

# Log validation_result if present
VALIDATION_RESULT=$(echo "${HTTP_BODY}" | jq -r '.output.validation_result // empty' 2>/dev/null || true)
if [ -n "${VALIDATION_RESULT}" ]; then
    info "Skill-writer validation_result: ${VALIDATION_RESULT}"
fi

# Step 4: Extract the skill content from output ------------------------------

SKILL_CONTENT=""

if echo "${HTTP_BODY}" | jq -e '.output.skill_yaml' > /dev/null 2>&1; then
    SKILL_CONTENT=$(echo "${HTTP_BODY}" | jq -r '.output.skill_yaml')
    info "Extracted skill content from .output.skill_yaml"
elif echo "${HTTP_BODY}" | jq -e '.output | type == "string"' > /dev/null 2>&1; then
    SKILL_CONTENT=$(echo "${HTTP_BODY}" | jq -r '.output')
    info "Extracted skill content from .output (string)"
else
    fail "Could not extract skill content from response output. Output keys: $(echo "${HTTP_BODY}" | jq -r '.output | keys[]' 2>/dev/null || echo 'N/A')"
fi

if [ -z "${SKILL_CONTENT}" ] || [ "${SKILL_CONTENT}" = "null" ]; then
    fail "Extracted skill content is empty or null"
fi
info "Skill content extracted (length: ${#SKILL_CONTENT} chars)"

# Step 5: Validate YAML frontmatter -----------------------------------------

FIRST_LINE=$(echo "${SKILL_CONTENT}" | head -n1 | tr -d '[:space:]')
if [ "${FIRST_LINE}" != "---" ]; then
    fail "Skill file does not start with YAML frontmatter delimiter (---). First line: '${FIRST_LINE}'"
fi
info "Skill file starts with --- delimiter"

# Extract frontmatter (between first and second ---) and preamble (after second ---)
FRONTMATTER=$(echo "${SKILL_CONTENT}" | awk '/^---[[:space:]]*$/{c++;next} c==1' )
PREAMBLE=$(echo "${SKILL_CONTENT}" | awk '/^---[[:space:]]*$/{c++;next} c>=2')

if [ -z "${FRONTMATTER}" ]; then
    fail "Could not extract YAML frontmatter between --- delimiters"
fi
info "YAML frontmatter extracted"

# Validate required top-level keys exist
validate_frontmatter_field() {
    local field_name="$1"
    local pattern="$2"
    if ! echo "${FRONTMATTER}" | grep -qE "${pattern}"; then
        fail "Frontmatter missing or invalid field: ${field_name}"
    fi
    info "Frontmatter field present: ${field_name}"
}

REQUIRED_FIELDS="name version description model tools constraints output"
for field in ${REQUIRED_FIELDS}; do
    validate_frontmatter_field "${field}" "^${field}:"
done

# Validate non-empty scalar fields
NAME_VALUE=$(get_frontmatter_value "name")
if [ -z "${NAME_VALUE}" ] || [ "${NAME_VALUE}" = "null" ]; then
    fail "name field is empty"
fi
info "name value: ${NAME_VALUE}"

VERSION_VALUE=$(get_frontmatter_value "version")
if [ -z "${VERSION_VALUE}" ] || [ "${VERSION_VALUE}" = "null" ]; then
    fail "version field is empty"
fi
info "version value: ${VERSION_VALUE}"

DESC_VALUE=$(get_frontmatter_value "description")
if [ -z "${DESC_VALUE}" ]; then
    fail "description field is empty"
fi
info "description present and non-empty"

# Validate model sub-fields
validate_frontmatter_field "model.provider" "[[:space:]]provider:"
validate_frontmatter_field "model.name" "[[:space:]]name:"
validate_frontmatter_field "model.temperature" "[[:space:]]temperature:"

# Validate tools list has at least one entry.
# Count list items between "tools:" and the next top-level key.
TOOL_COUNT=$(echo "${FRONTMATTER}" | awk '
    /^tools:/ { in_tools=1; next }
    in_tools && /^[a-z]/ { exit }
    in_tools && /^[[:space:]]+-/ { count++ }
    END { print count+0 }
')
if [ "${TOOL_COUNT}" -lt 1 ]; then
    fail "tools list must contain at least 1 tool, found ${TOOL_COUNT}"
fi
info "tools list has ${TOOL_COUNT} entries (at least 1 required)"

# Validate constraints sub-fields
validate_frontmatter_field "constraints.max_turns" "[[:space:]]max_turns:"
validate_frontmatter_field "constraints.confidence_threshold" "[[:space:]]confidence_threshold:"
validate_frontmatter_field "constraints.allowed_actions" "[[:space:]]allowed_actions:"

# Validate output sub-fields
validate_frontmatter_field "output.format" "[[:space:]]format:"
validate_frontmatter_field "output.schema" "[[:space:]]schema:"

# Validate output.format is one of the allowed values
FORMAT_VALUE=$(echo "${FRONTMATTER}" | grep -E "[[:space:]]+format:" | head -1 \
    | sed 's/.*format:[[:space:]]*//' | tr -d '"' | tr -d "'")
case "${FORMAT_VALUE}" in
    json|structured_json|text)
        info "output.format is valid: ${FORMAT_VALUE}"
        ;;
    *)
        fail "output.format must be one of json, structured_json, text. Got: '${FORMAT_VALUE}'"
        ;;
esac

# Step 6: Validate preamble (markdown body after closing ---) ----------------

PREAMBLE_TRIMMED=$(echo "${PREAMBLE}" | sed '/^[[:space:]]*$/d')
if [ -z "${PREAMBLE_TRIMMED}" ]; then
    fail "Preamble (markdown body after frontmatter) is empty"
fi
PREAMBLE_LENGTH=$(echo "${PREAMBLE_TRIMMED}" | wc -c | tr -d '[:space:]')
info "Preamble is non-empty (${PREAMBLE_LENGTH} chars)"

# Step 7: Save the skill file to artifacts ----------------------------------

echo "${SKILL_CONTENT}" > "${ARTIFACT_FILE}"
info "Saved generated skill to ${ARTIFACT_FILE}"

# Summary --------------------------------------------------------------------

echo ""
echo "============================================="
echo " STEP 1 VALIDATION PASSED"
echo "============================================="
echo " Skill name:     ${NAME_VALUE}"
echo " Version:        ${VERSION_VALUE}"
echo " Tools:          ${TOOL_COUNT} tool(s)"
echo " Output format:  ${FORMAT_VALUE}"
echo " Preamble:       ${PREAMBLE_LENGTH} chars"
echo " Artifact:       ${ARTIFACT_FILE}"
echo "============================================="
echo ""

exit 0
