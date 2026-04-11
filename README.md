# dcx — Agent-Native Data Cloud CLI

An agent-native CLI for Google Cloud's Data Cloud, built in Rust.
Covers BigQuery, Spanner, AlloyDB, Cloud SQL, and Looker through a
single binary with structured JSON output, declarative skills, and
an MCP bridge.

**6.0x faster than `bq`** on BigQuery tasks
([benchmark](docs/benchmark_results_bigquery.md)) — 100% correctness
vs 91% for `bq` across 11 validated tasks.

## Quick Start

```bash
# Install via npm (6 platforms)
npx dcx --help

# Or install globally
npm install -g dcx

# Uses existing gcloud credentials by default
dcx datasets list --project-id=myproject
```

| Platform | Package |
|---|---|
| macOS ARM64 (Apple Silicon) | `@dcx-cli/darwin-arm64` |
| macOS x64 (Intel) | `@dcx-cli/darwin-x64` |
| Linux x64 | `@dcx-cli/linux-x64` |
| Linux ARM64 | `@dcx-cli/linux-arm64` |
| Windows x64 | `@dcx-cli/win32-x64` |
| Windows ARM64 | `@dcx-cli/win32-arm64` |

---

## Command Overview

dcx has five command domains, all sharing the same auth, output formatting,
pagination, and profile system.

```
dcx <resource> <method>              # BigQuery API (dynamic, from Discovery)
dcx <service> <resource> <method>    # Spanner / AlloyDB / Cloud SQL / Looker (dynamic)
dcx analytics <command>              # Agent Analytics SDK (static)
dcx ca <command>                     # Conversational Analytics (static)
dcx meta|auth|profiles|mcp <cmd>    # Tooling and introspection
```

### BigQuery API (dynamic)

Generated from the BigQuery v2 Discovery Document. Read-only.

```bash
dcx datasets list --project-id=myproject
dcx datasets get --project-id=myproject --dataset-id=analytics
dcx tables list --project-id=myproject --dataset-id=analytics
dcx tables get --project-id=myproject --dataset-id=analytics --table-id=events
dcx routines list --project-id=myproject --dataset-id=analytics
dcx models list --project-id=myproject --dataset-id=analytics

# SQL queries
dcx jobs query --project-id=myproject --query="SELECT COUNT(*) FROM analytics.events"

# Dry-run (local-only, no network — 33x faster than bq --dry_run)
dcx jobs query --query="SELECT 1" --dry-run
```

### Data Cloud APIs (dynamic)

Same Discovery-driven pipeline, namespaced per service.

```bash
# Spanner
dcx spanner instances list --project-id=myproject
dcx spanner databases list --project-id=myproject --instance-id=my-inst
dcx spanner databases get-ddl --project-id=myproject --instance-id=my-inst --database-id=mydb

# AlloyDB
dcx alloydb clusters list --project-id=myproject
dcx alloydb instances list --project-id=myproject --cluster-id=my-cluster --location=us-central1

# Cloud SQL
dcx cloudsql instances list --project-id=myproject
dcx cloudsql databases list --project-id=myproject --instance=my-inst

# Looker (Discovery: instance management; hand-written: content)
dcx looker instances list --project-id=myproject --location=us-central1
dcx looker explores list --profile=sales-looker
dcx looker dashboards get --profile=sales-looker --dashboard-id=42
```

### Agent Analytics

Wraps the BigQuery Agent Analytics SDK. All 12 SDK commands, 6 evaluators.

```bash
# Health check
dcx analytics doctor

# Evaluate sessions (latency, error-rate, turn-count, token-efficiency, ttft, cost)
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --exit-code

# Session traces
dcx analytics get-trace --session-id=sess-001
dcx analytics list-traces --last=7d --agent-id=support_bot

# Drift detection against golden question set
dcx analytics drift --golden-dataset=golden_qs --last=7d --exit-code

# Insights, distribution, HITL metrics
dcx analytics insights --last=24h
dcx analytics distribution --last=24h
dcx analytics hitl-metrics --last=24h

# Event-type views and categorical evaluation
dcx analytics views create-all --prefix=adk_
dcx analytics categorical-eval --metrics-file=./metrics.json --last=24h
```

