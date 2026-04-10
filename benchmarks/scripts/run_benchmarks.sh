#!/usr/bin/env bash
# Run the benchmark suite.
#
# Usage: ./run_benchmarks.sh [--tasks bigquery_overlap|spanner_overlap|dcx_differentiated] [--trials N]
#
# Reads task specs from tasks/*.yaml, resolves placeholders from manifest.yaml,
# executes each CLI variant, and writes raw results to results/raw/.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BENCH_DIR="${SCRIPT_DIR}/.."
MANIFEST="${BENCH_DIR}/manifest.yaml"
RESULTS_DIR="${BENCH_DIR}/results/raw"
COLD_TRIALS=3
WARM_TRIALS=10
TASK_FILTER=""

# ── Parse arguments ──────────────────────────────────────────────────
while [[ $# -gt 0 ]]; do
  case "$1" in
    --tasks) TASK_FILTER="$2"; shift 2 ;;
    --trials) WARM_TRIALS="$2"; shift 2 ;;
    --cold-trials) COLD_TRIALS="$2"; shift 2 ;;
    *) echo "Unknown argument: $1"; exit 1 ;;
  esac
done

# ── Check dependencies ───────────────────────────────────────────────
for cmd in python3 yq dcx bq gcloud; do
  if ! command -v "${cmd}" &>/dev/null; then
    echo "ERROR: ${cmd} not found in PATH" >&2
    exit 1
  fi
done

# ── Environment freeze ───────────────────────────────────────────────
RUN_ID="$(date +%Y%m%d-%H%M%S)-$(git rev-parse --short HEAD 2>/dev/null || echo 'unknown')"
RUN_DIR="${RESULTS_DIR}/${RUN_ID}"
mkdir -p "${RUN_DIR}"

cat > "${RUN_DIR}/environment.json" <<ENVEOF
{
  "run_id": "${RUN_ID}",
  "timestamp": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "git_sha": "$(git rev-parse HEAD 2>/dev/null || echo 'unknown')",
  "dcx_version": "$(dcx --version 2>/dev/null || echo 'unknown')",
  "bq_version": "$(bq version 2>&1 | head -1 || echo 'unknown')",
  "gcloud_version": "$(gcloud version 2>&1 | head -1 || echo 'unknown')",
  "os": "$(uname -s) $(uname -r)",
  "cpu": "$(uname -m)",
  "project": "$(yq -r '.project' "${MANIFEST}")",
  "region": "$(yq -r '.region' "${MANIFEST}")",
  "auth_source": "$(yq -r '.auth_source' "${MANIFEST}")"
}
ENVEOF

echo "==> Benchmark run: ${RUN_ID}"
echo "==> Results dir: ${RUN_DIR}"

# ── Load manifest bindings ───────────────────────────────────────────
# Build a sed expression to substitute all {placeholder} tokens.
resolve_placeholders() {
  local cmd="$1"
  # Read each key-value pair from manifest and substitute.
  while IFS=': ' read -r key value; do
    # Skip comments and empty lines.
    [[ -z "${key}" || "${key}" == \#* ]] && continue
    # Strip quotes from value.
    value="${value%\"}"
    value="${value#\"}"
    cmd="${cmd//\{${key}\}/${value}}"
  done < "${MANIFEST}"
  echo "${cmd}"
}

# ── Run a single command and capture metrics ─────────────────────────
run_trial() {
  local task_id="$1"
  local cli_name="$2"
  local command="$3"
  local trial_num="$4"
  local trial_type="$5"  # cold or warm

  local resolved
  resolved="$(resolve_placeholders "${command}")"

  local start_ns end_ns exit_code stdout_file stderr_file
  stdout_file="${RUN_DIR}/${task_id}.${cli_name}.${trial_type}${trial_num}.stdout"
  stderr_file="${RUN_DIR}/${task_id}.${cli_name}.${trial_type}${trial_num}.stderr"

  start_ns="$(python3 -c 'import time; print(int(time.time()*1000))')"
  eval "${resolved}" >"${stdout_file}" 2>"${stderr_file}" || true
  exit_code=$?
  end_ns="$(python3 -c 'import time; print(int(time.time()*1000))')"

  local wall_clock_ms=$(( end_ns - start_ns ))
  local stdout_bytes stderr_bytes
  stdout_bytes="$(wc -c < "${stdout_file}" | tr -d ' ')"
  stderr_bytes="$(wc -c < "${stderr_file}" | tr -d ' ')"

  # Append result row as NDJSON.
  printf '{"task_id":"%s","cli":"%s","trial":%d,"trial_type":"%s","exit_code":%d,"wall_clock_ms":%d,"stdout_bytes":%d,"stderr_bytes":%d}\n' \
    "${task_id}" "${cli_name}" "${trial_num}" "${trial_type}" \
    "${exit_code}" "${wall_clock_ms}" "${stdout_bytes}" "${stderr_bytes}" \
    >> "${RUN_DIR}/results.ndjson"
}

# ── Process task files ───────────────────────────────────────────────
TASK_FILES=("${BENCH_DIR}"/tasks/*.yaml)
if [ -n "${TASK_FILTER}" ]; then
  TASK_FILES=("${BENCH_DIR}/tasks/${TASK_FILTER}.yaml")
fi

TOTAL_TASKS=0
for task_file in "${TASK_FILES[@]}"; do
  [ -f "${task_file}" ] || continue
  TASK_COUNT="$(yq '.tasks | length' "${task_file}")"
  TOTAL_TASKS=$((TOTAL_TASKS + TASK_COUNT))

  for ((idx = 0; idx < TASK_COUNT; idx++)); do
    TASK_ID="$(yq -r ".tasks[${idx}].id" "${task_file}")"
    GOAL="$(yq -r ".tasks[${idx}].goal" "${task_file}")"
    VARIANT_COUNT="$(yq ".tasks[${idx}].cli_variants | length" "${task_file}")"

    echo ""
    echo "── Task: ${TASK_ID} — ${GOAL}"

    for ((vidx = 0; vidx < VARIANT_COUNT; vidx++)); do
      CLI_NAME="$(yq -r ".tasks[${idx}].cli_variants[${vidx}].name" "${task_file}")"
      CLI_CMD="$(yq -r ".tasks[${idx}].cli_variants[${vidx}].command" "${task_file}")"

      echo "   ${CLI_NAME}: ${COLD_TRIALS} cold + ${WARM_TRIALS} warm trials"

      # Cold trials.
      for ((t = 1; t <= COLD_TRIALS; t++)); do
        run_trial "${TASK_ID}" "${CLI_NAME}" "${CLI_CMD}" "${t}" "cold"
      done

      # Warm trials.
      for ((t = 1; t <= WARM_TRIALS; t++)); do
        run_trial "${TASK_ID}" "${CLI_NAME}" "${CLI_CMD}" "${t}" "warm"
      done
    done
  done
done

echo ""
echo "==> Completed ${TOTAL_TASKS} tasks. Results in ${RUN_DIR}/results.ndjson"
echo "==> Run score_results.py to generate scorecards."
