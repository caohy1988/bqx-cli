---
name: dcx-cloudsql-api
description: Discovery-generated Cloud SQL (sqladmin v1) API commands — instances and databases get/list operations.
---

## When to use this skill

Use when the user wants to:
- List or get Cloud SQL instances or databases
- Check Cloud SQL infrastructure inventory
- Verify Cloud SQL resources via dry-run

## Prerequisites

Requires `--project-id`. See **dcx-bigquery** for authentication.

## Commands

### Instances

```bash
dcx cloudsql instances list --project-id P --format json
dcx cloudsql instances get --project-id P --instance=INST --format json
```

### Databases

```bash
dcx cloudsql databases list --project-id P --instance=INST --format json
dcx cloudsql databases get --project-id P --instance=INST --database=DB --format json
```

## Decision rules

- Cloud SQL uses `--instance` (not `--instance-id`) for Discovery commands
- For schema via profile, use `dcx cloudsql schema describe --profile`

## Constraints

- Generated from the sqladmin v1 Discovery document
- Read-only: no create, update, or delete operations
