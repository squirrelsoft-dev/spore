#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# e2e-test.sh -- Top-level E2E test driver for the Spore agent platform.
#
# Builds and starts all four services via docker compose, waits for health,
# then runs the four validation steps sequentially. On failure it captures
# container logs and exits non-zero.
#
# Usage:
#   ./scripts/e2e-test.sh [--no-cleanup] [--timeout N]
# =============================================================================

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
COMPOSE_FILE="${REPO_ROOT}/docker-compose.e2e.yml"
ARTIFACTS_DIR="${REPO_ROOT}/tests/e2e/artifacts"

NO_CLEANUP=false
TIMEOUT=600
START_TIME=""

SERVICES=("skill-writer" "tool-coder" "deploy-agent" "orchestrator")
PORTS=(8081 8082 8083 8084)

STEP_NAMES=(
    "validate_step1_skill"
    "validate_step2_tools"
    "validate_step3_deploy"
    "validate_step4_route"
)

declare -a STEP_TIMES=()
declare -a STEP_RESULTS=()

# -- Argument parsing --------------------------------------------------------

parse_arguments() {
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --no-cleanup)
                NO_CLEANUP=true
                shift
                ;;
            --timeout)
                if [[ -z "${2:-}" ]]; then
                    echo "ERROR: --timeout requires a positive integer argument" >&2
                    exit 1
                fi
                if ! [[ "$2" =~ ^[1-9][0-9]*$ ]]; then
                    echo "ERROR: --timeout requires a positive integer, got: $2" >&2
                    exit 1
                fi
                TIMEOUT="$2"
                shift 2
                ;;
            *)
                echo "ERROR: Unknown argument: $1" >&2
                echo "Usage: $0 [--no-cleanup] [--timeout N]" >&2
                exit 1
                ;;
        esac
    done
}

# -- Time helpers ------------------------------------------------------------

current_epoch() {
    date +%s
}

elapsed_since_start() {
    local now
    now="$(current_epoch)"
    echo $(( now - START_TIME ))
}

check_timeout() {
    local elapsed
    elapsed="$(elapsed_since_start)"
    if [[ "$elapsed" -ge "$TIMEOUT" ]]; then
        echo "ERROR: Timeout of ${TIMEOUT}s exceeded (elapsed: ${elapsed}s)" >&2
        dump_logs_to_artifacts
        exit 1
    fi
}

# -- Prerequisites -----------------------------------------------------------

check_prerequisites() {
    local missing=false
    for tool in curl jq docker; do
        if ! command -v "$tool" &>/dev/null; then
            echo "ERROR: ${tool} is not installed or not in PATH" >&2
            missing=true
        fi
    done
    if ! docker compose version &>/dev/null; then
        echo "ERROR: docker compose plugin is not available" >&2
        missing=true
    fi
    if [[ "$missing" == "true" ]]; then
        exit 1
    fi
}

# -- Docker compose helpers --------------------------------------------------

compose_up() {
    echo "Starting services via docker compose..."
    docker compose -f "${COMPOSE_FILE}" up -d --build
}

compose_down() {
    echo "Tearing down services..."
    docker compose -f "${COMPOSE_FILE}" down -v --remove-orphans
}

# -- Cleanup trap ------------------------------------------------------------

cleanup() {
    if [[ "${NO_CLEANUP}" == "true" ]]; then
        echo "Skipping cleanup (--no-cleanup set). Services are still running."
        return
    fi
    compose_down || true
}

# -- Health polling ----------------------------------------------------------

poll_single_service() {
    local name="$1"
    local port="$2"
    local url="http://localhost:${port}/health"
    local delay=1
    local max_delay=16
    local total_waited=0
    local max_wait=60

    while [[ "$total_waited" -lt "$max_wait" ]]; do
        check_timeout
        local response=""
        response="$(curl -sf --max-time 5 "${url}" 2>/dev/null)" || true
        if [[ -n "$response" ]]; then
            local status=""
            status="$(echo "$response" | jq -r '.status' 2>/dev/null)" || true
            if [[ "$status" == "Healthy" ]]; then
                echo "  ${name} (port ${port}) is healthy"
                return 0
            elif [[ "$status" == *"Degraded"* ]]; then
                echo "  WARNING: ${name} is degraded, continuing to wait..." >&2
            fi
        fi
        sleep "$delay"
        total_waited=$(( total_waited + delay ))
        delay=$(( delay * 2 ))
        if [[ "$delay" -gt "$max_delay" ]]; then
            delay="$max_delay"
        fi
    done

    echo "ERROR: ${name} did not become healthy within ${max_wait}s" >&2
    return 1
}

