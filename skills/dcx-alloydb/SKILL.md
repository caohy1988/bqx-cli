---
name: dcx-alloydb
description: Direct AlloyDB inventory, schema, and database commands via Discovery-driven API. Use for deterministic cluster/instance listing, schema inspection, and database enumeration without natural language.
---

## When to use this skill

Use when the user wants to:
- List AlloyDB clusters or instances in a project
- Get metadata for a specific cluster or instance
- Describe AlloyDB schema columns via a profile
- List databases in an AlloyDB instance via a profile
- Perform deterministic inventory checks (not natural language queries)

Do not use for natural language questions — use `dcx-ca-alloydb` instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

Requires: `--project-id` (or `DCX_PROJECT` env var)

## Discovery-driven commands

AlloyDB commands are generated from the bundled `alloydb/v1` Discovery
document. The global `--location` defaults to `-` (all locations).

### List clusters

```bash
dcx alloydb clusters list --project-id my-project --format json
dcx alloydb clusters list --project-id my-project --location us-central1 --format json
```

### Get cluster details

```bash
dcx alloydb clusters get --project-id my-project --location us-central1 --cluster-id my-cluster --format json
```

### List instances in a cluster

```bash
dcx alloydb instances list --project-id my-project --location us-central1 --cluster-id my-cluster --format json
```

### Get instance details

```bash
dcx alloydb instances get --project-id my-project --location us-central1 --cluster-id my-cluster --instance-id my-inst --format json
```

## Profile-aware commands

### Schema describe

Describe all columns in an AlloyDB PostgreSQL database:

```bash
dcx alloydb schema describe --profile alloydb-ops.yaml --format json
dcx alloydb schema describe --profile alloydb-ops.yaml --format table
```

### Databases list

List non-template databases in an AlloyDB instance:

```bash
dcx alloydb databases list --profile alloydb-ops.yaml --format json
dcx alloydb databases list --profile alloydb-ops.yaml --format text
```

Both commands use CA QueryData under the hood, routed by the profile.

## Decision rules

- Use `dcx alloydb clusters|instances` for infrastructure inventory
- Use `dcx alloydb schema describe` for column-level metadata via profile
- Use `dcx alloydb databases list` to enumerate databases via profile
- Use `dcx ca ask --profile` for natural language queries over AlloyDB data
- `--location` defaults to `-` (all locations) when omitted

## Constraints

- Read-only: no create, update, or delete operations
- Schema and database commands require a valid AlloyDB profile with `source_type: alloy_db`
- AlloyDB uses PostgreSQL dialect (not GoogleSQL)
- Database listing filters out template databases automatically
