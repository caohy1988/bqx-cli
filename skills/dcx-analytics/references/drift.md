# Drift Detection

## The `drift` command

Compare recent agent sessions against a golden question set to detect
coverage regressions.

```bash
dcx analytics drift \
  --golden-dataset <table_name> \
  [--last <duration>] \
  [--agent-id <name>] \
  [--min-coverage <ratio>] \
  [--exit-code] \
  [--format json|json-minified|table|text]
```

### Required flags

| Flag | Description |
|------|-------------|
| `--golden-dataset` | BigQuery table with `question` and `expected_answer` columns |

### Optional flags

| Flag | Default | Description |
|------|---------|-------------|
| `--last` | `7d` | Time window for recent sessions |
| `--agent-id` | — | Filter to a specific agent |
| `--min-coverage` | `0.8` | Minimum coverage ratio (0–1) to pass |
| `--exit-code` | — | Return exit code 1 if coverage < min-coverage |
| `--format` | `json` | Output format |

### Output

**JSON** includes: `golden_dataset`, `time_window`, `total_golden`, `covered`,
`uncovered`, `coverage`, `min_coverage`, `passed`, and a `questions` array
with per-question match status.

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Coverage meets or exceeds min-coverage |
| 1 | Coverage below min-coverage (requires `--exit-code`) |
| 2 | Infrastructure error |

### Golden dataset schema

The golden dataset table must have two STRING columns:

```sql
CREATE TABLE `project.dataset.golden_questions` (
  question STRING,
  expected_answer STRING
);
```

### Example

```bash
dcx analytics drift \
  --golden-dataset golden_questions \
  --last 7d \
  --min-coverage 0.8 \
  --exit-code \
  --project-id PROJECT --dataset-id DATASET
```

## Evaluate-based drift monitoring

You can also detect drift by comparing evaluator results across time windows:

### Latency drift

```bash
# Baseline
dcx analytics evaluate --evaluator latency --threshold 5000 --last 7d \
  --project-id PROJECT --dataset-id DATASET --format json > baseline.json

# Current
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id PROJECT --dataset-id DATASET --format json > current.json
```

### Error rate drift

```bash
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 7d \
  --project-id PROJECT --dataset-id DATASET --format json > baseline_errors.json

dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 24h \
  --project-id PROJECT --dataset-id DATASET --format json > current_errors.json
```

## Thresholds

- Use 7d baseline vs 24h current for daily drift checks
- Use 30d baseline vs 7d current for weekly drift reports
- A pass rate drop >10% warrants investigation
- Use `--agent-id` to isolate drift to a specific agent
