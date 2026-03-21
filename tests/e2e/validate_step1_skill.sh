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

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib.sh"

SKILL_WRITER_URL="${SKILL_WRITER_URL:-http://skill-writer:8080}"
ARTIFACT_FILE="${ARTIFACTS_DIR}/generated-skill.md"
RESPONSE_FILE="${ARTIFACTS_DIR}/step1-response.json"

SEED_INPUT="Create an agent that converts temperatures between Celsius, Fahrenheit, and Kelvin"

# Extract a top-level YAML frontmatter value by key name.
# Strips surrounding quotes. Works for simple scalar values only.
get_frontmatter_value() {
    local key="$1"
    echo "${FRONTMATTER}" | grep -E "^${key}:" | head -1 \
        | sed "s/^${key}:[[:space:]]*//" | tr -d '"' | tr -d "'"
}

check_dependencies curl jq
mkdir -p "${ARTIFACTS_DIR}"

log_info "Sending POST to ${SKILL_WRITER_URL}/invoke"

HTTP_STATUS=$(invoke_agent "${SKILL_WRITER_URL}/invoke" "${SEED_INPUT}" "${RESPONSE_FILE}" 120) \
    || fail "curl request to ${SKILL_WRITER_URL}/invoke failed"

assert_http_ok "Step 1" "${HTTP_STATUS}" "${RESPONSE_FILE}"
jq empty "${RESPONSE_FILE}" 2>/dev/null || fail "Response is not valid JSON"
assert_agent_response_fields "Step 1" "${RESPONSE_FILE}" id output confidence escalated tool_calls
log_info "AgentResponse has all required fields"

HTTP_BODY=$(cat "${RESPONSE_FILE}")

# Log validation_result if present
VALIDATION_RESULT=$(echo "${HTTP_BODY}" | jq -r '.output.validation_result // empty' 2>/dev/null || true)
if [ -n "${VALIDATION_RESULT}" ]; then
    log_info "Skill-writer validation_result: ${VALIDATION_RESULT}"
fi

# Step 4: Extract the skill content from output ------------------------------

SKILL_CONTENT=""

if echo "${HTTP_BODY}" | jq -e '.output.skill_yaml' > /dev/null 2>&1; then
    SKILL_CONTENT=$(echo "${HTTP_BODY}" | jq -r '.output.skill_yaml')
    log_info "Extracted skill content from .output.skill_yaml"
elif echo "${HTTP_BODY}" | jq -e '.output | type == "string"' > /dev/null 2>&1; then
    SKILL_CONTENT=$(echo "${HTTP_BODY}" | jq -r '.output')
    log_info "Extracted skill content from .output (string)"
else
    fail "Could not extract skill content from response output. Output keys: $(echo "${HTTP_BODY}" | jq -r '.output | keys[]' 2>/dev/null || echo 'N/A')"
fi

if [ -z "${SKILL_CONTENT}" ] || [ "${SKILL_CONTENT}" = "null" ]; then
    fail "Extracted skill content is empty or null"
fi
log_info "Skill content extracted (length: ${#SKILL_CONTENT} chars)"

# Step 5: Validate YAML frontmatter -----------------------------------------

FIRST_LINE=$(echo "${SKILL_CONTENT}" | head -n1 | tr -d '[:space:]')
if [ "${FIRST_LINE}" != "---" ]; then
    fail "Skill file does not start with YAML frontmatter delimiter (---). First line: '${FIRST_LINE}'"
fi
log_info "Skill file starts with --- delimiter"

# Extract frontmatter (between first and second ---) and preamble (after second ---)
FRONTMATTER=$(echo "${SKILL_CONTENT}" | awk '/^---[[:space:]]*$/{c++;next} c==1' )
PREAMBLE=$(echo "${SKILL_CONTENT}" | awk '/^---[[:space:]]*$/{c++;next} c>=2')

if [ -z "${FRONTMATTER}" ]; then
    fail "Could not extract YAML frontmatter between --- delimiters"
fi
log_info "YAML frontmatter extracted"

# Validate required top-level keys exist
validate_frontmatter_field() {
    local field_name="$1"
    local pattern="$2"
    if ! echo "${FRONTMATTER}" | grep -qE "${pattern}"; then
        fail "Frontmatter missing or invalid field: ${field_name}"
    fi
    log_info "Frontmatter field present: ${field_name}"
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
log_info "name value: ${NAME_VALUE}"

VERSION_VALUE=$(get_frontmatter_value "version")
if [ -z "${VERSION_VALUE}" ] || [ "${VERSION_VALUE}" = "null" ]; then
    fail "version field is empty"
fi
log_info "version value: ${VERSION_VALUE}"

DESC_VALUE=$(get_frontmatter_value "description")
if [ -z "${DESC_VALUE}" ]; then
    fail "description field is empty"
fi
log_info "description present and non-empty"

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
log_info "tools list has ${TOOL_COUNT} entries (at least 1 required)"

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
        log_info "output.format is valid: ${FORMAT_VALUE}"
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
log_info "Preamble is non-empty (${PREAMBLE_LENGTH} chars)"

# Step 7: Save the skill file to artifacts ----------------------------------

echo "${SKILL_CONTENT}" > "${ARTIFACT_FILE}"
log_info "Saved generated skill to ${ARTIFACT_FILE}"

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