**Exit codes:** 0 = success, 1 = evaluation failure (`--exit-code`),
2 = infrastructure error. Matches upstream SDK semantics.

### Conversational Analytics

Natural language queries across 6 data sources via a single `ca ask` command.

| Source | API Family |
|--------|-----------|
| BigQuery, Looker, Looker Studio | Chat / DataAgent |
| AlloyDB, Spanner, Cloud SQL | QueryData |

```bash
# BigQuery: ask via data agent
dcx ca ask "What were the top errors yesterday?" --agent=agent-analytics

# BigQuery: ask with inline table context
dcx ca ask "p95 latency for support_bot?" --tables=myproject.analytics.events

# Multi-source via profiles
dcx ca ask --profile=finance-spanner.yaml "total revenue by region"
dcx ca ask --profile=ops-alloydb.yaml "show active connections"
dcx ca ask --profile=sales-looker.yaml "top selling products?"

# Agent management
dcx ca create-agent --name=agent-analytics --tables=myproject.analytics.events
dcx ca list-agents --project-id=myproject
dcx ca add-verified-query --agent=agent-analytics \
  --question="error rate for support_bot?" --query="SELECT ..."
```

### Introspection and Tooling

```bash
# Machine-readable command contract
dcx meta commands --format=json
dcx meta describe analytics evaluate --format=json

# MCP bridge (JSON-RPC 2.0 over stdio)
dcx mcp serve

# Auth
dcx auth login                    # Interactive OAuth
dcx auth status                   # Current auth state
dcx auth check --format=json      # Preflight check (no API calls)

# Profiles
dcx profiles list
dcx profiles validate --profile=bench
dcx profiles test --profile=bench

# Shell completions
dcx completions bash > /usr/local/etc/bash_completion.d/dcx
dcx completions zsh > "${fpath[1]}/_dcx"
dcx completions fish > ~/.config/fish/completions/dcx.fish
```

---

## Output Format

All commands default to structured JSON. `--format` controls output:

```bash
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h
# → {"evaluator":"latency","pass_rate":0.70,"sessions":[...]}

dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --format=table
# → SESSION_ID   PASSED  LATENCY_MS  SCORE
#   sess-001     true    2340        0.85

dcx jobs query --query="SELECT 1" --dry-run
# → {"dry_run":true,"url":"...","method":"POST","body":{...}}
```

List responses use a normalized envelope:
```json
{"items": [...], "source": "BigQuery", "next_page_token": "..."}
```

Errors are structured JSON on stderr:
```json
{"error": "Dataset not found: does_not_exist"}
```

---

## Authentication

Five methods in priority order:

| Priority | Method | Use Case |
|----------|--------|----------|
| 1 | `DCX_TOKEN` env var | Pre-obtained access token |
| 2 | `DCX_CREDENTIALS_FILE` env var | Service account JSON |
| 3 | `dcx auth login` | Interactive OAuth (AES-256-GCM at rest) |
| 4 | `GOOGLE_APPLICATION_CREDENTIALS` | Standard ADC |
| 5 | `gcloud auth application-default` | Implicit gcloud credentials |

```bash
# Uses existing gcloud credentials (priority 5)
dcx datasets list --project-id=myproject

# Service account for CI
export DCX_CREDENTIALS_FILE=/path/to/sa-key.json
dcx analytics evaluate --evaluator=latency --last=24h --exit-code
```

---

## MCP Bridge

`dcx mcp serve` exposes all read-only commands as an MCP (Model Context
Protocol) server over stdio, using JSON-RPC 2.0.

- Domain filtering via `MCP_DOMAINS` env var
- Mutations and interactive commands excluded
- Format flag forced to JSON

