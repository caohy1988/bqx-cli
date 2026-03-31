---
name: dcx-spanner-api
description: Discovery-generated Spanner v1 API commands — instances and databases get/list/get-ddl operations.
---

## When to use this skill

Use when the user wants to:
- List or get Spanner instances or databases
- Retrieve DDL (CREATE TABLE statements) for a Spanner database
- Verify Spanner resources via dry-run

## Prerequisites

Requires `--project-id`. See **dcx-bigquery** for authentication.

## Commands

### Instances

```bash
dcx spanner instances list --project-id P --format json
dcx spanner instances get --project-id P --instance-id I --format json
```

### Databases

```bash
dcx spanner databases list --project-id P --instance-id I --format json
dcx spanner databases get --project-id P --instance-id I --database-id D --format json
dcx spanner databases get-ddl --project-id P --instance-id I --database-id D --format json
```

## Dry-run

```bash
dcx spanner instances list --project-id my-project --dry-run
```

## Decision rules

- Use `instances list` to discover Spanner instances in a project
- Use `databases get-ddl` for exact CREATE TABLE statements (GoogleSQL)
- Use `--dry-run` to verify URL construction without auth
- For column-level schema via profile, use `dcx spanner schema describe --profile`

## Constraints

- Generated from the Spanner v1 Discovery document
- Read-only: no create, update, or delete operations
- DDL returns GoogleSQL CREATE statements
- All path parameters are validated locally before network calls
