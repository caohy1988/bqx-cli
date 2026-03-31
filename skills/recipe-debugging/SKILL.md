---
name: recipe-debugging
description: Step-by-step recipe for debugging agent sessions, cross-source connectivity issues, and CA failures across Data Cloud sources.
---

## When to use this skill

Use when the user wants to:
- Debug a failing agent session
- Troubleshoot cross-source connectivity
- Diagnose CA failures for any source type
- Triage agent incidents

## Recipe: Debug a failing session

### Step 1: Evaluate to find failing sessions

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h \
  --project-id PROJECT --dataset-id DATASET --format json
```

### Step 2: Inspect the trace

```bash
dcx analytics get-trace --session-id SESSION_ID \
  --project-id PROJECT --dataset-id DATASET --format text
```

### Step 3: Check for error patterns

```bash
dcx analytics evaluate --evaluator error-rate --threshold 0.1 --last 24h \
  --project-id PROJECT --dataset-id DATASET --format text
```

### Step 4: Compare against baseline (drift check)

```bash
dcx analytics evaluate --evaluator latency --threshold 5000 --last 7d --format json > baseline.json
dcx analytics evaluate --evaluator latency --threshold 5000 --last 24h --format json > current.json
```

## Recipe: Debug cross-source connectivity

### Step 1: Check auth

```bash
dcx auth status
```

### Step 2: Check source type match

```bash
dcx profiles show --profile my-profile --format json
```

Ensure the source type matches the command family:
- `dcx spanner schema describe` requires `source_type: spanner`
- `dcx alloydb databases list` requires `source_type: alloy_db`
- `dcx cloudsql schema describe` requires `source_type: cloud_sql`
- `dcx looker explores list` requires `source_type: looker`

### Step 3: Validate profile structure

```bash
dcx profiles validate --profile my-profile --format text
```

### Step 4: Test with dry-run (inventory commands)

```bash
dcx spanner instances list --project-id PROJECT --dry-run
```

Verify URL construction is correct before auth.

### Step 5: Test with a lightweight command

```bash
dcx ca ask --profile my-profile "show all tables" --format text
```

## Recipe: Diagnose CA failures

### Common causes

| Error | Likely cause | Fix |
|-------|-------------|-----|
| "Data API Access is default" | AlloyDB Data API not enabled | Enable via REST API |
| "Internal error" (500) | IAM user not created or instance not ready | Create IAM DB user, retry |
| "Permission denied" | Missing IAM role | Grant `spanner.databases.read` or equivalent |
| "context_set_id not found" (404) | Invalid context set reference | Remove `context_set_id` or create the set |
| Source type mismatch | Wrong `source_type` in profile | Fix to match service |

## Decision rules

- Fix structural issues before testing connectivity
- Check auth independently before blaming the source
- Use `--dry-run` to verify URL construction without auth
- Wrong source type is the most common profile error
- Start without `context_set_id` for database sources
