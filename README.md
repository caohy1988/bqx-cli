# Proposal: `dcx` — An Agent-Native Data Cloud CLI with Skills

**Status:** Proposal
**Date:** 2026-03-08
**Related:** [gws CLI](https://github.com/googleworkspace/cli),
[Agent Skills](https://agentskills.io),
[Conversational Analytics API](https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/overview),
[BigQuery Agent Analytics SDK](https://github.com/haiyuan-eng-google/BigQuery-Agent-Analytics-SDK)

For a detailed technical comparison with the standard `bq` CLI, including
demoable workflow examples, see [DCX vs BQ](docs/dcx-vs-bq.md).

---

## Quick Start

### Install via npm

```bash
npx dcx --help
```

Or install globally:

```bash
npm install -g dcx
dcx --help
```

### Supported Platforms

| Platform | Package |
|---|---|
| macOS ARM64 (Apple Silicon) | `@dcx-cli/darwin-arm64` |
| macOS x64 (Intel) | `@dcx-cli/darwin-x64` |
| Linux x64 | `@dcx-cli/linux-x64` |
| Linux ARM64 | `@dcx-cli/linux-arm64` |
| Windows x64 | `@dcx-cli/win32-x64` |
| Windows ARM64 | `@dcx-cli/win32-arm64` |

### GitHub Actions

```yaml
- run: npm install -g dcx
- run: dcx analytics evaluate --evaluator latency --threshold 5000 --last 1h --exit-code
```

See [docs/github-actions.md](docs/github-actions.md) for full CI examples.

### Release

Tag `vX.Y.Z` on `main` triggers CI to build binaries, run smoke tests, and publish npm packages.

---

## 1. Problem Statement

BigQuery is the most common data platform for AI agent analytics, but its
CLI tooling (`bq`) was designed in 2012 for human operators. It is:

- **Not extensible** — monolithic Python binary, no plugin or skill system
- **Not agent-friendly** — inconsistent output formats, no structured JSON
  default, verbose help text that wastes context tokens
- **Not AI-aware** — no integration with Conversational Analytics, AI
  functions, or agent evaluation workflows

Meanwhile, AI agents are becoming the primary consumers of CLI tools.
Early benchmarks suggest CLIs can be significantly more token-efficient
than MCP schemas and achieve higher task completion rates for identical
tasks ([CLI vs MCP benchmarks](https://github.com/anthropics/claude-code/wiki/cli-vs-mcp-benchmarks)).
But agents need CLIs designed for them: structured output, progressive
disclosure, and discoverable skills.

The Google Workspace CLI ([`gws`](https://github.com/googleworkspace/cli))
has proven this model works for a different Google domain — it dynamically
generates commands from Workspace API Discovery Documents, ships 100+
declarative skills, defaults to JSON output, and has been adopted by
Claude Code, Gemini CLI, Cursor, and others. BigQuery needs the same
agent-first treatment.

---

## 2. Proposal: `dcx` (BigQuery Extended → Data Cloud CLI)

A new agent-native CLI for Google Cloud's Data Cloud that combines:

1. **Dynamic command generation** from Google Cloud Discovery APIs (like `gws`)
   — BigQuery, Spanner, AlloyDB, and Cloud SQL
2. **Agent Skills** for discoverability (SKILL.md format)
3. **Conversational Analytics** integration across 6 data sources —
   BigQuery, Looker, Looker Studio, AlloyDB, Spanner, and Cloud SQL
4. **BigQuery Agent Analytics SDK** capabilities (evaluation, traces, drift)

```
┌────────────────────────────────────────────────────────────────────────┐
│                              dcx CLI                                   │
│                                                                        │
│  ┌──────────────────────┐  ┌───────────────┐  ┌────────────────────┐  │
│  │ Data Cloud APIs       │  │ Agent         │  │ Conversational     │  │
│  │ (Discovery-driven)    │  │ Analytics SDK │  │ Analytics API      │  │
│  │                       │  │               │  │                    │  │
│  │ BigQuery  (top-level) │  │ evaluate,     │  │ ask (6 sources),   │  │
│  │ Spanner   (namespaced)│  │ get-trace,    │  │ create-agent,      │  │
│  │ AlloyDB   (namespaced)│  │ drift,        │  │ list-agents        │  │
│  │ Cloud SQL (namespaced)│  │ insights      │  │                    │  │
│  │ Looker    (namespaced)│  │               │  │                    │  │
│  └──────────┬────────────┘  └───────┬───────┘  └─────────┬──────────┘ │
│             │                       │                     │            │
│  ┌──────────┴───────────────────────┴─────────────────────┴─────────┐  │
│  │                      Shared Core                                  │  │
│  │  Auth · JSON output · Model Armor · Pagination · Profiles         │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │                     Skills (SKILL.md)                              │  │
│  │  32 skills (4 generated + 28 curated; see §4.1)                    │  │
│  │  1 shared · 7 service · 6 helper · 7 CA · 3 persona · 8 recipe    │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │               Looker Content (per-instance API)                        │  │
│  │  explores list|get · dashboards list|get                          │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                        │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │               CA Source Matrix                                    │  │
│  │  Chat/DataAgent: BigQuery · Looker · Looker Studio                │  │
│  │  QueryData:      AlloyDB  · Spanner · Cloud SQL                   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└────────────────────────────────────────────────────────────────────────┘
```

### Why `dcx`, not extending `bq`

| Factor | `bq` (existing) | `dcx` (proposed) |
|--------|-----------------|-------------------|
| Language | Python | Rust (fast startup, single binary) |
| Extensibility | None | Skills + dynamic command generation |
| Output format | Mixed text/JSON | JSON-first (+ table, text) |
| Agent consumption | Not designed for agents | Progressive disclosure, SKILL.md |
| Release cycle | Coupled to gcloud SDK | Independent releases |
| AI integration | None | Conversational Analytics, AI functions, Agent Analytics |
| Discovery | Static commands | Dynamic from Google Cloud Discovery APIs |

---

## 3. Architecture

### 3.1 Dynamic Command Generation (from `gws` pattern)

Like `gws`, `dcx` uses two-phase argument parsing:

1. `argv[1]` identifies the service module (`analytics`, `ca`, `looker`, or
   falls through to dynamic resource names)
2. For API commands, the binary loads bundled
   [Discovery Documents](https://www.googleapis.com/discovery/v1/apis/)
   for each service, builds a `clap::Command` tree dynamically, and routes
   to a shared HTTP executor

**Multi-service dynamic generation:** The same Discovery-driven pipeline
serves five Google Cloud services:

| Service | Namespace | Discovery Doc | Methods |
|---------|-----------|---------------|---------|
| BigQuery | _(top-level)_ | `bigquery/v2` | 8 (datasets, tables, routines, models) |
| Spanner | `spanner` | `spanner/v1` | 5 (instances, databases, getDdl) |
| AlloyDB | `alloydb` | `alloydb/v1` | 4 (clusters, instances) |
| Cloud SQL | `cloudsql` | `sqladmin/v1` | 4 (instances, databases) |
| Looker | `looker` | `looker/v1` | 4 (instances, backups) |

The `ServiceConfig` abstraction in `src/bigquery/dynamic/service.rs` holds
per-service configuration: namespace, allowlist, global param mapping,
bundled JSON, and flatPath preference. BigQuery commands are top-level
(`dcx datasets list`); other services are namespaced (`dcx spanner
instances list`).

**Offline / CI resilience:** The binary ships with pinned copies of all
five Discovery Documents (committed at build time via `include_str!`).
No runtime fetch. This ensures deterministic builds, reproducible CI,
and no network dependency. The bundled documents are updated intentionally
and reviewed like vendored API input.

**Read-only allowlists:** Dynamic commands are restricted to read-only
allowlists per service. Write/mutation methods are excluded. The
allowlists are defined in `src/bigquery/dynamic/service.rs`.

```bash
# Dynamic commands — BigQuery (top-level, generated from Discovery)
dcx datasets list --project-id=myproject
dcx tables get --project-id=myproject --dataset-id=analytics --table-id=agent_events

# Dynamic commands — Spanner (namespaced, generated from Discovery)
dcx spanner instances list --project-id=myproject
dcx spanner databases get-ddl --project-id=myproject --instance-id=my-inst --database-id=mydb

# Dynamic commands — AlloyDB (namespaced, generated from Discovery)
dcx alloydb clusters list --project-id=myproject
dcx alloydb instances list --project-id=myproject --cluster-id=my-cluster --location=us-central1

# Dynamic commands — Cloud SQL (namespaced, generated from Discovery)
dcx cloudsql instances list --project-id=myproject
dcx cloudsql databases list --project-id=myproject --instance=my-inst

# Static commands (Agent Analytics SDK — compiled in)
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h
dcx analytics get-trace --session-id=sess-001
dcx analytics drift --golden-dataset=golden_qs

# Static commands (Conversational Analytics API)
dcx ca ask "What were the top errors yesterday?" --agent=my-data-agent
dcx ca create-agent --name=agent-analytics --tables=agent_events

# Static commands (Looker — hand-written, not Discovery)
dcx looker explores list --profile=sales-looker
dcx looker dashboards get --profile=sales-looker --dashboard-id=42
```

### 3.2 Five Command Domains

#### Domain 1: `dcx <resource> <method>` — BigQuery API (dynamic)

Generated from the BigQuery v2 Discovery Document, covering datasets,
tables, routines, and models.

```bash
# List datasets
dcx datasets list --project-id=myproject

# Show table schema
dcx tables get --project-id=myproject --dataset-id=analytics --table-id=agent_events
```

#### Domain 1b: `dcx <service> <resource> <method>` — Data Cloud APIs (dynamic)

Generated from bundled Discovery Documents for Spanner (`spanner/v1`),
AlloyDB (`alloydb/v1`), and Cloud SQL (`sqladmin/v1`). Same pipeline as
BigQuery — one `ServiceConfig` per service, shared executor.

```bash
# Spanner
dcx spanner instances list --project-id=myproject
dcx spanner databases list --project-id=myproject --instance-id=my-inst
dcx spanner databases get-ddl --project-id=myproject --instance-id=my-inst --database-id=mydb

# AlloyDB (--location defaults to all regions)
dcx alloydb clusters list --project-id=myproject
dcx alloydb instances list --project-id=myproject --cluster-id=my-cluster --location=us-central1

# Cloud SQL
dcx cloudsql instances list --project-id=myproject
dcx cloudsql instances get --project-id=myproject --instance=my-inst
dcx cloudsql databases list --project-id=myproject --instance=my-inst
```

**Profile-aware helpers (M4):** Schema and database helpers use CA
QueryData under the hood, routed by source profile. They validate
profile/source compatibility before auth or network.

```bash
# Spanner: describe schema columns via profile
dcx spanner schema describe --profile=spanner-finance

# Cloud SQL: describe schema columns via profile
dcx cloudsql schema describe --profile=cloudsql-app

# AlloyDB: describe schema columns via profile
dcx alloydb schema describe --profile=alloydb-ops

# AlloyDB: list databases via profile (no Discovery equivalent)
dcx alloydb databases list --profile=alloydb-ops
```

#### Domain 1c: `dcx looker <resource> <method>` — Looker API (hybrid)

Looker has two APIs: (1) the GCP admin API (`looker.googleapis.com`) for
managing Looker *instances* — this has a Discovery document and is
handled by the dynamic pipeline; (2) the per-instance Looker API
(`https://<instance>.cloud.looker.com/api/4.0/`) for *content* like
explores and dashboards — this is hand-written and profile-driven.

```bash
# Discovery-driven: Looker instance management (GCP admin API)
dcx looker instances list --project-id=myproject --location=us-central1
dcx looker instances get --project-id=myproject --location=us-central1 --instance-id=my-looker
dcx looker backups list --project-id=myproject --location=us-central1 --instance-id=my-looker

# Hand-written: Looker content (per-instance API, profile-driven)
dcx looker explores list --profile=sales-looker
dcx looker explores get --profile=sales-looker --explore=model/explore
dcx looker dashboards list --profile=sales-looker
dcx looker dashboards get --profile=sales-looker --dashboard-id=42
```

#### Domain 2: `dcx analytics <command>` — Agent Analytics (static)

Wraps the BigQuery Agent Analytics SDK. Commands are compiled into the
binary (not dynamically generated) since they don't come from a Discovery
Document.

```bash
# Evaluate agent performance
dcx analytics evaluate \
  --evaluator=latency \
  --threshold=5000 \
  --agent-id=support_bot \
  --last=1h

# Retrieve a session trace
dcx analytics get-trace --session-id=sess-001

# Health check
dcx analytics doctor

# Drift detection
dcx analytics drift \
  --golden-dataset=golden_questions \
  --agent-id=support_bot \
  --last=7d

# LLM-as-judge evaluation
dcx analytics evaluate \
  --evaluator=llm-judge \
  --criterion=correctness \
  --threshold=0.7 \
  --last=24h \
  --exit-code

# Create event-type views
dcx analytics views create-all --prefix=adk_

# Generate insights report
dcx analytics insights --agent-id=support_bot --last=24h
```

#### Domain 3: `dcx ca <command>` — Conversational Analytics (static)

Wraps the Conversational Analytics API, bringing natural language queries
to the terminal across 6 data sources.

**Supported sources:**

| Source | API Family | Access Method |
|--------|-----------|---------------|
| BigQuery | Chat / DataAgent | `--agent` or `--tables` flags |
| Looker | Chat / DataAgent | `--profile` with explore references |
| Looker Studio | Chat / DataAgent | `--profile` with datasource references |
| AlloyDB | QueryData | `--profile` with database connection |
| Spanner | QueryData | `--profile` with instance/database |
| Cloud SQL | QueryData | `--profile` with instance/database |

```bash
# BigQuery: ask via data agent
dcx ca ask "Show me the top 5 agents by error rate this week" \
  --agent=agent-analytics-data-agent

# BigQuery: ask with inline table context
dcx ca ask "What's the p95 latency trend for support_bot?" \
  --tables=myproject.analytics.agent_events

# Looker: ask against explore profiles
dcx ca ask --profile sales-looker.yaml "What are the top selling products?"

# AlloyDB: operational queries via database profiles
dcx ca ask --profile ops-alloydb.yaml "show active connections"

# Spanner: business queries via database profiles
dcx ca ask --profile finance-spanner.yaml "total revenue by region"

# Cloud SQL: query via database profiles
dcx ca ask --profile app-cloudsql.yaml "show all tables"

# Create a BigQuery data agent with verified queries
dcx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events,myproject.analytics.adk_llm_response \
  --verified-queries=./deploy/ca/verified_queries.yaml \
  --instructions="This agent helps analyze AI agent performance metrics."

# List data agents
dcx ca list-agents --project-id=myproject
```

### 3.3 Output Format

All output is JSON by default, with alternative formats via `--format`:

```bash
# Default: structured JSON (agent-consumable)
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h
{
  "evaluator": "latency",
  "threshold_ms": 5000,
  "total_sessions": 10,
  "passed": 7,
  "failed": 3,
  "pass_rate": 0.70,
  "aggregate_scores": {
    "avg_latency_ms": 3200,
    "p95_latency_ms": 6100
  }
}

# Table format (human-readable)
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --format=table
SESSION_ID   PASSED  LATENCY_MS  SCORE
sess-001     true    2340        0.85
sess-002     false   7800        0.32
sess-003     true    1850        0.91

# Dry-run mode (shows what would happen)
dcx jobs query --query="SELECT 1" --dry-run
{
  "dry_run": true,
  "url": "https://bigquery.googleapis.com/bigquery/v2/projects/myproject/queries",
  "method": "POST",
  "body": {"query": "SELECT 1", "useLegacySql": false},
  "estimated_bytes_processed": 0
}
```

### 3.4 Authentication

Five methods, same priority model as `gws`:

| Priority | Method | Use Case |
|----------|--------|----------|
| 1 (highest) | `DCX_TOKEN` env var | Pre-obtained access token |
| 2 | `DCX_CREDENTIALS_FILE` env var | Service account JSON path |
| 3 | `dcx auth login` (encrypted) | Interactive OAuth, AES-256-GCM at rest |
| 4 | `GOOGLE_APPLICATION_CREDENTIALS` | Standard ADC fallback |
| 5 | `gcloud auth application-default` | Implicit gcloud credentials |

```bash
# Quick start (uses existing gcloud credentials)
dcx datasets list --project-id=myproject

# Explicit login (default: BigQuery-only scopes)
dcx auth login

# Override scopes (-s replaces the default scope set, not additive)
dcx auth login -s bigquery,cloud-platform

# Service account (CI/CD)
export DCX_CREDENTIALS_FILE=/path/to/sa-key.json
dcx analytics evaluate --evaluator=latency --last=24h --exit-code
```

### 3.5 Security

- **Model Armor integration:** `--sanitize <template>` screens API responses
  through [Model Armor](https://cloud.google.com/security-command-center/docs/model-armor-overview)
  for prompt injection and malicious content. Flagged responses are redacted
  before reaching stdout; a notice is printed to stderr. Set
  `DCX_SANITIZE_TEMPLATE` env var for global default.
  ```bash
  # Screen a query response through Model Armor
  dcx jobs query --query "SELECT * FROM my_table" \
    --sanitize projects/my-proj/locations/us-central1/templates/my-template

  # Set globally for all commands
  export DCX_SANITIZE_TEMPLATE=projects/my-proj/locations/us-central1/templates/my-template
  dcx datasets list --project-id=myproject
  ```
  **Note:** Model Armor requires regional endpoints. The location is
  extracted automatically from the template resource name.
- **Credential encryption:** AES-256-GCM at rest, key in OS keyring.
- **Destructive operation guards:** Write/delete commands require `--confirm`
  flag or interactive confirmation. Skill generator blocks destructive
  methods by default.
- **Least-privilege defaults:** `dcx auth login` requests only BigQuery
  scopes by default. `-s` *replaces* the default scope set (it does not
  append), so users must opt in explicitly to broader scopes like
  `cloud-platform`.

---

## 4. Skills Architecture

### 4.1 Overview

Skills follow the [Agent Skills](https://agentskills.io) open standard:
declarative `SKILL.md` files that any compatible agent (Claude Code, Gemini
CLI, Cursor, Copilot, Codex) can discover and use.

```
skills/
│ ## Shared
├── dcx-shared/SKILL.md                       # Curated: auth, global flags, security rules
│
│ ## Service skills — generated from BigQuery v2 Discovery API
├── dcx-datasets/SKILL.md                     # Generated: dataset list/get
├── dcx-tables/SKILL.md                       # Generated: table list/get
├── dcx-routines/SKILL.md                     # Generated: routine list/get
├── dcx-models/SKILL.md                       # Generated: ML model list/get
│
│ ## Service skills — curated (static commands or non-Discovery APIs)
├── dcx-jobs/SKILL.md                         # Curated: query execution (static command)
├── dcx-connections/SKILL.md                  # Curated: external connections (via INFORMATION_SCHEMA)
├── dcx-analytics/SKILL.md                    # Curated: Agent Analytics SDK
│
│ ## Helper skills — curated
├── dcx-analytics-evaluate/SKILL.md           # Curated: run evaluations
├── dcx-analytics-trace/SKILL.md              # Curated: retrieve traces
├── dcx-analytics-drift/SKILL.md              # Curated: drift detection workflow
├── dcx-analytics-views/SKILL.md              # Curated: manage event views
├── dcx-query/SKILL.md                        # Curated: shortcut for `dcx jobs query`
├── dcx-schema/SKILL.md                       # Curated: inspect table schemas
│
│ ## CA skills — multi-source (Phase 3 + Phase 4)
├── dcx-ca/SKILL.md                           # Routing: CA entry point, source selection
├── dcx-ca-ask/SKILL.md                       # Helper: ask questions across all sources
├── dcx-ca-create-agent/SKILL.md              # Helper: create BigQuery data agents
├── dcx-ca-looker/SKILL.md                    # Phase 4: Looker explore CA profiles
├── dcx-ca-database/SKILL.md                  # Phase 4: database source routing
├── dcx-ca-alloydb/SKILL.md                   # Phase 4: AlloyDB prerequisites + CA
├── dcx-ca-spanner/SKILL.md                   # Phase 4: Spanner GoogleSQL CA patterns
│
│ ## Personas — curated
├── persona-agent-developer/SKILL.md          # Curated: agent developer workflows
├── persona-data-analyst/SKILL.md             # Curated: SQL analyst workflows
├── persona-sre/SKILL.md                      # SRE/on-call with cross-source CA
│
│ ## Recipes — curated
├── recipe-eval-pipeline/SKILL.md             # Curated: CI/CD eval gate setup
├── recipe-quality-dashboard/SKILL.md         # Curated: dashboard via BigQuery views
├── recipe-drift-monitoring/SKILL.md          # Curated: weekly drift detection
├── recipe-error-alerting/SKILL.md            # Phase 3: CQ + AI.GENERATE_TEXT alerting
├── recipe-self-diagnostic-agent/SKILL.md     # Phase 3: agent self-correction loop
├── recipe-ca-data-agent-setup/SKILL.md       # Phase 3: CA data agent creation
├── recipe-ca-looker-exploration/SKILL.md     # Phase 4: Looker CA exploration workflow
└── recipe-ca-database-ops/SKILL.md           # Phase 4: database CA ops workflow
```

### 4.2 Example Skills

#### Service Skill: `dcx-analytics/SKILL.md`

```markdown
---
name: dcx-analytics
version: 1.0.0
description: "BigQuery Agent Analytics: Evaluate, trace, and monitor AI agent sessions."
metadata:
  category: "analytics"
  requires:
    bins: ["dcx"]
  cliHelp: "dcx analytics --help"
---

# analytics

> **PREREQUISITE:** Read `../dcx-shared/SKILL.md` for auth, global flags,
> and security rules.

```bash
dcx analytics <command> [flags]
```

## Commands

| Command | Description |
|---------|-------------|
| `doctor` | Run diagnostic health check on BigQuery table and configuration |
| `evaluate` | Run code-based or LLM evaluation over agent session traces |
| `get-trace` | Retrieve and display a single session trace |
| `list-traces` | List recent traces matching filter criteria |
| `insights` | Generate comprehensive agent insights report |
| `drift` | Run drift detection against a golden question set |
| `distribution` | Analyze question distribution patterns |
| `hitl-metrics` | Show human-in-the-loop interaction metrics |
| `views` | Create per-event-type BigQuery views (18 event types) |

## Helper Skills

For common tasks, use the shortcut helper skills:

| Helper | Description |
|--------|-------------|
| [`dcx-analytics-evaluate`](../dcx-analytics-evaluate/SKILL.md) | Quick evaluation commands |
| [`dcx-analytics-trace`](../dcx-analytics-trace/SKILL.md) | Trace retrieval and analysis |
| [`dcx-analytics-drift`](../dcx-analytics-drift/SKILL.md) | Drift detection workflows |
| [`dcx-analytics-views`](../dcx-analytics-views/SKILL.md) | Manage per-event-type views |

## Global Flags

| Flag | Description |
|------|-------------|
| `--project-id TEXT` | GCP project ID [env: `DCX_PROJECT`] |
| `--dataset-id TEXT` | BigQuery dataset [env: `DCX_DATASET`] |
| `--last TEXT` | Time window: `1h`, `24h`, `7d`, `30d` |
| `--agent-id TEXT` | Filter by agent name |
| `--format TEXT` | Output: `json` (default), `table`, `text` |
| `--exit-code` | Return exit code 1 on evaluation failure |
```

#### Helper Skill: `dcx-analytics-evaluate/SKILL.md`

```markdown
---
name: dcx-analytics-evaluate
version: 1.0.0
description: "Evaluate AI agent sessions for latency, error rate, or correctness."
metadata:
  category: "analytics"
  requires:
    bins: ["dcx"]
  cliHelp: "dcx analytics evaluate --help"
---

# analytics evaluate

> **PREREQUISITE:** Read `../dcx-shared/SKILL.md` for auth and global flags.

Evaluate agent sessions against a threshold. Returns pass/fail per session.

## Usage

```bash
dcx analytics evaluate --evaluator=<TYPE> --threshold=<N> [flags]
```

## Flags

| Flag | Required | Default | Description |
|------|----------|---------|-------------|
| `--evaluator` | Yes | — | `latency`, `error_rate`, `turn_count`, `token_efficiency`, `llm-judge` |
| `--threshold` | Yes | — | Pass/fail threshold (ms for latency, 0-1 for rates/scores) |
| `--criterion` | If llm-judge | — | `correctness`, `hallucination`, `sentiment`, `custom` |
| `--custom-prompt` | If custom | — | Custom LLM judge prompt |
| `--exit-code` | No | false | Return exit code 1 on failure (for CI/CD) |

## Examples

```bash
# Check latency compliance (agent self-diagnostic)
dcx analytics evaluate --evaluator=latency --threshold=5000 --agent-id=support_bot --last=1h

# CI/CD gate: fail if correctness drops below 0.7
dcx analytics evaluate --evaluator=llm-judge --criterion=correctness \
  --threshold=0.7 --last=24h --exit-code

# Custom evaluation
dcx analytics evaluate --evaluator=llm-judge --criterion=custom \
  --custom-prompt="Rate how well the agent handled PII. Score 0-1." \
  --threshold=0.9 --last=24h
```

> [!NOTE]
> This is a **read-only** command. Safe to run without confirmation.
```

#### Persona Skill: `persona-sre/SKILL.md`

```markdown
---
name: persona-sre
version: 1.0.0
description: "On-call SRE workflows for monitoring and triaging AI agent issues."
metadata:
  category: "persona"
  requires:
    bins: ["dcx"]
    skills: ["dcx-analytics", "dcx-ca", "dcx-query"]
---

# SRE / On-Call Engineer

> **PREREQUISITE:** Load the following skills: `dcx-analytics`, `dcx-ca`,
> `dcx-query`

Monitor AI agent health, triage incidents, and validate fixes.

## Incident Triage Workflow

1. Check overall health:
   `dcx analytics doctor`
2. Look for error spikes:
   `dcx analytics evaluate --evaluator=error-rate --threshold=0.05 --last=1h`
3. Identify failing sessions:
   `dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h --format=table`
4. Inspect a specific failure:
   `dcx analytics get-trace --session-id=<ID_FROM_STEP_3>`
5. Ask follow-up in natural language:
   `dcx ca ask "What tools failed most in the last hour?" --agent=agent-analytics`

## Daily Health Check

```bash
dcx analytics doctor && \
dcx analytics evaluate --evaluator=error-rate --threshold=0.05 --last=24h && \
dcx analytics evaluate --evaluator=latency --threshold=5000 --last=24h
```

## Tips

- Use `--format=table` for quick visual scans during incidents.
- Pipe `--format=json` output to `jq` for scripted analysis.
- Set `DCX_PROJECT` and `DCX_DATASET` env vars to avoid repetitive flags.
```

#### Recipe Skill: `recipe-eval-pipeline/SKILL.md`

```markdown
---
name: recipe-eval-pipeline
version: 1.0.0
description: "Set up a CI/CD evaluation pipeline that gates agent deployment on quality metrics."
metadata:
  category: "recipe"
  domain: "devops"
  requires:
    bins: ["dcx"]
    skills: ["dcx-analytics"]
---

# CI/CD Evaluation Pipeline

> **PREREQUISITE:** Load `dcx-analytics` skill.

Set up a GitHub Actions workflow that blocks deployment when agent quality
drops below thresholds.

## Steps

1. Install `dcx` in CI (distributed as platform-specific binaries via npm,
   similar to [`esbuild`](https://github.com/evanw/esbuild) and
   [`turbo`](https://github.com/vercel/turbo)):
   `npm install -g dcx`

2. Authenticate with Workload Identity Federation:
   ```yaml
   - uses: google-github-actions/auth@v2
     with:
       workload_identity_provider: ${{ vars.WIF_PROVIDER }}
       service_account: ${{ vars.SA_EMAIL }}
   ```

3. Add evaluation gates:
   ```bash
   dcx analytics evaluate --evaluator=latency --threshold=5000 --last=24h --exit-code
   dcx analytics evaluate --evaluator=error-rate --threshold=0.05 --last=24h --exit-code
   dcx analytics drift --golden-dataset=golden_qs --min-coverage=0.85 --exit-code
   ```

4. Upload reports as artifacts:
   ```bash
   dcx analytics insights --last=24h > insights.json
   ```

> [!CAUTION]
> Ensure the CI service account has `bigquery.dataViewer` and
> `bigquery.jobUser` roles only. Never grant `dataEditor` to CI.
```

### 4.3 Skill Generation

Like `gws generate-skills`, `dcx` auto-generates skills from the BigQuery
Discovery Document:

```bash
# Generate all skills from BigQuery API commands
dcx generate-skills --output-dir=./skills

# Regenerate only dataset skills
dcx generate-skills --filter=dcx-datasets --output-dir=./skills
```

The generator:
- Uses the bundled BigQuery v2 Discovery Document
- Creates one `SKILL.md` + `agents/openai.yaml` per API resource family
- Only generates skills for methods in the read-only allowlist
- Includes flag tables, usage examples, and cross-references

**Generated vs curated scope:** `generate-skills` produces service skills
for BigQuery API resource families (datasets, tables, routines, models).
Analytics helpers, personas, and recipes are curated by hand — they
require opinionated workflow guidance that raw Discovery metadata cannot
provide.

| Type | Count | Generated? | Examples |
|------|-------|------------|----------|
| Shared | 1 | No | `dcx-shared` |
| Service (API) | 4 | Yes | `dcx-datasets`, `dcx-tables`, `dcx-routines`, `dcx-models` |
| Service (static) | 3 | No | `dcx-jobs`, `dcx-connections`, `dcx-analytics` |
| Helper | 6 | No | `dcx-analytics-evaluate`, `dcx-analytics-trace`, `dcx-analytics-drift`, `dcx-analytics-views`, `dcx-query`, `dcx-schema` |
| CA | 7 | No | `dcx-ca`, `dcx-ca-ask`, `dcx-ca-create-agent`, `dcx-ca-looker`, `dcx-ca-database`, `dcx-ca-alloydb`, `dcx-ca-spanner` |
| Persona | 3 | No | `persona-agent-developer`, `persona-data-analyst`, `persona-sre` |
| Recipe | 8 | No | `recipe-eval-pipeline`, `recipe-ca-data-agent-setup`, `recipe-ca-looker-exploration`, `recipe-ca-database-ops` |

### 4.4 Skill Distribution

```bash
# npm (all skills)
npx skills add https://github.com/bigquery/dcx

# Individual skill
npx skills add https://github.com/bigquery/dcx/tree/main/skills/dcx-analytics

# OpenClaw
ln -s $(pwd)/skills/dcx-* ~/.openclaw/skills/

# Gemini CLI (extension manifest packaged at extensions/gemini/manifest.json)
# Not yet tested with live `gemini extensions install` — spec is evolving
gemini extensions install https://github.com/bigquery/dcx

# Claude Code (auto-discover from project)
# Just clone the repo — Claude Code reads SKILL.md files automatically
```

### 4.5 Shell Completions

```bash
# Bash
dcx completions bash > /usr/local/etc/bash_completion.d/dcx
# or: dcx completions bash >> ~/.bashrc

# Zsh (add to fpath first if needed)
dcx completions zsh > "${fpath[1]}/_dcx"
# or: dcx completions zsh > ~/.zsh/completions/_dcx

# Fish
dcx completions fish > ~/.config/fish/completions/dcx.fish
```

Pre-generated scripts are also available in the `completions/` directory.

---

## 5. Conversational Analytics Integration

### 5.1 Why This Matters

The Conversational Analytics API lets users ask natural language questions
over data in BigQuery, Looker, Looker Studio, AlloyDB, Spanner, and
Cloud SQL. `dcx ca` brings all of these sources to the terminal and to
agents through a unified `ca ask` command.

The API has two families:

| API Family | Sources | Use Case |
|-----------|---------|----------|
| Chat / DataAgent | BigQuery, Looker, Looker Studio | Analytic queries with data agents |
| QueryData | AlloyDB, Spanner, Cloud SQL | Database queries via source profiles |

`dcx ca ask` normalizes both families behind a single command. The
`--profile` flag determines which API path is used based on the source
type.

### 5.2 Data Agent for Agent Analytics

The SDK ships a pre-built data agent configuration with verified queries
tuned for agent analytics:

```bash
# Create the agent-analytics data agent (one-time setup)
dcx ca create-agent \
  --name=agent-analytics \
  --tables=myproject.analytics.agent_events \
  --views=myproject.analytics.adk_llm_response,myproject.analytics.adk_tool_completed \
  --verified-queries=./deploy/ca/verified_queries.yaml \
  --instructions="You help analyze AI agent performance. The agent_events
    table stores traces from ADK agents. Key event types: LLM_REQUEST,
    LLM_RESPONSE, TOOL_STARTING, TOOL_COMPLETED, TOOL_ERROR.
    Error detection: event_type ends with _ERROR OR error_message IS NOT NULL
    OR status = 'ERROR'."
```

#### Verified Queries (shipped with SDK)

```yaml
# deploy/ca/verified_queries.yaml
verified_queries:
  - question: "What is the error rate for {agent}?"
    query: |
      SELECT
        COUNT(CASE WHEN ENDS_WITH(event_type, '_ERROR')
                     OR error_message IS NOT NULL
                     OR status = 'ERROR' THEN 1 END) AS errors,
        COUNT(DISTINCT session_id) AS sessions,
        SAFE_DIVIDE(
          COUNT(CASE WHEN ENDS_WITH(event_type, '_ERROR')
                       OR error_message IS NOT NULL
                       OR status = 'ERROR' THEN 1 END),
          COUNT(DISTINCT session_id)
        ) AS error_rate
      FROM `{project}.{dataset}.agent_events`
      WHERE agent = @agent
        AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)

  - question: "What is the p95 latency for {agent}?"
    query: |
      SELECT
        APPROX_QUANTILES(
          CAST(JSON_VALUE(latency_ms, '$.total_ms') AS INT64), 100
        )[OFFSET(95)] AS p95_latency_ms
      FROM `{project}.{dataset}.agent_events`
      WHERE agent = @agent
        AND event_type = 'LLM_RESPONSE'
        AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)

  - question: "Which tools fail most often?"
    query: |
      SELECT
        JSON_VALUE(content, '$.tool') AS tool_name,
        COUNT(*) AS error_count
      FROM `{project}.{dataset}.agent_events`
      WHERE event_type = 'TOOL_ERROR'
        AND timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 7 DAY)
      GROUP BY tool_name
      ORDER BY error_count DESC
      LIMIT 10

  - question: "Show me the sessions with highest latency"
    query: |
      SELECT
        session_id,
        agent,
        MAX(CAST(JSON_VALUE(latency_ms, '$.total_ms') AS INT64)) AS max_latency_ms,
        COUNT(*) AS event_count,
        MIN(timestamp) AS started_at
      FROM `{project}.{dataset}.agent_events`
      WHERE timestamp >= TIMESTAMP_SUB(CURRENT_TIMESTAMP(), INTERVAL 24 HOUR)
      GROUP BY session_id, agent
      ORDER BY max_latency_ms DESC
      LIMIT 10
```

### 5.3 Usage

```bash
# BigQuery: natural language query via data agent
$ dcx ca ask "What were the top errors for support_bot yesterday?" \
    --agent=agent-analytics

{
  "question": "What were the top errors for support_bot yesterday?",
  "sql": "SELECT JSON_VALUE(content, '$.tool') AS tool, error_message, COUNT(*) ...",
  "results": [
    {"tool": "database_query", "error_message": "Connection timeout", "count": 15},
    {"tool": "search_api", "error_message": "Rate limit exceeded", "count": 8}
  ],
  "explanation": "The most common errors for support_bot in the last 24 hours were..."
}

# Compose with analytics commands
$ dcx ca ask "Which agent had the worst performance today?" --agent=agent-analytics \
  | jq -r '.results[0].agent' \
  | xargs -I{} dcx analytics evaluate --agent-id={} --evaluator=latency --threshold=5000 --last=24h
```

#### Multi-source CA via Profiles

```bash
# Spanner: business queries
$ dcx ca ask --profile finance-spanner.yaml "total revenue by region"

# AlloyDB: operational queries
$ dcx ca ask --profile ops-alloydb.yaml "show active connections"

# Cloud SQL: schema exploration
$ dcx ca ask --profile app-cloudsql.yaml "show all tables"

# Looker: explore-based analytics
$ dcx ca ask --profile sales-looker.yaml "What are the top selling products?"
```

Profile files are YAML with a `source_type` discriminator:

```yaml
# Example: Spanner profile
name: finance-spanner
source_type: spanner
project: my-gcp-project
location: us-central1
instance_id: my-spanner-instance
database_id: my-database
```

See `skills/dcx-ca-database/SKILL.md` and `skills/dcx-ca-looker/SKILL.md`
for source-specific profile formats and prerequisites.

---

## 6. How the Three Domains Compose

The power of `dcx` is that its three domains — BigQuery API, Agent
Analytics, and Conversational Analytics (across all 6 data sources) —
compose through Unix pipes and agent reasoning:

### Scenario: Agent Investigates Its Own Performance

```
Agent thinks: "User asked a complex question. Let me check if I've been
              performing well recently before I commit to an expensive
              tool call."

Step 1: Quick health check
  $ dcx analytics evaluate --evaluator=latency --threshold=5000 --last=1h
  → pass_rate: 0.70 (borderline)

Step 2: Natural language drill-down
  $ dcx ca ask "What's causing high latency in the last hour?" --agent=agent-analytics
  → "The database_query tool has p95 latency of 12s due to 3 timeout events"

Step 3: Check specific trace
  $ dcx analytics get-trace --session-id=sess-042
  → Shows TOOL_ERROR: "Connection timeout after 30s"

Agent decides: Switch to cached data source for this query.
```

### Scenario: SRE Triages an Alert

```bash
# 1. What's the overall health?
dcx analytics doctor

# 2. Which agents are failing?
dcx ca ask "Which agents have error rate above 5% in the last hour?"

# 3. Deep dive into the worst one
dcx analytics evaluate --agent-id=support_bot --evaluator=error-rate --last=1h --format=table

# 4. Get the specific traces
dcx analytics get-trace --session-id=sess-042 --format=tree

# 5. Run raw SQL for custom analysis
dcx jobs query --query="
  SELECT event_type, COUNT(*)
  FROM analytics.agent_events
  WHERE session_id = 'sess-042'
  GROUP BY event_type"

# 6. Cross-source investigation via database profiles
dcx ca ask --profile ops-alloydb.yaml "any blocked queries right now?"
dcx ca ask --profile finance-spanner.yaml "failed transactions last hour"
```

---

## 7. Implementation Roadmap

### Phase 1: Core CLI + Analytics (v0.1) — Complete

- [x] Rust CLI scaffold with `clap` (auth, global flags, `--format`)
- [x] `dcx analytics` commands: `doctor`, `evaluate`, `get-trace`
- [x] `--exit-code` for CI/CD
- [x] JSON/table/text output formatting
- [x] Auth: ADC + service account + `dcx auth login`
- [x] npm distribution (`npx dcx`) — platform-specific binaries via
  optional dependencies (same approach as `esbuild`, `turbo`)
- [x] 5 core skills: `dcx-shared`, `dcx-analytics`, `dcx-analytics-evaluate`,
  `dcx-analytics-trace`, `dcx-query`

**Exit criteria:** `npx dcx analytics evaluate --last=1h --exit-code` works
in GitHub Actions; 5 skills installable via `npx skills add`.

### Phase 2: Dynamic BigQuery API + Skills (v0.2) — Complete

- [x] Discovery Document fetching + caching (bundled, pinned copy)
- [x] Dynamic `clap::Command` tree generation for BigQuery v2 API
- [x] `dcx generate-skills` command
- [x] Non-CA curated skills: 19 of 26 skills (see §4.1); CA-dependent
  skills ship in Phase 3
- [x] Model Armor integration (`--sanitize`) — uses regional endpoints,
  verified with live prompt injection detection and redaction
- [x] Gemini CLI extension manifest packaged and validated
  (`extensions/gemini/manifest.json`, 17 tools); `gemini extensions
  install` not yet tested live as the Gemini CLI extension spec is
  still evolving

**Exit criteria:**
- `dcx datasets list` works without any hardcoded command definition ✓
- `dcx generate-skills` produces valid SKILL.md files ✓
- Gemini extension manifest packaged and programmatically validated ✓
- `--sanitize` verified end-to-end against live Model Armor ✓

See [docs/e2e-validation.md](docs/e2e-validation.md) for the full
reproducible validation script.

### Phase 3: Conversational Analytics + Polish (v0.3) — Complete

- [x] `dcx ca ask` — natural language query via CA API
- [x] `dcx ca create-agent` — create data agents
- [x] `dcx ca add-verified-query` — add verified queries
- [x] Ship `deploy/ca/verified_queries.yaml` with SDK
- [x] Remaining CA-dependent skills (7 of 26): `dcx-ca`, `dcx-ca-ask`,
  `dcx-ca-create-agent`, `persona-sre` (requires `dcx-ca`),
  `recipe-ca-data-agent-setup`, `recipe-error-alerting`,
  `recipe-self-diagnostic-agent`
- [x] Remaining analytics commands: `insights`, `drift`, `distribution`,
  `views`, `hitl-metrics`, `list-traces`
- [x] Completion scripts (bash, zsh, fish)
- [x] Documentation and examples

**Exit criteria:** `dcx ca ask "error rate for support_bot?"` returns
structured JSON with SQL and results; all analytics commands pass
integration tests.

### Phase 4: Data Cloud CA + Multi-Source Profiles (v0.4) — Complete

- [x] Source model: `CaProfile` with `SourceType` enum (BigQuery, Looker,
  LookerStudio, AlloyDb, Spanner, CloudSql)
- [x] Provider split: Chat/DataAgent for BigQuery/Looker/Studio,
  QueryData for AlloyDB/Spanner/Cloud SQL
- [x] `dcx ca ask --profile` routes to the correct API family based on
  source type
- [x] Looker explore profiles with instance URL and model/explore references
- [x] AlloyDB, Spanner, Cloud SQL profiles with database connection details
- [x] QueryData API integration with optional `context_set_id`
- [x] 6 new Data Cloud skills: `dcx-ca-looker`, `dcx-ca-database`,
  `dcx-ca-alloydb`, `dcx-ca-spanner`, `recipe-ca-looker-exploration`,
  `recipe-ca-database-ops`
- [x] Updated routing skills (`dcx-ca`, `dcx-ca-ask`, `persona-sre`)
- [x] E2E validation against live AlloyDB, Spanner, and Cloud SQL instances
- [x] Docs and positioning update
- [x] Version bump to 0.4.0 and release closure

**Exit criteria:** `dcx ca ask --profile <source>.yaml` works for BigQuery,
Looker, AlloyDB, Spanner, and Cloud SQL; skill layer reflects multi-source
Data Cloud support; docs updated.

See [PHASE4_PLAN.md](PHASE4_PLAN.md) for the full plan.

### Phase 5: Native Data Cloud Commands Beyond BigQuery (v0.5) — In Progress

- [x] Add top-level profile utilities: `dcx profiles list|show|validate`
- [x] Add `dcx looker instances|backups list|get` (Discovery-driven)
- [x] Add `dcx looker explores|dashboards list|get` (hand-written,
  per-instance Looker API)
- [x] Add `dcx spanner instances|databases list|get|get-ddl` (Discovery-driven)
- [x] Add `dcx alloydb clusters|instances list|get` (Discovery-driven)
- [x] Add `dcx cloudsql instances|databases list|get` (Discovery-driven)
- [x] Add profile-aware schema and database helpers: `dcx spanner schema
  describe`, `dcx cloudsql schema describe`, `dcx alloydb schema describe`,
  `dcx alloydb databases list`
- [ ] Expand skills and docs so agents can choose between `ca ask` and direct
  source commands
- [ ] Release `0.5.0` with a validated cross-source command matrix

**Architecture note (M3):** Spanner, AlloyDB, and Cloud SQL commands are
generated from bundled Discovery documents (`spanner/v1`, `alloydb/v1`,
`sqladmin/v1`) using the same dynamic pipeline as BigQuery. This replaced
the original hand-written static approach, eliminating ~1,300 lines of
per-service code and giving automatic coverage of all allowlisted API
methods. The `ServiceConfig` abstraction in `src/bigquery/dynamic/service.rs`
holds per-service configuration (namespace, allowlist, global param mapping,
flatPath preference). Looker is a hybrid: instance management uses the
Discovery-driven pipeline (`looker/v1`), while content commands (explores,
dashboards) use a hand-written client against the per-instance Looker API.

**M4 note:** Profile-aware helpers (`spanner schema describe`,
`cloudsql schema describe`, `alloydb schema describe`,
`alloydb databases list`) use CA QueryData to inspect source schemas
and databases. They validate profile/source type compatibility before
auth, and support `json`, `table`, and `text` output formats.
Implementation: `src/commands/database_helpers.rs`.

**Exit criteria:** `dcx` supports direct, structured, non-CA commands for
Looker, Spanner, AlloyDB, and Cloud SQL in addition to the existing BigQuery
command surface.

See [PHASE5_PLAN.md](PHASE5_PLAN.md) for the full plan.

### Testing Strategy

- **Unit tests:** Core parsing, auth resolution, output formatting
- **Integration tests:** Golden-file tests comparing CLI output against
  expected JSON/table snapshots
- **API mocking:** Record/replay via [`wiremock`](https://crates.io/crates/wiremock)
  for BigQuery API calls; no live GCP dependency in CI
- **End-to-end:** Optional `--live` test suite against a dedicated GCP
  project for pre-release validation

---

## 8. Relationship to Existing Tools

| Tool | Role | Relationship to `dcx` |
|------|------|----------------------|
| `bq` CLI | Legacy BigQuery CLI | `dcx` is a successor, not a wrapper. Coexists — users can migrate gradually. |
| `gcloud` | Google Cloud CLI | `dcx` handles Data Cloud-specific workflows and source-aware operations; delegates to `gcloud` for IAM, projects, and infrastructure admin. |
| `gws` CLI | Google Workspace CLI | Architectural template. Same skills format, same output patterns, different domain. |
| `bq-agent-sdk` (from PRD) | Python CLI from current PRD | Ships first as a preview CLI. Once `dcx analytics` reaches feature parity (v0.2), the Python CLI is sunset; the Python SDK *library* continues independently. |
| BigQuery Console | Web UI | `dcx ca ask` brings CA to terminal; `dcx analytics` brings SDK to terminal. |

---

## 9. Open Questions

1. **Language choice:** Rust (like `gws`) vs Go (like `gcloud`/`bq`)?
   **Recommendation:** Rust — faster startup, smaller binary, proven by `gws`.

2. **Naming:** `dcx` vs `bqai` vs `bq2`?
   **Recommendation:** `dcx` — short, clearly extends `bq`, no conflict.

3. **BigQuery API coverage scope:** Full Discovery Document or curated subset?
   **Recommendation:** Start with curated (datasets, tables, jobs, routines,
   models, connections); add more resources via `generate-skills` as needed.

4. **CA API availability:** The Conversational Analytics API supports 6
   source types (BigQuery, Looker, Looker Studio, AlloyDB, Spanner,
   Cloud SQL). The API split between Chat/DataAgent and QueryData is
   modeled explicitly in the code. **Mitigation:** Provider-specific logic
   is isolated; source-specific tests ensure stability as the API evolves.

5. **Relationship to `bq-agent-sdk` CLI in current PRD:**
   **Recommendation:** The current PRD's Python CLI (§4) ships as
   `bq-agent-sdk` (Python/typer) in a preview release. Once `dcx`
   reaches v0.2 with feature parity, analytics commands migrate to
   `dcx analytics` and the Python CLI is sunset. The Python SDK
   *library* continues to be maintained independently.