wait_for_all_services() {
    echo "Waiting for services to become healthy..."
    local i
    for i in "${!SERVICES[@]}"; do
        if ! poll_single_service "${SERVICES[$i]}" "${PORTS[$i]}"; then
            echo "ERROR: Service ${SERVICES[$i]} failed health check" >&2
            dump_logs_to_artifacts
            exit 1
        fi
    done
    echo "All services healthy."
}

# -- Log capture -------------------------------------------------------------

dump_logs_to_artifacts() {
    mkdir -p "${ARTIFACTS_DIR}"
    local logfile="${ARTIFACTS_DIR}/docker-compose-logs.txt"
    echo "Dumping container logs to ${logfile}"
    docker compose -f "${COMPOSE_FILE}" logs --no-color > "${logfile}" 2>&1 || true
}

# -- Step execution ----------------------------------------------------------

run_step() {
    local index="$1"
    local script_name="${STEP_NAMES[$index]}"
    local script_path="${REPO_ROOT}/tests/e2e/${script_name}.sh"
    local step_num=$(( index + 1 ))

    echo "--- Step ${step_num}: ${script_name} ---"
    check_timeout

    if [[ ! -x "${script_path}" ]]; then
        echo "ERROR: Validator not found or not executable: ${script_path}" >&2
        return 1
    fi

    local step_start
    step_start="$(current_epoch)"

    set +e
    (
        export SKILL_WRITER_URL="http://localhost:${PORTS[0]}"
        export TOOL_CODER_URL="http://localhost:${PORTS[1]}"
        export DEPLOY_AGENT_HOST="localhost:${PORTS[2]}"
        export ORCHESTRATOR_URL="http://localhost:${PORTS[3]}"
        export ARTIFACTS_DIR="${ARTIFACTS_DIR}"
        bash "${script_path}"
    )
    local exit_code=$?
    set -e

    local step_end
    step_end="$(current_epoch)"
    local step_elapsed=$(( step_end - step_start ))
    STEP_TIMES+=("${step_elapsed}")

    if [[ "$exit_code" -ne 0 ]]; then
        STEP_RESULTS+=("FAIL")
        handle_step_failure "$step_num" "$script_name" "$exit_code"
        return 1
    fi

    STEP_RESULTS+=("PASS")
    echo "Step ${step_num} passed (${step_elapsed}s)"
    return 0
}

handle_step_failure() {
    local step_num="$1"
    local script_name="$2"
    local exit_code="$3"

    echo "ERROR: Step ${step_num} (${script_name}) failed with exit code ${exit_code}" >&2
    dump_logs_to_artifacts
}

run_all_steps() {
    local i
    for i in "${!STEP_NAMES[@]}"; do
        run_step "$i"
    done
}

# -- Summary -----------------------------------------------------------------

print_summary() {
    local total_elapsed
    total_elapsed="$(elapsed_since_start)"
    local all_passed=true

    echo ""
    echo "========================================"
    echo "  E2E Test Summary"
    echo "========================================"
    local i
    for i in "${!STEP_NAMES[@]}"; do
        local result="${STEP_RESULTS[$i]:-SKIP}"
        local elapsed="${STEP_TIMES[$i]:-0}"
        printf "  Step %d %-30s %s (%ss)\n" \
            $(( i + 1 )) "${STEP_NAMES[$i]}" "$result" "$elapsed"
        if [[ "$result" != "PASS" ]]; then
            all_passed=false
        fi
    done
    echo "----------------------------------------"
    printf "  Total time: %ss\n" "$total_elapsed"
    if [[ "$all_passed" == "true" ]]; then
        echo "  Result: PASS"
    else
        echo "  Result: FAIL"
    fi
    echo "========================================"

    if [[ "$all_passed" != "true" ]]; then
        return 1
    fi
}

# -- Main --------------------------------------------------------------------

main() {
    parse_arguments "$@"
    START_TIME="$(current_epoch)"

    echo "E2E test run started (timeout: ${TIMEOUT}s)"

    check_prerequisites
    mkdir -p "${ARTIFACTS_DIR}"

    trap cleanup EXIT

    compose_up
    wait_for_all_services
    run_all_steps
    print_summary
}

main "$@"
