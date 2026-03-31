---
name: dcx-profiles
description: Profile management commands for listing, inspecting, and validating source profiles across all Data Cloud source types.
---

## When to use this skill

Use when the user wants to:
- List available source profiles
- Inspect a profile's resolved configuration
- Validate a profile before using it with CA or direct commands
- Debug profile setup issues

## Prerequisites

Profiles are YAML files that define source connection context.
See **dcx-bigquery** for authentication.

## Commands

### List profiles

```bash
dcx profiles list --format json
dcx profiles list --format table
```

Lists all discoverable profiles with name, source type, family, and path.

### Show profile details

```bash
dcx profiles show --profile spanner-finance --format json
dcx profiles show --profile /path/to/profile.yaml --format json
```

Displays resolved configuration with secrets redacted. Accepts name or path.

### Validate profile

```bash
dcx profiles validate --profile spanner-finance --format text
```

Checks required fields, source type, and field constraints without network calls.

## Profile discovery order

1. Explicit file path: `--profile path/to/file.yaml`
2. User config directory: `$XDG_CONFIG_HOME/dcx/profiles/` (or `~/.config/dcx/profiles/` when `XDG_CONFIG_HOME` is unset)
3. Repo-local fixtures: `deploy/ca/profiles/` (development/test fallback)

User-local profiles shadow repo-local profiles with the same name.

## Supported source types

| Source Type | Family | Required Fields |
|------------|--------|----------------|
| `bigquery` | Chat | `project` |
| `looker` | Chat | `looker_instance_url`, `looker_explores` |
| `looker_studio` | Chat | `studio_datasource_id` |
| `spanner` | QueryData | `project`, `instance_id`, `database_id` |
| `alloy_db` | QueryData | `project`, `cluster_id`, `instance_id`, `database_id` |
| `cloud_sql` | QueryData | `project`, `instance_id`, `database_id`, `db_type` |

## Decision rules

- Run `dcx profiles validate` before first use of a new profile
- Use `dcx profiles show` to verify resolved configuration (secrets are redacted)
- Use `dcx profiles list` to discover all available profiles
- Profiles are required for CA, schema describe, and Looker content commands
- Profiles are not needed for Discovery-driven inventory commands

## Constraints

- `validate` checks structure only — does not test network connectivity
- Secrets (OAuth client_secret, tokens) are redacted in `show` output
- User-local profiles shadow repo-local profiles with the same name
