---
name: recipe-source-profile-validation
description: Step-by-step recipe for validating and debugging source profiles before using them with CA or direct commands.
---

## When to use this skill

Use when the user wants to:
- Validate a new profile before running queries
- Debug why a profile-based command is failing
- Set up profiles for a new Data Cloud source
- Verify profile discovery and resolution

## Prerequisites

Load the following skills: `dcx-profiles`, `dcx-shared`

## Recipe: Validate a new profile

### Step 1: List existing profiles

```bash
dcx profiles list --format table
```

Verify the profile appears and has the correct source type.

### Step 2: Validate structure

```bash
dcx profiles validate --profile my-profile --format text
```

This checks required fields, source type constraints, and field formats
without making network calls. Fix any reported issues before proceeding.

### Step 3: Inspect resolved configuration

```bash
dcx profiles show --profile my-profile --format json
```

Verify project, instance, database, and other fields are correct.
Secrets are redacted in output.

### Step 4: Test with a lightweight command

Choose the right test command based on source type:

```bash
# Spanner
dcx spanner schema describe --profile my-profile --format text

# AlloyDB
dcx alloydb databases list --profile my-profile --format text

# Cloud SQL
dcx cloudsql schema describe --profile my-profile --format text

# Looker
dcx looker explores list --profile my-profile --format text

# BigQuery (via CA)
dcx ca ask --profile my-profile "show all tables" --format text
```

### Step 5: Run a business query

```bash
dcx ca ask --profile my-profile "total count of records" --format json
```

## Recipe: Debug a failing profile

### Step 1: Check source type match

```bash
dcx profiles show --profile my-profile --format json | grep source_type
```

Ensure the source type matches the command family:
- `dcx spanner schema describe` requires `source_type: spanner`
- `dcx alloydb databases list` requires `source_type: alloy_db`
- `dcx cloudsql schema describe` requires `source_type: cloud_sql`
- `dcx looker explores list` requires `source_type: looker`

### Step 2: Test auth independently

```bash
dcx auth status
```

Ensure ADC or token is active.

### Step 3: Test with dry-run (inventory commands)

```bash
dcx spanner instances list --project-id PROJECT --dry-run
```

Verify URL construction is correct before auth.

## Decision rules

- Always validate before first use: `dcx profiles validate`
- Fix structural issues before testing network connectivity
- Use `--format text` for human debugging, `--format json` for scripting
- Wrong source type is the most common profile error
