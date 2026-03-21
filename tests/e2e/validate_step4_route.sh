#!/usr/bin/env bash
set -euo pipefail

###############################################################################
# Step 4 Validator: Orchestrator Routing
#
# Sends two temperature-conversion queries to the orchestrator and validates
# that the responses are valid AgentResponse objects with reasonable numeric
# answers, sufficient confidence, and non-empty tool_calls.
###############################################################################

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "${SCRIPT_DIR}/lib.sh"

ORCHESTRATOR_URL="${ORCHESTRATOR_URL:-http://orchestrator:8080}"
mkdir -p "${ARTIFACTS_DIR}"

INVOKE_URL="${ORCHESTRATOR_URL}/invoke"

# float_in_range VALUE LOW HIGH
# Returns 0 (true) if LOW <= VALUE <= HIGH, 1 otherwise.
float_in_range() {
  local value="$1" low="$2" high="$3"
  awk -v v="$value" -v lo="$low" -v hi="$high" \
    'BEGIN { exit !(v+0 >= lo+0 && v+0 <= hi+0) }'
}

# extract_number_from_output ARTIFACT_FILE
# Attempts structured field extraction first, then falls back to regex.
# Prints the extracted number to stdout. Returns non-zero if none found.
extract_number_from_output() {
  local artifact="$1"
  local val

  # Try direct numeric output
  val=$(jq -e '.output | select(type == "number")' "${artifact}" 2>/dev/null) && { echo "${val}"; return 0; }

  # Try common structured object fields
  for field in result value answer temperature celsius; do
    val=$(jq -e ".output.${field} | select(type == \"number\")" "${artifact}" 2>/dev/null) && { echo "${val}"; return 0; }
  done

  # Fall back to regex extraction from stringified output
  local output_str
  output_str=$(jq -r '.output | if type == "object" or type == "array" then tostring else . end' "${artifact}")
  val=$(echo "${output_str}" | grep -oE -- '-?[0-9]+\.?[0-9]*' | head -n1)

  if [ -n "${val}" ]; then
    echo "${val}"
    return 0
  fi

  return 1
}

assert_confidence() {
  local label="$1" artifact="$2" threshold="$3"
  local confidence
  confidence=$(jq '.confidence' "${artifact}")
  if ! float_in_range "${confidence}" "${threshold}" 1.0; then
    fail "${label}: confidence ${confidence} is below ${threshold}"
  fi
  log_pass "${label}: confidence ${confidence} >= ${threshold}"
}

assert_tool_calls_nonempty() {
  local label="$1" artifact="$2"
  local tool_calls_len
  tool_calls_len=$(jq '.tool_calls | length' "${artifact}")
  if [ "${tool_calls_len}" -le 0 ]; then
    fail "${label}: tool_calls is empty"
  fi
  log_pass "${label}: tool_calls has ${tool_calls_len} entries"
}

assert_in_range() {
  local label="$1" actual="$2" low="$3" high="$4"
  if ! float_in_range "${actual}" "${low}" "${high}"; then
    fail "${label}: numeric answer ${actual} is outside [${low}, ${high}]"
  fi
  log_pass "${label}: numeric answer ${actual} is within [${low}, ${high}]"
}

# send_query QUERY_TEXT ARTIFACT_FILE
# POSTs an AgentRequest to the orchestrator and saves the response.
# Prints the HTTP status code to stdout.
send_query() {
  local query="$1"
  local artifact="$2"
  local id
  id="$(generate_uuid)"

  local payload
  payload=$(jq -n \
    --arg id "$id" \
    --arg input "$query" \
    --arg caller "$E2E_CALLER" \
    '{id: $id, input: $input, context: null, caller: $caller}')

  local http_code
  http_code=$(curl -s -o "${artifact}" -w '%{http_code}' \
    -X POST \
    -H 'Content-Type: application/json' \
    -d "${payload}" \
    --max-time 60 \
    "${INVOKE_URL}")

  echo "${http_code}"
}

# validate_response LABEL ARTIFACT_FILE EXPECTED_LOW EXPECTED_HIGH
# Validates a saved AgentResponse JSON file. Exits on first failure.
validate_response() {
  local label="$1"
  local artifact="$2"
  local expected_low="$3"
  local expected_high="$4"

  log_info "${label}: validating response from ${artifact}"

  if [ ! -s "${artifact}" ]; then
    fail "${label}: response file is empty or missing"
  fi

  if ! jq empty "${artifact}" 2>/dev/null; then
    cat "${artifact}" >&2
    fail "${label}: response is not valid JSON"
  fi

  # Validate required AgentResponse fields
  for field in id output confidence tool_calls; do
    if [ "$(jq "has(\"${field}\")" "${artifact}")" != "true" ]; then
      fail "${label}: missing '${field}' field"
    fi
  done
  log_pass "${label}: response has valid AgentResponse structure"

  assert_confidence "${label}" "${artifact}" 0.8
  assert_tool_calls_nonempty "${label}" "${artifact}"

  local numeric_value
  if ! numeric_value=$(extract_number_from_output "${artifact}"); then
    local output_raw
    output_raw=$(jq -r '.output' "${artifact}")
    fail "${label}: could not extract a numeric value from output: ${output_raw}"
  fi

  assert_in_range "${label}" "${numeric_value}" "${expected_low}" "${expected_high}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

log_info "Orchestrator invoke URL: ${INVOKE_URL}"
echo ""

# Query 1: 100 F -> C  (expect ~37.78, accept 37.0-39.0)
Q1_ARTIFACT="${ARTIFACTS_DIR}/step4-response-q1.json"
log_info "Query 1: Convert 100 degrees Fahrenheit to Celsius"
HTTP1=$(send_query "Convert 100 degrees Fahrenheit to Celsius" "${Q1_ARTIFACT}")
log_info "Query 1: HTTP status ${HTTP1}"
assert_http_ok "Query 1" "${HTTP1}" "${Q1_ARTIFACT}"
validate_response "Query 1" "${Q1_ARTIFACT}" 37.0 39.0
log_pass "PASS: Query 1 (100F -> C)"

echo ""

# Query 2: 0 K -> C  (expect ~-273.15, accept -274.0 to -273.0)
Q2_ARTIFACT="${ARTIFACTS_DIR}/step4-response-q2.json"
log_info "Query 2: Convert 0 Kelvin to Celsius"
HTTP2=$(send_query "Convert 0 Kelvin to Celsius" "${Q2_ARTIFACT}")
log_info "Query 2: HTTP status ${HTTP2}"
assert_http_ok "Query 2" "${HTTP2}" "${Q2_ARTIFACT}"
validate_response "Query 2" "${Q2_ARTIFACT}" -274.0 -273.0
log_pass "PASS: Query 2 (0K -> C)"

echo ""

# ---------------------------------------------------------------------------
# Summary
# ---------------------------------------------------------------------------

# Save combined artifact for downstream consumption
jq -n \
  --slurpfile q1 "${Q1_ARTIFACT}" \
  --slurpfile q2 "${Q2_ARTIFACT}" \
  '{query1: $q1[0], query2: $q2[0]}' \
  > "${ARTIFACTS_DIR}/step4-response.json"

echo "Step 4: All routing validations passed"
exit 0
