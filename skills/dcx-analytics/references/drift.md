# Drift Detection

Drift detection compares agent metrics across two time windows: a **baseline**
period and a **current** period.

## Workflow

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

## Notes

- Drift detection is a workflow pattern, not a standalone command
- Comparison logic runs outside dcx (script, CI pipeline, etc.)
- Time window granularity depends on event density in the table
