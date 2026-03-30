---
name: dcx-looker
description: Direct Looker commands for explore/dashboard inspection and instance management. Combines profile-driven content commands with Discovery-driven admin commands.
---

## When to use this skill

Use when the user wants to:
- List or inspect Looker explores and dashboards
- List or get Looker instances (GCP admin API)
- List or get Looker instance backups
- Understand the Looker hybrid command surface

Do not use for natural language questions — use `dcx-ca-looker` instead.

## Prerequisites

See **dcx-shared** for authentication and global flags.

- Content commands (explores, dashboards) require a Looker profile
- Admin commands (instances, backups) require `--project-id` and GCP auth

## Content commands (profile-driven)

These use the per-instance Looker API (`https://<instance>.cloud.looker.com/api/4.0/`).

### List explores

```bash
dcx looker explores list --profile looker-sales.yaml --format json
```

### Get explore details

```bash
dcx looker explores get --profile looker-sales.yaml --explore model/explore_name --format json
```

### List dashboards

```bash
dcx looker dashboards list --profile looker-sales.yaml --format json
```

### Get dashboard details

```bash
dcx looker dashboards get --profile looker-sales.yaml --dashboard-id 42 --format json
```

## Admin commands (Discovery-driven)

These use the GCP Looker admin API (`looker.googleapis.com`), generated from
the bundled `looker/v1` Discovery document.

### List instances

```bash
dcx looker instances list --project-id my-project --format json
dcx looker instances list --project-id my-project --location us-central1 --format json
```

### Get instance details

```bash
dcx looker instances get --project-id my-project --location us-central1 --instance-id my-looker --format json
```

### List backups

```bash
dcx looker backups list --project-id my-project --location us-central1 --instance-id my-looker --format json
```

### Get backup details

```bash
dcx looker backups get --project-id my-project --location us-central1 --instance-id my-looker --backup-id bk1 --format json
```

## Hybrid architecture

Looker has two distinct APIs unified under `dcx looker`:

| Subcommand | API | Auth |
|-----------|-----|------|
| `explores`, `dashboards` | Per-instance Looker API | Looker profile credentials |
| `instances`, `backups` | GCP admin API (Discovery) | GCP IAM / ADC |

## Decision rules

- Use `dcx looker explores|dashboards` for BI content inspection
- Use `dcx looker instances|backups` for infrastructure inventory
- Use `dcx ca ask --profile` for natural language Looker data exploration
- `--location` defaults to `-` (all locations) when omitted for admin commands
- Content commands require `--profile`; admin commands require `--project-id`

## Constraints

- Read-only: no create, update, or delete operations
- Content commands require a valid Looker profile with `source_type: looker`
- Explore reference format must be `model/explore` (with slash separator)
- Admin commands use GCP auth, not Looker instance credentials
