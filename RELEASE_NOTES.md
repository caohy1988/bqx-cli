# Release Notes

## v0.5.0 — Native Data Cloud Commands + SDK Alignment (2026-03-30)

### Highlights

`dcx` is now a **direct Data Cloud operations CLI**. In addition to
Conversational Analytics, users and agents can run first-class,
source-native commands for Looker, Spanner, AlloyDB, and Cloud SQL
without dropping to `gcloud` or ad-hoc wrappers.

**Analytics SDK alignment milestones A–E are complete.** All 12 SDK
CLI commands are present, all 6 code evaluators implemented, and exit
codes match SDK semantics. Remaining intentional divergences (e.g.
`llm-judge` not yet functional, warning-only flags) are documented in
the generated compatibility contract. Automated drift detection ensures
the two surfaces do not silently diverge.

### New: Direct Source Commands

All generated from bundled Google Discovery documents using the same
dynamic pipeline as BigQuery:

| Service | Commands |
|---------|----------|
| Spanner | `dcx spanner instances list\|get`, `databases list\|get\|get-ddl` |
| AlloyDB | `dcx alloydb clusters list\|get`, `instances list\|get` |
| Cloud SQL | `dcx cloudsql instances list\|get`, `databases list\|get` |
| Looker (admin) | `dcx looker instances list\|get`, `backups list\|get` |

### New: Profile-Aware Schema and Database Helpers

```bash
dcx spanner schema describe --profile spanner-finance.yaml --format table
dcx alloydb schema describe --profile alloydb-ops.yaml --format json
dcx alloydb databases list --profile alloydb-ops.yaml --format text
dcx cloudsql schema describe --profile cloudsql-app.yaml --format table
```

These use CA QueryData under the hood, routed by the profile's source type.

### New: Looker Content Commands

Hand-written client for per-instance Looker API:

```bash
dcx looker explores list --profile looker-sales.yaml --format json
dcx looker dashboards list --profile looker-sales.yaml --format json
```

### New: Profile Utilities

```bash
dcx profiles list --format table
dcx profiles show --profile my-profile --format json
dcx profiles validate --profile my-profile --format text
```

### Skill Consolidation (39 → 14)

The skill layer was redesigned per agent-skills best practices:

| Type | Count | Skills |
|------|-------|--------|
| Router | 6 | `dcx-bigquery`, `dcx-analytics`, `dcx-ca`, `dcx-databases`, `dcx-looker`, `dcx-profiles` |
| API | 5 | `dcx-bigquery-api`, `dcx-spanner-api`, `dcx-alloydb-api`, `dcx-cloudsql-api`, `dcx-looker-admin-api` |
| Recipe | 3 | `recipe-source-onboarding`, `recipe-debugging`, `recipe-quality-ops` |

Router skills are routing-focused (~1 page each) with command detail in
`references/` subdirectories. API skills are service-level (one per
Discovery namespace). Recipes are consolidated workflows.

### Gemini Manifest

Expanded from 17 → 28 tools, adding Spanner, AlloyDB, Cloud SQL, Looker
admin, and profiles tools.

### Architecture

- `ServiceConfig` abstraction: per-service config for Discovery-driven
  commands (namespace, allowlist, global param mapping, flatPath preference)
- Looker hybrid: GCP admin API (Discovery) + per-instance content API
  (hand-written), unified under `dcx looker`
- Location normalization: AlloyDB and Looker "US" default → "-" (all locations)
- Profile-aware helpers validate source type compatibility before auth

### Source Matrix

See `docs/source-matrix.md` for the full cross-source command matrix.

### New: Analytics SDK Alignment (Milestones A–E)

All 12 SDK CLI commands present; remaining intentional divergences documented:

- **Contract generator** — `scripts/parse_sdk_cli.py` parses upstream
  `cli.py` and `SDK.md`, generates `analytics_sdk_contract.json` and
  `analytics_sdk_contract.md` with per-command/flag parity status
- **Command parity** — all 12 SDK CLI commands present (`views create`,
  `categorical-eval`, `categorical-views` added)
- **Flag/evaluator parity** — all 6 code evaluators (latency, error-rate,
  turn-count, token-efficiency, ttft, cost), `--limit` in SQL, runtime
  warnings for placeholder flags (`--criterion`, `--strict`, `--mode`,
  `--top-k`), session-id validation against SQL injection
- **Output/exit-code parity** — exit codes match SDK (0=success, 1=eval
  failure, 2=infra error), output-key regression tests for all result structs
