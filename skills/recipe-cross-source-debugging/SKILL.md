---
name: recipe-cross-source-debugging
description: Step-by-step recipe for debugging issues across multiple Data Cloud sources using direct commands and CA together.
---

## When to use this skill

Use when the user wants to:
- Compare data or schema across different source types
- Debug connectivity issues across multiple sources
- Investigate data discrepancies between BigQuery and database sources
- Verify cross-source setup is working end-to-end

## Prerequisites

Load the following skills: `dcx-profiles`, `dcx-shared`

Source-specific skills as needed: `dcx-spanner`, `dcx-alloydb`,
`dcx-cloudsql`, `dcx-looker`

## Recipe: Verify cross-source connectivity

### Step 1: Check authentication

```bash
dcx auth status
```

GCP ADC covers BigQuery, Spanner, AlloyDB, Cloud SQL, and Looker admin.
Looker content commands use profile-provided credentials.

### Step 2: List all profiles

```bash
dcx profiles list --format table
```

### Step 3: Validate each profile

```bash
dcx profiles validate --profile spanner-finance --format text
dcx profiles validate --profile alloydb-ops --format text
dcx profiles validate --profile cloudsql-app --format text
```

### Step 4: Test inventory for each source

```bash
# BigQuery
dcx datasets list --project-id PROJECT --format json

# Spanner
dcx spanner instances list --project-id PROJECT --format json

# AlloyDB
dcx alloydb clusters list --project-id PROJECT --format json

# Cloud SQL
dcx cloudsql instances list --project-id PROJECT --format json

# Looker (admin)
dcx looker instances list --project-id PROJECT --format json
```

### Step 5: Test schema access

```bash
dcx spanner schema describe --profile spanner-finance --format table
dcx alloydb schema describe --profile alloydb-ops --format table
dcx cloudsql schema describe --profile cloudsql-app --format table
```

### Step 6: Test CA access

```bash
dcx ca ask --profile spanner-finance "show all tables" --format text
dcx ca ask --profile alloydb-ops "show all tables" --format text
```

## Recipe: Compare schemas across sources

### Step 1: Get schema from each source

```bash
dcx spanner schema describe --profile spanner-finance --format json > /tmp/spanner-schema.json
dcx cloudsql schema describe --profile cloudsql-app --format json > /tmp/cloudsql-schema.json
```

### Step 2: Compare column counts

```bash
dcx spanner schema describe --profile spanner-finance --format text
dcx cloudsql schema describe --profile cloudsql-app --format text
```

## Decision rules

- Start with `auth status` — most cross-source failures are auth issues
- Use direct commands for deterministic checks, CA for exploratory queries
- Compare schemas with `--format table` for visual inspection
- Test one source at a time to isolate failures
- Spanner is the simplest source to set up (no Data API toggle needed)

## Common issues

| Symptom | Likely cause | Fix |
|---------|-------------|-----|
| 403 on Spanner | Missing `spanner.databases.read` | Grant `roles/spanner.databaseReader` |
| 500 on AlloyDB CA | Missing IAM database user | Create IAM user in AlloyDB |
| Wrong source type error | Profile mismatch | Check `source_type` in profile |
| Empty instances list | Wrong project or location | Verify `--project-id` and `--location` |
