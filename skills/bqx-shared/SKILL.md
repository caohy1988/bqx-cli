---
name: bqx-shared
description: Common setup, authentication, global flags, output formats, and dataset requirements for the bqx CLI. Reference this skill from other bqx skills instead of duplicating auth and flag guidance.
---

## When to use this skill

Use when the user asks about:
- "how do I authenticate bqx"
- "what flags does bqx need"
- "why is dataset-id required"
- "show me bqx output formats"
- "set up bqx for my project"
- "what environment variables does bqx use"

Do not use when the user is asking about a specific command workflow — use the command-specific skill instead.

## Authentication

bqx resolves credentials in this order (first match wins):

1. `--token` flag or `BQX_TOKEN` env var (static bearer token)
2. `--credentials-file` flag or `BQX_CREDENTIALS_FILE` env var (service account or authorized user JSON)
3. Stored credentials from `bqx auth login` (OAuth browser flow, uses refresh token)
4. `GOOGLE_APPLICATION_CREDENTIALS` env var (standard GCP credential file)
5. Default ADC (`gcloud auth application-default login` or GCE metadata)

### Auth commands

```bash
bqx auth login          # opens browser for Google OAuth
bqx auth status         # shows which credential source is active
bqx auth logout         # clears stored credentials
```

## Global flags

| Flag | Env var | Default | Required |
|------|---------|---------|----------|
| `--project-id` | `BQX_PROJECT` | — | Yes (all commands) |
| `--dataset-id` | `BQX_DATASET` | — | Analytics commands only |
| `--location` | `BQX_LOCATION` | `US` | No |
| `--table` | — | `agent_events` | No |
| `--format` | — | `json` | No |
| `--token` | `BQX_TOKEN` | — | No |
| `--credentials-file` | `BQX_CREDENTIALS_FILE` | — | No |

## Dataset requirement

- `jobs query` does **not** require `--dataset-id`
- All `analytics` commands (`doctor`, `evaluate`, `get-trace`) **require** `--dataset-id`
- If missing, bqx returns an error before attempting authentication

## Output formats

| Format | Flag | Best for |
|--------|------|----------|
| JSON | `--format json` | Automation, piping to `jq`, CI scripts |
| Table | `--format table` | Scanning rows visually in a terminal |
| Text | `--format text` | Demos, summaries, human-readable output |

## Examples

```bash
# Check auth status
bqx auth status

# Login via browser
bqx auth login

# Minimal query (only needs project-id)
bqx jobs query --project-id my-proj --query "SELECT 1"

# Analytics command (needs both project-id and dataset-id)
bqx analytics doctor --project-id my-proj --dataset-id analytics_demo --format text
```

## Constraints

- bqx supports service account JSON, authorized user JSON, and OAuth browser login
- The `--token` flag is hidden from help output but functional
- All input validation (identifiers, session IDs, durations) runs before auth resolution