```bash
# Start MCP server
dcx mcp serve

# Example JSON-RPC request
echo '{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"dcx_datasets_list","arguments":{"project_id":"myproject"}}}' | dcx mcp serve
```

Supported methods: `initialize`, `tools/list`, `tools/call`, `ping`.

---

## Skills

14 declarative skills following the [Agent Skills](https://agentskills.io)
open standard. Compatible with Claude Code, Gemini CLI, Cursor, Copilot,
and Codex.

| Type | Count | Generated? | Skills |
|------|-------|------------|--------|
| Router | 6 | No | `dcx-bigquery`, `dcx-analytics`, `dcx-ca`, `dcx-databases`, `dcx-looker`, `dcx-profiles` |
| API | 5 | Yes | `dcx-bigquery-api`, `dcx-spanner-api`, `dcx-alloydb-api`, `dcx-cloudsql-api`, `dcx-looker-admin-api` |
| Recipe | 3 | No | `recipe-source-onboarding`, `recipe-debugging`, `recipe-quality-ops` |

```bash
# Generate API skills from Discovery Documents
dcx generate-skills --output-dir=./skills

# Gemini extension manifest
# Packaged at extensions/gemini/manifest.json (17 tools)
dcx meta gemini-tools --format=json
```

---

## Security

- **Model Armor:** `--sanitize <template>` screens responses through
  [Model Armor](https://cloud.google.com/security-command-center/docs/model-armor-overview)
  for prompt injection. Set `DCX_SANITIZE_TEMPLATE` for global default.
- **Credential encryption:** AES-256-GCM at rest, key in OS keyring.
- **Read-only API allowlists:** Dynamic commands restricted to read operations.
  Mutations excluded from MCP bridge.
- **Least-privilege defaults:** `dcx auth login` requests BigQuery-only
  scopes. `-s` *replaces* (not appends) the scope set.

---

## Profiles

Source profiles are YAML files in `~/.config/dcx/profiles/` that configure
connection details for CA and schema commands.

```yaml
# ~/.config/dcx/profiles/finance-spanner.yaml
name: finance-spanner
source_type: spanner
project: my-gcp-project
location: us-central1
instance_id: my-spanner-instance
database_id: my-database
```

Supported source types: `bigquery`, `looker`, `looker_studio`, `alloy_db`,
`spanner`, `cloud_sql`.

See [docs/source-matrix.md](docs/source-matrix.md) for the full source
compatibility matrix.

---

## CI/CD

```yaml
# GitHub Actions
- run: npm install -g dcx
- run: dcx analytics evaluate --evaluator latency --threshold 5000 --last 1h --exit-code
```

`--exit-code` returns exit code 1 on evaluation failure — GitHub Actions,
Jenkins, or any CI system treats this as a step failure.

See [docs/github-actions.md](docs/github-actions.md) for full examples.

---

## Benchmarks

Systematic benchmark suite comparing dcx against `bq` and `gcloud spanner`.

| Track | Tasks | Key Result |
|-------|-------|------------|
| [BigQuery parity](docs/benchmark_results_bigquery.md) | 12 | **6.0x faster**, 100% vs 91% correctness |
| Spanner parity | 11 | 1.3–3.9x faster on error-handling tasks |
| dcx differentiated | 8 | 7/8 pass, avg 141 ms |

See [docs/cli_benchmark_plan.md](docs/cli_benchmark_plan.md) for methodology
and [benchmarks/](benchmarks/) for task specs, runner, and raw results.

```bash
# Run benchmarks
benchmarks/scripts/run_benchmarks.sh --tasks bigquery_overlap --trials 3 --cold-trials 1

# Generate scorecard
python3 benchmarks/scripts/score_results.py benchmarks/results/raw/<run-id>
```

---

## Architecture

### Dynamic Command Generation

Like [`gws`](https://github.com/googleworkspace/cli), dcx generates commands
from bundled Google Cloud Discovery Documents at startup:

| Service | Namespace | Discovery Doc | Commands |
|---------|-----------|---------------|----------|
| BigQuery | _(top-level)_ | `bigquery/v2` | datasets, tables, routines, models |
| Spanner | `spanner` | `spanner/v1` | instances, databases, getDdl |
| AlloyDB | `alloydb` | `alloydb/v1` | clusters, instances |
| Cloud SQL | `cloudsql` | `sqladmin/v1` | instances, databases |
| Looker | `looker` | `looker/v1` | instances, backups |

Discovery Documents are pinned at build time (`include_str!`). No runtime
fetch, no network dependency. Read-only allowlists per service.

### Profile-Aware Helpers

Schema and database helpers use CA QueryData, routed by source profile:

```bash
dcx spanner schema describe --profile=spanner-finance
dcx alloydb schema describe --profile=alloydb-ops
dcx cloudsql schema describe --profile=cloudsql-app
dcx alloydb databases list --profile=alloydb-ops
```

### Testing

615 tests across 21 test binaries:

- **Unit tests:** Core parsing, auth resolution, output formatting
- **Integration tests:** Golden-file / snapshot tests against expected JSON
- **API mocking:** Record/replay via `wiremock` — no live GCP in CI
- **Contract tests:** Output-key regression, exit-code assertions
- **SDK contract CI:** Detects stale compatibility contracts; weekly sync
  opens PRs when upstream SDK changes
- **E2E:** Optional `--live` suite against a dedicated GCP project

---

## Documentation

| Doc | Description |
|-----|-------------|
| [Benchmark results (BigQuery)](docs/benchmark_results_bigquery.md) | dcx vs bq: latency, correctness, token efficiency |
| [Benchmark plan](docs/cli_benchmark_plan.md) | Methodology, scoring model, success criteria |
| [dcx vs bq](docs/dcx-vs-bq.md) | Technical comparison and workflow gallery |
| [GitHub Actions](docs/github-actions.md) | CI/CD integration examples |
| [Source matrix](docs/source-matrix.md) | CA source compatibility |
| [SDK alignment](docs/analytics_sdk_alignment_plan.md) | Analytics SDK parity plan |
| [SDK contract](docs/analytics_sdk_contract.md) | Per-flag compatibility status |
| [E2E validation](docs/e2e-validation.md) | Reproducible validation script |

---

## Implementation Status

All 6 phases complete. Current version: 0.5.0.

| Phase | Version | What shipped |
|-------|---------|-------------|
| 1 | 0.1 | Core CLI, `analytics` (doctor, evaluate, get-trace), npm distribution, auth |
| 2 | 0.2 | Dynamic BigQuery API, `generate-skills`, Model Armor, Gemini manifest |
| 3 | 0.3 | `ca ask/create-agent`, remaining analytics commands, shell completions |
| 4 | 0.4 | Multi-source CA (6 sources), source profiles, QueryData integration |
| 5 | 0.5 | Native Data Cloud commands (Spanner, AlloyDB, Cloud SQL, Looker), SDK alignment, skill consolidation (14 skills), 615 tests |
| 6 | 0.5 | `meta commands/describe`, MCP bridge (`dcx mcp serve`), agent contract hardening |

See [PHASE5_PLAN.md](PHASE5_PLAN.md) and [PHASE6_PLAN.md](PHASE6_PLAN.md)
for detailed plans.

---

## Release

Tag `vX.Y.Z` on `main` triggers CI to build binaries, run smoke tests,
and publish npm packages.

## Relationship to Existing Tools

| Tool | Relationship |
|------|-------------|
| `bq` CLI | dcx is a successor, not a wrapper. Coexists — users migrate gradually. |
| `gcloud` | dcx handles Data Cloud workflows; delegates IAM/projects to `gcloud`. |
| `gws` CLI | Architectural template. Same skills format, different domain. |
