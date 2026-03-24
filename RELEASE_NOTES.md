# Release Notes

## v0.4.0 — Data Cloud CA (2026-03-23)

### Highlights

`bqx` is now an **agentic Data Cloud CLI**. Conversational Analytics support
has been broadened from BigQuery-only to 6 data sources across two API
families.

### New: Multi-Source Conversational Analytics

`bqx ca ask` now supports all official CA data sources through a unified
`--profile` flag:

| Source | API Family | Example |
|--------|-----------|---------|
| BigQuery | Chat / DataAgent | `bqx ca ask --agent my-agent "error rate?"` |
| Looker | Chat / DataAgent | `bqx ca ask --profile sales-looker.yaml "top products?"` |
| Looker Studio | Chat / DataAgent | `bqx ca ask --profile studio.yaml "monthly trend?"` |
| AlloyDB | QueryData | `bqx ca ask --profile ops-alloydb.yaml "active connections?"` |
| Spanner | QueryData | `bqx ca ask --profile finance-spanner.yaml "revenue by region?"` |
| Cloud SQL | QueryData | `bqx ca ask --profile app-cloudsql.yaml "show tables"` |

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

- `bqx-ca-looker` — Looker explore profile setup and CA usage
- `bqx-ca-database` — Database source routing (AlloyDB, Spanner, Cloud SQL)
- `bqx-ca-alloydb` — AlloyDB prerequisites and troubleshooting
- `bqx-ca-spanner` — Spanner GoogleSQL patterns and business queries
- `recipe-ca-looker-exploration` — Step-by-step Looker CA setup recipe
- `recipe-ca-database-ops` — Step-by-step database CA recipe

### Updated Skills

- `bqx-ca` — Broadened from BigQuery-only to 6-source Data Cloud routing
- `bqx-ca-ask` — Added `--profile` flag docs and multi-source examples
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

- `bqx ca ask` — natural language queries via CA API (BigQuery)
- `bqx ca create-agent` — create BigQuery data agents
- `bqx ca add-verified-query` — add verified queries
- Remaining analytics commands: `insights`, `drift`, `distribution`,
  `views`, `hitl-metrics`, `list-traces`
- Shell completions (bash, zsh, fish)
- 26 skills total

## v0.2.0 — Dynamic BigQuery API + Skills (2026-03-10)

- Dynamic `clap::Command` tree from BigQuery v2 Discovery Document
- `bqx generate-skills` command
- 19 skills (4 generated, 15 curated)
- Model Armor integration (`--sanitize`)
- Gemini CLI extension manifest

## v0.1.0 — Core CLI + Analytics (2026-03-08)

- Rust CLI with `clap`, auth, `--format`, `--exit-code`
- `bqx analytics`: `doctor`, `evaluate`, `get-trace`
- npm distribution (`npx bqx`)
- 5 core skills
