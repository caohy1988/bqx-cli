---
name: dcx-ca-spanner
description: Spanner-specific Conversational Analytics setup, profile configuration, and business query patterns using dcx ca ask.
---

## When to use this skill

Use when the user wants to:
- Set up CA for a Spanner database
- Run business or analytical queries against Spanner via natural language
- Understand Spanner-specific profile configuration
- Troubleshoot Spanner CA issues

## Prerequisites

Load the following skills: `dcx-ca-database`

See **dcx-shared** for authentication and global flags.

## Spanner profile

```yaml
name: finance-spanner
source_type: spanner
project: my-gcp-project
location: us-central1
instance_id: my-spanner-instance
database_id: my-database
```

### Required fields

| Field | Description |
|-------|-------------|
| `source_type` | Must be `spanner` |
| `project` | GCP project ID |
| `location` | Region (e.g., `us-central1`) |
| `instance_id` | Spanner instance name |
| `database_id` | Spanner database name |

### Optional fields

| Field | Description |
|-------|-------------|
| `context_set_id` | Pre-authored context set for improved accuracy |

## Spanner prerequisites

### 1. Enable the Spanner API

```bash
gcloud services enable spanner.googleapis.com
```

### 2. Ensure IAM permissions

The calling identity needs `spanner.databases.read` permission on the
database. The `roles/spanner.databaseReader` role is sufficient.

No additional Data API or IAM user setup is needed — Spanner uses
GoogleSQL and authenticates via IAM directly.

## Usage examples

```bash
# Business queries
dcx ca ask --profile finance-spanner.yaml "total revenue by region"
dcx ca ask --profile finance-spanner.yaml "who are the top 5 customers by spend?"
dcx ca ask --profile finance-spanner.yaml --format text "daily transaction count this week"

# Schema exploration
dcx ca ask --profile finance-spanner.yaml "show all tables"
dcx ca ask --profile finance-spanner.yaml "what columns does the orders table have?"

# Analytical queries
dcx ca ask --profile finance-spanner.yaml "average order value by month"
dcx ca ask --profile finance-spanner.yaml --format json "orders with amount over 1000" | jq '.results | length'
```

## Spanner query patterns

The CA API generates **GoogleSQL** for Spanner databases. Common patterns:

- Aggregations: `SUM`, `COUNT`, `AVG` with `GROUP BY`
- Filtering: `WHERE` with date/timestamp comparisons
- Subqueries: supported for complex questions
- Joins: supported when the database has multiple related tables

The API understands table relationships and generates appropriate joins
automatically when the question spans multiple tables.

## Troubleshooting

### Error: "Spanner API not enabled"

```bash
gcloud services enable spanner.googleapis.com --project=PROJECT_ID
```

### Error: "Permission denied"

Ensure the calling identity has `spanner.databases.read`:

```bash
gcloud spanner databases add-iam-policy-binding DATABASE \
  --instance=INSTANCE \
  --member="user:USER@DOMAIN" \
  --role="roles/spanner.databaseReader"
```

### Error: "context_set_id not found" (404)

Remove `context_set_id` from the profile to query without pre-authored
context, or create the referenced context set.

## Decision rules

- Spanner is the simplest database source to set up — no Data API toggle needed
- Start without `context_set_id` for quick validation
- Use `--format text` for business users, `--format json` for pipelines
- Spanner generates GoogleSQL — syntax differs from PostgreSQL (AlloyDB/Cloud SQL)

## Constraints

- Spanner uses GoogleSQL dialect (not PostgreSQL)
- `cluster_id` is not used for Spanner (AlloyDB only)
- Data agent creation (`ca create-agent`) is not supported for Spanner
- The CA API is preview — Spanner support may evolve
