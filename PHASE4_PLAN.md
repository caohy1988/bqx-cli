# Phase 4 Plan: bqx to Agentic Data Cloud CLI (Proposed)

## Goal

Repurpose `bqx` from a BigQuery-first agent CLI into a broader **agentic Data
Cloud CLI** for Google Cloud.

This is a proposed Phase 4 direction. The committed roadmap in
[README.md](/Users/haiyuancao/bqx-cli/README.md) currently ends at Phase 3, so
this document is a forward-looking plan based on:

- the current Phase 3 repo state
- the official Conversational Analytics API support matrix
- the Data Cloud product direction across BigQuery, Looker, AlloyDB, Spanner,
  and related database surfaces

The main reason to change direction is that Conversational Analytics is no
longer just a BigQuery story.

As of March 19, 2026, official Google Cloud documentation says the
Conversational Analytics API supports:

- BigQuery
- Looker
- Looker Studio
- AlloyDB for PostgreSQL
- GoogleSQL for Spanner
- Cloud SQL
- Cloud SQL for PostgreSQL

But the support model is split:

- `Chat` and `DataAgent` are for BigQuery, Looker, and Looker Studio
- `QueryData` is for database sources such as AlloyDB, Spanner, and Cloud SQL

That means the current Phase 3 framing - "BigQuery CLI with CA support" - is no
longer enough. Phase 4 should turn `bqx` into the CLI layer for **agent access
to Google Cloud Data Cloud sources**, starting with CA and source-aware
workflows.

Versioning note:
the repo is currently at `0.3.0` in
[Cargo.toml](/Users/haiyuancao/bqx-cli/Cargo.toml). This proposal treats Phase
4 as the path to `0.4.0`.

## Recommended Phase 4 Positioning

Short version:

- keep `bqx` as the implementation vehicle and binary for continuity
- reposition it as an **agentic Data Cloud CLI**
- broaden CA support from BigQuery-only to multi-source CA across Data Cloud
- defer broad admin/CRUD ambitions for AlloyDB and Spanner until after the CA
  and source-model story is stable

This is the most pragmatic path because the current repo already has:

- working CLI infrastructure
- auth, packaging, skills, completions, and release automation
- a CA client and command surface
- strong output and test patterns

The right Phase 4 move is to generalize the CA and workflow layer first, not to
rebuild the tool from scratch.

## Research Summary

Official Google Cloud documentation supports the following planning assumptions:

### 1. Conversational Analytics API is already multi-source

The official overview says the API answers questions about structured data in
BigQuery, Looker, and Looker Studio, and also supports querying AlloyDB,
GoogleSQL for Spanner, Cloud SQL, and Cloud SQL for PostgreSQL through the new
`QueryData` method.

### 2. The API has two distinct source models

Known limitations explicitly state:

- `QueryData` does **not** support BigQuery or Looker data sources
- `Chat` and `DataAgent` do **not** support database data sources such as
  AlloyDB, Spanner, and Cloud SQL

This is the most important product and architecture constraint for Phase 4.
There is not one universal CA command implementation today. We need a CLI
surface that normalizes two official API patterns.

### 3. Looker support is real, but has source-specific constraints

Official docs require:

- Looker instance URL
- model + explore references
- Looker-specific permissions
- a maximum of five explores in context

So Looker cannot be treated as "BigQuery with another flag." It needs a proper
source profile and validation path.

### 4. Database sources need authored context, not just connection info

For AlloyDB, Spanner, Cloud SQL, and Cloud SQL for PostgreSQL, the official
database authored-context docs say:

- database CA flows go through `QueryData`
- context is referenced through database-specific datasource references
- database sources require an `agentContextReference` / `context_set_id`

This means Phase 4 cannot just reuse `ca create-agent` unchanged for database
sources. Database source support needs its own profile/context model.

### 5. Data Cloud is the right umbrella

The Google Cloud Data Cloud page positions BigQuery, Looker, and database
products such as AlloyDB within one broader data-to-AI platform story. That
makes a broader "agentic Data Cloud CLI" story more aligned than a
BigQuery-only framing once multi-source CA becomes the center of the product.

## Baseline (post-Phase 3)

Phase 3 established the current starting point:

- static command tree for:
  - `jobs query`
  - `analytics doctor|evaluate|get-trace|list-traces|insights|drift|distribution|hitl-metrics|views`
  - `auth login|status|logout`
  - `ca ask|create-agent|list-agents|add-verified-query`
  - `generate-skills`
  - `completions`