- **Drift automation** — CI `contract-check` job detects stale contracts,
  weekly `sdk-sync` workflow fetches upstream changes and opens PRs

### Test Count

513 tests (was 459 in v0.4.0)

---

## v0.4.0 — Data Cloud CA (2026-03-23)

### Highlights

`dcx` is now an **agentic Data Cloud CLI**. Conversational Analytics support
has been broadened from BigQuery-only to 6 data sources across two API
families.

### New: Multi-Source Conversational Analytics

`dcx ca ask` now supports all official CA data sources through a unified
`--profile` flag:

| Source | API Family | Example |
|--------|-----------|---------|
| BigQuery | Chat / DataAgent | `dcx ca ask --agent my-agent "error rate?"` |
| Looker | Chat / DataAgent | `dcx ca ask --profile sales-looker.yaml "top products?"` |
| Looker Studio | Chat / DataAgent | `dcx ca ask --profile studio.yaml "monthly trend?"` |
| AlloyDB | QueryData | `dcx ca ask --profile ops-alloydb.yaml "active connections?"` |
| Spanner | QueryData | `dcx ca ask --profile finance-spanner.yaml "revenue by region?"` |
| Cloud SQL | QueryData | `dcx ca ask --profile app-cloudsql.yaml "show tables"` |

The profile's `source_type` determines which API family is used. Users and
agents interact through the same `ca ask` command regardless of source.

### New: Source Profile Model

A `CaProfile` system with YAML-based source profiles. Each profile captures
connection details for a specific data source:

```yaml
name: finance-spanner
source_type: spanner
project: my-gcp-project
location: us-central1
instance_id: my-instance
database_id: my-database
```

Supported source types: `bigquery`, `looker`, `looker_studio`, `alloy_db`,
`spanner`, `cloud_sql`.

### New: 6 Data Cloud Skills

- `dcx-ca-looker` — Looker explore profile setup and CA usage
- `dcx-ca-database` — Database source routing (AlloyDB, Spanner, Cloud SQL)
- `dcx-ca-alloydb` — AlloyDB prerequisites and troubleshooting
- `dcx-ca-spanner` — Spanner GoogleSQL patterns and business queries
- `recipe-ca-looker-exploration` — Step-by-step Looker CA setup recipe
- `recipe-ca-database-ops` — Step-by-step database CA recipe

### Updated Skills

- `dcx-ca` — Broadened from BigQuery-only to 6-source Data Cloud routing
- `dcx-ca-ask` — Added `--profile` flag docs and multi-source examples
- `persona-sre` — Added cross-source investigation workflow with profiles

### Architecture

- `context_set_id` is now optional for all database sources
- QueryData API uses `geminidataanalytics.googleapis.com` endpoint
- `--profile` and `--agent` flags are mutually exclusive
- Chat/DataAgent vs QueryData split modeled honestly in code, normalized
  at the CLI boundary

### Skill Count

32 total skills (was 26 in v0.3.0):
- 1 shared, 7 service, 6 helper, 7 CA, 3 persona, 8 recipe

### E2E Validation

All sources validated against live GCP instances in
`test-project-0728-467323`:
- Spanner: math, schema, business queries
- AlloyDB: math, schema, operational queries
- Cloud SQL: math, schema queries
- All output formats (json, text, table) verified
- Conflict guards (`--profile` + `--agent`) verified

---

## v0.3.0 — Conversational Analytics + Polish (2026-03-14)

- `dcx ca ask` — natural language queries via CA API (BigQuery)
- `dcx ca create-agent` — create BigQuery data agents
- `dcx ca add-verified-query` — add verified queries
- Remaining analytics commands: `insights`, `drift`, `distribution`,
  `views`, `hitl-metrics`, `list-traces`
- Shell completions (bash, zsh, fish)
- 26 skills total

## v0.2.0 — Dynamic BigQuery API + Skills (2026-03-10)

- Dynamic `clap::Command` tree from BigQuery v2 Discovery Document
- `dcx generate-skills` command
- 19 skills (4 generated, 15 curated)
- Model Armor integration (`--sanitize`)
- Gemini CLI extension manifest

## v0.1.0 — Core CLI + Analytics (2026-03-08)

- Rust CLI with `clap`, auth, `--format`, `--exit-code`
- `dcx analytics`: `doctor`, `evaluate`, `get-trace`
- npm distribution (`npx dcx`)
- 5 core skills
