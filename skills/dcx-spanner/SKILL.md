---
name: dcx-spanner
description: Direct Spanner inventory and schema commands via Discovery-driven API. Use for deterministic instance/database listing, DDL retrieval, and schema inspection without natural language.
---

## When to use this skill

Use when the user wants to:
- List Spanner instances or databases in a project
- Get metadata for a specific Spanner instance or database
- Retrieve DDL (schema definitions) for a Spanner database
- Describe Spanner schema columns via a profile
- Perform deterministic inventory checks (not natural language queries)

Do not use for natural language questions — use `dcx-ca-spanner` instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` (or `DCX_PROJECT` env var)

## Discovery-driven commands

Spanner commands are generated from the bundled `spanner/v1` Discovery
document. They use the same dynamic pipeline as BigQuery.

### List instances

```bash
dcx spanner instances list --project-id my-project --format json
```

### Get instance details

```bash
dcx spanner instances get --project-id my-project --instance-id my-instance --format json
```

### List databases

```bash
dcx spanner databases list --project-id my-project --instance-id my-instance --format json
```

### Get database metadata

```bash
dcx spanner databases get --project-id my-project --instance-id my-instance --database-id mydb --format json
```

### Get database DDL

```bash
dcx spanner databases get-ddl --project-id my-project --instance-id my-instance --database-id mydb --format json
```

## Profile-aware schema describe

Describe all columns in a Spanner database using a CA profile:

```bash
dcx spanner schema describe --profile spanner-finance.yaml --format json
dcx spanner schema describe --profile spanner-finance.yaml --format table
```

The profile supplies project, instance, and database context. The command
uses CA QueryData under the hood and returns structured column metadata.

## Dry-run mode

Verify URL construction without auth:

```bash
dcx spanner instances list --project-id my-project --dry-run
# → {"dry_run":true,"method":"GET","url":"https://spanner.googleapis.com/v1/projects/my-project/instances"}
```

## Decision rules

- Use `dcx spanner` for deterministic inventory and metadata inspection
- Use `dcx ca ask --profile` for natural language exploration of Spanner data
- Use `schema describe` when you need column-level detail from a profile
- Use `databases get-ddl` when you need exact CREATE TABLE statements
- All path parameters are validated locally before network calls

## Constraints

- Read-only: no create, update, or delete operations
- Schema describe requires a valid Spanner profile with `source_type: spanner`
- DDL retrieval returns GoogleSQL CREATE statements