- dynamic BigQuery API commands generated from Discovery:
  - `datasets list|get`
  - `tables list|get`
  - `routines list|get`
  - `models list|get`
- JSON-first output plus `table` and `text`
- shared auth, config, sanitize, output, and client layers under
  [src/](/Users/haiyuancao/bqx-cli/src)
- 26 checked-in skills under [skills/](/Users/haiyuancao/bqx-cli/skills)
- Gemini extension manifest and release automation
- npm distribution via `bqx`
- bash/zsh/fish completions
- live e2e validation docs in
  [docs/e2e-validation.md](/Users/haiyuancao/bqx-cli/docs/e2e-validation.md)

What the current repo does **not** provide:

- no Looker or Looker Studio CA support
- no AlloyDB CA support
- no Spanner CA support
- no Cloud SQL CA support
- no unified "Data Cloud source profile" model
- no source-aware CA routing that understands `Chat` versus `QueryData`
- no non-BigQuery product story in the skill layer
- no naming or docs framing for `bqx` as a broader Data Cloud CLI

## Scope

In scope:

- broaden CA support from BigQuery-only to multi-source Data Cloud support
- support the official CA source classes:
  - BigQuery
  - Looker
  - Looker Studio
  - AlloyDB
  - Spanner
  - Cloud SQL / Cloud SQL for PostgreSQL
- introduce a source-profile model for CA requests
- keep `bqx` as the binary while broadening the product framing
- add the source-aware command surface, skills, docs, and validation needed to
  make the CLI feel intentionally multi-source

Out of scope:

- becoming the full admin CLI for AlloyDB, Spanner, or Looker
- replacing `gcloud`, `bq`, or product-native CLIs for generic CRUD
- broad dynamic discovery across every Google Cloud Data product in Phase 4
- full write/mutation coverage for non-BigQuery products
- broad Homebrew/watch/migrate work from the previous Phase 4 draft

The previous proposed Phase 4 items such as mutation safety, watch mode, and
Homebrew are still valid backlog ideas, but they are no longer the top Phase 4
priority if the product direction becomes "agentic Data Cloud CLI."

## Architecture Direction

Keep the current split:

- BigQuery dynamic API commands stay in the Phase 2 system
- analytics commands stay hand-written and BigQuery-specific for now; these
  prototype the workflow patterns that will migrate to Skills once agent ops
  APIs land (see the one-pager Skills-over-APIs architecture)
- CA becomes a **source-aware multi-provider layer**

Recommended implementation rule:

- do not try to hide the official API split internally
- normalize it at the CLI boundary, but model it honestly in the code
- treat BigQuery/Looker/Looker Studio as one CA family (`chat` / `dataAgent`)
- treat AlloyDB/Spanner/Cloud SQL as a second CA family (`queryData`)

Target modules:

```text
src/
├── ca/
│   ├── mod.rs
│   ├── client.rs
│   ├── models.rs
│   ├── profiles.rs
│   ├── provider.rs
│   ├── bigquery.rs
│   ├── looker.rs
│   ├── studio.rs
│   └── databases/
│       ├── mod.rs
│       ├── alloydb.rs
│       ├── spanner.rs
│       ├── cloudsql.rs
│       └── query_data.rs
├── commands/
│   ├── analytics/
│   ├── ca/
│   │   ├── ask.rs              # unified entry point; dispatches by profile source type
│   │   ├── create_agent.rs
│   │   ├── list_agents.rs
│   │   ├── add_verified_query.rs
│   │   └── profiles.rs
│   ├── generate_skills.rs
│   └── jobs_query.rs
├── cli.rs
├── config.rs
├── output.rs
└── main.rs
deploy/
└── ca/
    ├── verified_queries.yaml
    └── profiles/
        ├── bigquery/
        ├── looker/
        └── databases/
tests/
├── ca_tests.rs
├── ca_profile_tests.rs
├── ca_looker_tests.rs
├── ca_database_tests.rs
├── fixtures/
│   └── ca/
│       ├── bigquery/
│       ├── looker/
│       ├── alloydb/
│       └── spanner/
└── snapshots/
```

## Core Design

### 1. Source Model

Phase 4 should introduce an explicit CA source model.

Recommended source classes:

- `BigQuery`
- `Looker`
- `LookerStudio`
- `AlloyDb`
- `Spanner`
- `CloudSql`

Recommended internal split:

