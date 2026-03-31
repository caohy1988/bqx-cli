---
name: dcx-alloydb-api
description: Discovery-generated AlloyDB v1 API commands — clusters and instances get/list operations.
---

## When to use this skill

Use when the user wants to:
- List or get AlloyDB clusters or instances
- Check AlloyDB infrastructure inventory
- Verify AlloyDB resources via dry-run

## Prerequisites

Requires `--project-id`. See **dcx-bigquery** for authentication.

## Commands

### Clusters

```bash
dcx alloydb clusters list --project-id P [--location LOC] --format json
dcx alloydb clusters get --project-id P --location LOC --cluster-id C --format json
```

### Instances

```bash
dcx alloydb instances list --project-id P --location LOC --cluster-id C --format json
dcx alloydb instances get --project-id P --location LOC --cluster-id C --instance-id I --format json
```

## Decision rules

- `--location` defaults to `-` (all locations) when omitted
- Use `clusters list` to discover clusters, then `instances list` within a cluster
- For schema/database commands via profile, use `dcx alloydb schema describe --profile` or `dcx alloydb databases list --profile`

## Constraints

- Generated from the AlloyDB v1 Discovery document
- Read-only: no create, update, or delete operations
- AlloyDB uses PostgreSQL dialect
