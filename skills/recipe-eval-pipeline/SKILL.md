---
name: recipe-eval-pipeline
description: Recipe for setting up a CI/CD evaluation gate that blocks deployments when agent quality drops below thresholds.
---

## When to use this skill

Use when the user wants to:
- Set up automated quality gates in CI/CD
- Block deployments when agent performance degrades
- Add evaluation checks to GitHub Actions, GitLab CI, or similar

## Prerequisites

Load the `bqx-analytics` skill.

See **bqx-shared** for authentication and global flags.

## Recipe

### Step 1: Define quality thresholds

Choose thresholds for your agent:

| Metric | Evaluator | Typical threshold |
|--------|-----------|-------------------|
| Latency | `latency` | 3000–5000 ms |
| Error rate | `error-rate` | 0.05–0.10 (5–10%) |

### Step 2: Add evaluation commands to CI

#### GitHub Actions example

```yaml
jobs:
  agent-quality-gate:
    runs-on: ubuntu-latest
    steps:
      - name: Install bqx
        run: npm install -g bqx

      - name: Check latency
        run: |
          bqx analytics evaluate \
            --project-id ${{ secrets.GCP_PROJECT }} \
            --dataset-id ${{ secrets.BQX_DATASET }} \
            --evaluator latency \
            --threshold 5000 \
            --last 24h \
            --exit-code \
            --format json

      - name: Check error rate
        run: |
          bqx analytics evaluate \
            --project-id ${{ secrets.GCP_PROJECT }} \
            --dataset-id ${{ secrets.BQX_DATASET }} \
            --evaluator error-rate \
            --threshold 0.05 \
            --last 24h \
            --exit-code \
            --format json
```

#### Shell script example

```bash
#!/usr/bin/env bash
set -euo pipefail

PROJECT_ID="${1:?Usage: eval-gate.sh <project-id> <dataset-id>}"
DATASET_ID="${2:?Usage: eval-gate.sh <project-id> <dataset-id>}"

echo "=== Latency gate ==="
bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator latency \
  --threshold 5000 \
  --last 24h \
  --exit-code \
  --format table

echo "=== Error rate gate ==="
bqx analytics evaluate \
  --project-id "$PROJECT_ID" \
  --dataset-id "$DATASET_ID" \
  --evaluator error-rate \
  --threshold 0.05 \
  --last 24h \
  --exit-code \
  --format table

echo "All gates passed."
```

### Step 3: Configure authentication

In CI, use a service account with `bigquery.jobs.create` and `bigquery.tables.getData`
permissions. Set the `GOOGLE_APPLICATION_CREDENTIALS` environment variable to the
service account key path, or use workload identity federation.

### Step 4: Tune thresholds

After running for a week, review pass rates:

```bash
bqx analytics evaluate \
  --project-id <PROJECT_ID> \
  --dataset-id <DATASET_ID> \
  --evaluator latency \
  --threshold 5000 \
  --last 7d \
  --format table
```

Adjust thresholds to achieve a baseline pass rate of 95%+ before enforcing gates.

## Decision rules

- Use `--exit-code` in CI to fail the pipeline on quality violations
- Use `--last 24h` for per-commit checks, `--last 7d` for release gates
- Run latency and error rate checks as separate steps for clear failure signals
- Use `--agent-id` if your pipeline deploys specific agents independently
- Use `--format json` in CI for machine-readable output; `--format table` for logs

## Constraints

- Agent events must be flowing to BigQuery before the gate can evaluate anything
- `--exit-code` returns exit code 1 when any session fails the threshold
- CI service accounts need BigQuery read permissions on the dataset
