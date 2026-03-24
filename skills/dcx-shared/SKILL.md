---
name: dcx-shared
description: Common setup, authentication, global flags, output formats, and dataset requirements for the dcx CLI. Reference this skill from other dcx skills instead of duplicating auth and flag guidance.
---

## When to use this skill

Use when the user asks about:
- "how do I authenticate dcx"
- "what flags does dcx need"
- "why is dataset-id required"
- "show me dcx output formats"
- "set up dcx for my project"
- "what environment variables does dcx use"

Do not use when the user is asking about a specific command workflow — use the command-specific skill instead.

## Authentication

dcx resolves credentials in this order (first match wins):

1. `--token` flag or `DCX_TOKEN` env var (static bearer token)
2. `--credentials-file` flag or `DCX_CREDENTIALS_FILE` env var (service account or authorized user JSON)
3. Stored credentials from `dcx auth login` (OAuth browser flow, uses refresh token)
4. `GOOGLE_APPLICATION_CREDENTIALS` env var (standard GCP credential file)
5. Default ADC (`gcloud auth application-default login` or GCE metadata)

### Auth commands

```bash
dcx auth login          # opens browser for Google OAuth
dcx auth status         # shows which credential source is active
dcx auth logout         # clears stored credentials
```

## Global flags

| Flag | Env var | Default | Required |
|------|---------|---------|----------|
| `--project-id` | `DCX_PROJECT` | — | Yes (all commands) |
| `--dataset-id` | `DCX_DATASET` | — | Analytics commands only |
| `--location` | `DCX_LOCATION` | `US` | No |
| `--table` | — | `agent_events` | No |
| `--format` | — | `json` | No |
| `--token` | `DCX_TOKEN` | — | No |
| `--credentials-file` | `DCX_CREDENTIALS_FILE` | — | No |

## Dataset requirement

- `jobs query` does **not** require `--dataset-id`
- All `analytics` commands (`doctor`, `evaluate`, `get-trace`) **require** `--dataset-id`
- If missing, dcx returns an error before attempting authentication

## Output formats

| Format | Flag | Best for |
|--------|------|----------|
| JSON | `--format json` | Automation, piping to `jq`, CI scripts |
| Table | `--format table` | Scanning rows visually in a terminal |
| Text | `--format text` | Demos, summaries, human-readable output |

## Examples

```bash
# Check auth status
dcx auth status

# Login via browser
dcx auth login

# Minimal query (only needs project-id)
dcx jobs query --project-id my-proj --query "SELECT 1"

# Analytics command (needs both project-id and dataset-id)
dcx analytics doctor --project-id my-proj --dataset-id analytics_demo --format text
```

## Constraints

- dcx supports service account JSON, authorized user JSON, and OAuth browser login
- The `--token` flag is hidden from help output but functional
- All input validation (identifiers, session IDs, durations) runs before auth resolution