- `ChatSource`: BigQuery, Looker, Looker Studio
- `QueryDataSource`: AlloyDB, Spanner, Cloud SQL

Start with a single `CaProfile` struct that has a `source_type` discriminator
and optional source-specific fields. Do not build separate profile structs
for each source until the model proves stable:

```rust
struct CaProfile {
    name: String,
    source_type: SourceType,        // BigQuery | Looker | AlloyDb | Spanner | ...
    project: String,
    location: Option<String>,
    // BigQuery / Looker / Studio (Chat/DataAgent)
    agent: Option<String>,
    tables: Option<Vec<String>>,
    looker_instance_url: Option<String>,
    looker_explores: Option<Vec<String>>,
    // Database (QueryData)
    context_set_id: Option<String>,
    datasource_ref: Option<String>,
}
```

Split into specialized structs only when the single struct becomes unwieldy.

### 2. Command Surface

The current `ca` commands should expand carefully instead of being replaced.

Recommended Phase 4 command surface:

- `bqx ca ask --profile <name> "<question>"`
  - unified entry point for all CA sources
  - the CLI reads the source type from the profile and routes internally:
    - BigQuery, Looker, Looker Studio → `Chat` / `DataAgent` API
    - AlloyDB, Spanner, Cloud SQL → `QueryData` API
  - agents and users never need to know the API split — the profile handles it
- `bqx ca create-agent`
  - for BigQuery, Looker, Looker Studio only (Chat/DataAgent sources)
- `bqx ca list-agents`
- `bqx ca add-verified-query`
- `bqx ca profiles add`
- `bqx ca profiles list`
- `bqx ca profiles get`

Recommended rule:

- present **one command** (`ca ask`) to users and agents — the profile
  determines which API family is called
- model the `Chat`/`DataAgent` vs `QueryData` split honestly **in code**
  (`ca/bigquery.rs` vs `ca/databases/query_data.rs`), but do not expose it as
  two separate commands
- if `ca create-agent` is called with a database profile, return a clear error
  explaining that DataAgent is not supported for database sources

### 3. Profile Model

Profiles are the key Phase 4 abstraction.

BigQuery today mostly gets by with flags like `--agent` and `--tables`. That is
not enough for Looker or database sources.

Recommended profile types:

- BigQuery agent profile
- Looker explore profile
- Looker Studio datasource profile
- AlloyDB database profile
- Spanner database profile
- Cloud SQL database profile

Each profile should capture:

- source type
- project / billing project
- location if applicable
- source-specific identifiers
- auth mode or required credential hints
- authored context references
- default output mode or safe defaults where useful

Recommended initial file format:

- checked-in YAML or JSON under `deploy/ca/profiles/`
- optional local user profile directory later

### 4. BigQuery / Looker / Looker Studio CA Family

This family uses `Chat` and `DataAgent`.

Phase 4 should support:

- BigQuery inline tables and agents
- Looker explores
- Looker Studio datasources

Important constraints from official docs:

- Looker requires instance URL plus model/explore references
- Looker has additional permissions requirements
- Looker supports up to five explores in context
- Looker Studio is a distinct datasource type from Looker

Recommended Phase 4 outcome:

- a user can create or reference a source profile for Looker or Studio
- `bqx ca ask --profile sales-looker "..."` works
- response rendering remains stable across these source types

### 5. Database CA Family

This family uses `QueryData`, not `Chat`.

Phase 4 should support:

- AlloyDB
- Spanner
- Cloud SQL / Cloud SQL for PostgreSQL

Important constraints from official docs:

- database sources require datasource references inside `QueryData`
- authored context is referenced through `context_set_id`
- database source support is preview and source-specific
- building data agents and rendering visualizations are not supported for
  database sources in the same way as for chat/data-agent sources

Recommended Phase 4 outcome:

- `bqx ca ask --profile ops-alloydb "top error categories last 24h"`
- `bqx ca ask --profile finance-spanner "daily failed payments by region"`
- stable JSON shape that includes generated SQL, result rows, and reasoning if
  returned by the API

### 6. Branding and Product Framing

The repo can stay on the `bqx` binary in Phase 4.

Recommended framing:

- short term: "`bqx` is evolving into an agentic Data Cloud CLI"
- medium term: validate whether the broader Data Cloud scope warrants a rename
  or alias

Do not front-load a rename. Validate the broader source model first.

## Proposed Milestones

### Milestone 1: Multi-Source CA Foundation

