---
name: dcx-looker-admin-api
description: Discovery-generated Looker v1 admin API commands — instances and backups get/list operations via GCP admin API.
---

## When to use this skill

Use when the user wants to:
- List or get Looker instances in a GCP project
- List or get Looker instance backups
- Check Looker infrastructure inventory

## Prerequisites

Requires `--project-id` and GCP IAM/ADC auth.
See **dcx-bigquery** for authentication.

## Commands

### Instances

```bash
dcx looker instances list --project-id P [--location LOC] --format json
dcx looker instances get --project-id P --location LOC --instance-id I --format json
```

### Backups

```bash
dcx looker backups list --project-id P --location LOC --instance-id I --format json
dcx looker backups get --project-id P --location LOC --instance-id I --backup-id B --format json
```

## Decision rules

- `--location` defaults to `-` (all locations) when omitted
- These are GCP admin API commands — they use GCP auth, not Looker instance credentials
- For Looker content (explores, dashboards), use **dcx-looker** with a profile

## Constraints

- Generated from the Looker v1 Discovery document
- Read-only: no create, update, or delete operations
- Uses GCP admin API, not per-instance Looker API
