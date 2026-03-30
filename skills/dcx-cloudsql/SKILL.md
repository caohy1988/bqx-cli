---
name: dcx-cloudsql
description: Direct Cloud SQL inventory and schema commands via Discovery-driven API. Use for deterministic instance/database listing and schema inspection without natural language.
---

## When to use this skill

Use when the user wants to:
- List Cloud SQL instances or databases in a project
- Get metadata for a specific Cloud SQL instance or database
- Describe Cloud SQL schema columns via a profile
- Perform deterministic inventory checks (not natural language queries)

Do not use for natural language questions — use `dcx-ca-database` instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` (or `DCX_PROJECT` env var)

## Discovery-driven commands

Cloud SQL commands are generated from the bundled `sqladmin/v1` Discovery
document.

### List instances

```bash
dcx cloudsql instances list --project-id my-project --format json
```

### Get instance details

```bash
dcx cloudsql instances get --project-id my-project --instance=my-instance --format json
```

### List databases

```bash
dcx cloudsql databases list --project-id my-project --instance=my-instance --format json
```

### Get database metadata

```bash
dcx cloudsql databases get --project-id my-project --instance=my-instance --database=mydb --format json
```

## Profile-aware schema describe

Describe all columns in a Cloud SQL database using a CA profile:

```bash
dcx cloudsql schema describe --profile cloudsql-app.yaml --format json
dcx cloudsql schema describe --profile cloudsql-app.yaml --format table
```

The profile supplies project, instance, database, and engine context.
The schema prompt adapts to MySQL or PostgreSQL based on the `db_type`
field in the profile.

## Decision rules

- Use `dcx cloudsql` for deterministic inventory and metadata inspection
- Use `dcx ca ask --profile` for natural language exploration of Cloud SQL data
- Use `schema describe` when you need column-level detail from a profile
- Cloud SQL uses `--instance` (not `--instance-id`) for Discovery commands

## Constraints

- Read-only: no create, update, or delete operations
- Schema describe requires a valid Cloud SQL profile with `source_type: cloud_sql`
- Profile must include `db_type` (`mysql` or `postgresql`) for schema describe