Objective:
introduce the source model, provider abstraction, and profile system.

Tasks:

- add source/profile types under
  [src/ca/](/Users/haiyuancao/bqx-cli/src/ca)
- split the CA client into chat/data-agent and query-data families
- add source-aware validation before auth/network
- add profile loading and profile schema tests
- keep current BigQuery CA behavior working unchanged

Done when:

- the codebase has an explicit source model
- BigQuery CA still works
- source profiles can be parsed and validated for at least BigQuery, Looker,
  AlloyDB, and Spanner

### Milestone 2: Looker + Looker Studio CA

Objective:
add non-BigQuery analytic source support through the `Chat`/`DataAgent` path.

Tasks:

- implement Looker explore references
- implement Looker Studio datasource references
- add `ca ask --profile ...` for these sources
- add text/json output coverage
- document Looker-specific permission and explore-count constraints

Done when:

- `bqx ca ask` works against Looker profiles
- `bqx ca ask` works against Looker Studio profiles
- docs clearly explain what is different from BigQuery CA

### Milestone 3: AlloyDB + Spanner + Cloud SQL QueryData

Objective:
support database-source CA through the `QueryData` API, routed via
`ca ask --profile`.

Tasks:

- implement QueryData provider behind the existing `ca ask` command
- support AlloyDB profile references
- support Spanner profile references
- support Cloud SQL profile references
- model `context_set_id` / authored context references
- add tests for source-specific request construction

Done when:

- `bqx ca ask --profile ops-alloydb "..."` routes to QueryData and returns
  results
- Spanner profiles work the same way
- Cloud SQL support is either working or explicitly deferred with a clear
  reason
- docs explain that database sources use QueryData under the hood, but users
  interact through the same `ca ask` command

### Milestone 4: Data Cloud Skill Layer

Objective:
expand the skill story from BigQuery-only to Data Cloud source workflows.

Tasks:

- add CA source-specific skills:
  - `bqx-ca-looker`
  - `bqx-ca-query-data`
  - `bqx-ca-alloydb`
  - `bqx-ca-spanner`
- update routing skills to select the right `--profile` for the user's data
  source
- add recipes for:
  - Looker conversational exploration
  - AlloyDB operational troubleshooting
  - Spanner business query workflows

Done when:

- the skill tree no longer implies that CA is only a BigQuery feature
- a tool-using agent can discover the right CA command for each source family

### Milestone 5: Docs, Positioning, and Validation

Objective:
make the product story intentionally broader than BigQuery while staying
accurate to the current implementation.

Tasks:

- update [README.md](/Users/haiyuancao/bqx-cli/README.md) framing from
  BigQuery-only CA to Data Cloud CA support
- update one-pagers and proposal docs
- add e2e docs for:
  - BigQuery CA
  - Looker CA
  - AlloyDB / Spanner query-data flows
- add validation docs for source-specific prerequisites and limitations

Done when:

- the docs make the Data Cloud story explicit
- limitations are documented honestly
- the product pitch no longer reads like a BigQuery-only CA tool

### Milestone 6: Release Closure and `0.4.0`

Objective:
ship the broadened Data Cloud CA surface as the first `0.4.0` release.

Tasks:

- complete version bump and release notes
- refresh package/docs/manifest metadata as needed
- validate the final supported source matrix
- decide whether to keep the `bqx` name only or add a broader product alias in
  docs

Done when:

- `0.4.0` reflects the broadened Data Cloud positioning
- BigQuery, Looker, AlloyDB, and Spanner are all represented in the shipped
  command/docs/skill surface

## Recommended Build Order

1. Multi-source CA foundation
2. Looker + Looker Studio support
3. AlloyDB + Spanner + Cloud SQL `query-data`
4. Data Cloud skill layer
5. Docs and product repositioning
6. Release closeout

Rationale:

- the source model is the hardest architectural dependency
- Looker can reuse more of the existing chat/data-agent shape than databases
- database support is the real differentiator, but it should land on a stable
  profile model
- skills and messaging should follow the actual product surface, not lead it

## Testing Strategy

BigQuery / Looker / Studio:

- unit tests for source-profile parsing
- fixture-backed request builder tests
- integration tests for response mapping and rendering

Database QueryData:

- request-construction tests per source
- profile validation tests for required context-set identifiers
- mocked API tests for `queryData` result mapping

Fixture capture:

- capture real API response samples from each source type early in the
  milestone (even manually via curl)
