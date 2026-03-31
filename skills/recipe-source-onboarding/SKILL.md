---
name: recipe-source-onboarding
description: Step-by-step recipe for onboarding a new Data Cloud source — profile validation, CA setup, Looker exploration, and database operations bootstrap.
---

## When to use this skill

Use when the user wants to:
- Set up a new source for the first time
- Validate and test a new profile
- Bootstrap CA for BigQuery, Looker, or database sources
- Connect Looker explores to CA

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

Fix any reported issues before proceeding.

### Step 3: Inspect resolved configuration

```bash
dcx profiles show --profile my-profile --format json
```

Verify project, instance, database, and other fields. Secrets are redacted.

### Step 4: Test with a lightweight command

Choose based on source type:

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

## Recipe: Bootstrap CA for BigQuery

### Step 1: Verify table exists

```bash
dcx analytics doctor --project-id PROJECT --dataset-id DATASET --format text
```

### Step 2: Create a data agent

```bash
dcx ca create-agent --name=my-agent \
  --tables=PROJECT.DATASET.TABLE \
  --instructions="Describe what this data contains."
```

### Step 3: Test with a question

```bash
dcx ca ask "what tables are available?" --agent=my-agent --format text
```

### Step 4: Add verified queries (optional)

```bash
dcx ca add-verified-query --agent=my-agent \
  --question="What is the error rate?" \
  --query="SELECT ..."
```

## Recipe: Connect Looker to CA

### Step 1: Create profile

```yaml
# ~/.config/dcx/profiles/sales-looker.yaml
name: sales-looker
source_type: looker
project: my-gcp-project
looker_instance_url: https://mycompany.looker.com
looker_explores:
  - sales_model/orders
  - sales_model/customers
```

### Step 2: Validate

```bash
dcx profiles validate --profile sales-looker --format text
```

### Step 3: Explore content

```bash
dcx looker explores list --profile sales-looker.yaml --format table
```

### Step 4: Ask questions

```bash
dcx ca ask --profile sales-looker.yaml "top selling products last month" --format text
```

## Decision rules

- Always validate before first use: `dcx profiles validate`
- Fix structural issues before testing network connectivity
- Use `--format text` for human debugging, `--format json` for scripting
- Wrong `source_type` is the most common profile error
- Start without `context_set_id` for database sources — add later for accuracy