- store them under `tests/fixtures/ca/{bigquery,looker,alloydb,spanner}/`
- build snapshot tests from these fixtures so CI coverage is not gated on live
  access

Live validation:

- BigQuery CA smoke checks remain the easiest live path
- Looker and database source validation should begin as documented manual or
  scheduled workflows, not required PR gates

Docs validation:

- every example command in docs should be runnable against a test profile or
  explicitly marked illustrative

## Open Decisions

### 1. Naming

Question:
should the binary remain `bqx` once it stops being BigQuery-only?

Recommended answer:
keep `bqx` in Phase 4 for continuity and speed. Make a concrete rename
decision in M6: if multi-source CA is validated, pick an alias (e.g., `dcx`,
`dcloud`) and ship it alongside `bqx` in the 0.4.0 release. The name `bqx`
actively works against a "Data Cloud CLI" pitch — this should be resolved at
release, not deferred indefinitely.

### 2. Command unification

Question:
should database sources use a separate `ca query-data` command or go through
`ca ask`?

Recommended answer:
unify under `ca ask`. The profile knows the source type and routes to the
right API internally. Users and agents should not need to know whether the
backend uses Chat/DataAgent or QueryData — that is an API implementation
detail, not a user-facing concern.

### 3. Scope breadth

Question:
should Phase 4 support all officially documented CA sources or only a subset?

Recommended answer:
make BigQuery, Looker, AlloyDB, and Spanner the required targets. Treat Looker
Studio and Cloud SQL as included if they are cheap, but not at the cost of
slipping the core four.

### 4. Source profiles

Question:
should profiles be checked-in repo config, user-local config, or both?

Recommended answer:
start with checked-in YAML/JSON for determinism and team sharing. Add user-local
profiles later.

## Risks

### 1. CA preview instability

Risk:
the official CA API is still preview/pre-GA and may change.

Mitigation:

- isolate provider-specific logic
- keep source-specific tests strong
- document preview assumptions explicitly

### 2. Product-shape mismatch

Risk:
BigQuery/Looker and database sources are not symmetric. A fake "one command
fits all" abstraction will leak quickly.

Mitigation:

- model the split honestly
- use profiles and command families to normalize only what is actually common

### 3. Overbroad Data Cloud ambition

Risk:
trying to become the admin CLI for every Data Cloud product would dilute the
phase immediately.

Mitigation:

- keep Phase 4 centered on conversational access and agent workflows
- defer broad product-specific CRUD to later phases

### 4. Naming confusion

Risk:
`bqx` sounds BigQuery-specific even if the product broadens.

Mitigation:

- keep the binary for continuity
- use docs and positioning to introduce the broader agentic Data Cloud framing
- revisit rename only after product-market fit is clearer

## Definition of Done

This proposed Phase 4 is complete when:

- `bqx` supports CA across BigQuery, Looker, AlloyDB, and Spanner at minimum
- the CLI has a clear source-profile model
- `ca ask --profile` routes to the right API for each source type
- the skill layer reflects the broader Data Cloud source model
- docs explain the official source split and its limits honestly
- the `0.4.0` release reflects the broadened Data Cloud positioning

## Suggested First PRs

1. `refactor(ca): add source profiles and provider abstraction`
2. `feat(ca): add Looker and Looker Studio profiles to ca ask`
3. `feat(ca): add QueryData routing for AlloyDB and Spanner via ca ask`
4. `skills(ca): add Data Cloud source-specific CA skills`
5. `docs: reposition bqx as agentic Data Cloud CLI`

## Recommended Starting Point

If Phase 4 starts now, the right first PR is Milestone 1:

- add an explicit CA source/profile model
- split `Chat`/`DataAgent` and `QueryData` internally
- preserve current BigQuery behavior while preparing for Looker and database
  sources

That gives the project the right foundation for a genuine Data Cloud CLI,
instead of a BigQuery CLI with a few extra adapters.

## Research Basis

Primary official sources consulted for this update:

- Conversational Analytics API overview:
  https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/overview
- Conversational Analytics API authentication and source setup:
  https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/authentication
- Database authored context for `QueryData`:
  https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/data-agent-authored-context-databases
- FAQ and support matrix:
  https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/frequently-asked-questions
- Known limitations:
  https://docs.cloud.google.com/gemini/data-agents/conversational-analytics-api/known-limitations
- Google Cloud Data Cloud overview:
  https://cloud.google.com/data-cloud
