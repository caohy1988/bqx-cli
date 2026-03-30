# Phase 5 Plan: Native Data Cloud Commands Beyond BigQuery (v0.4 to v0.5)

## Goal

Move `dcx` from a multi-source Conversational Analytics CLI into a broader
**Data Cloud operations CLI** with direct command surfaces for products outside
of BigQuery.

Phase 4 proved that `dcx` can speak to six Data Cloud source types through
`dcx ca ask --profile`. Phase 5 should add the next layer: first-class,
source-native commands for Looker, Spanner, AlloyDB, and Cloud SQL so users and
agents do not have to drop back to `gcloud`, ad hoc curl scripts, or custom
wrappers for every non-BigQuery task.

This phase is intentionally not "replace every product CLI." The goal is a
focused read-only and workflow-oriented command layer that makes `dcx` useful
for real Data Cloud work outside BigQuery.

Authority note:
this plan extends the current roadmap in
[README.md](README.md). The README remains
authoritative for the top-level roadmap, and this document is the detailed
implementation proposal for a new proposed Phase 5.

Versioning note:
the repo is currently at `0.4.0` in
[Cargo.toml](Cargo.toml). This plan treats Phase 5 as
the path to `0.5.0`.

## Baseline (post-Phase 4)

Phase 4 established:

- a renamed `dcx` binary and package surface
- `dcx ca ask --profile` across:
  - BigQuery
  - Looker
  - Looker Studio
  - AlloyDB
  - Spanner
  - Cloud SQL
- source-aware CA routing across `Chat` / `DataAgent` and `QueryData`
- source profiles under `deploy/ca/profiles/`
- 32 skills (4 generated + 28 curated)
- release automation, npm distribution, completions, Gemini manifest, and e2e
  docs

What the repo still does not provide:

- no direct `dcx looker ...` commands
- no direct `dcx spanner ...` commands
- no direct `dcx alloydb ...` commands
- no direct `dcx cloudsql ...` commands
- no source-native metadata or inventory commands outside the BigQuery dynamic
  layer
- no profile utility command domain for inspecting or validating non-BigQuery
  source configs
- no skill layer for direct non-CA product commands

That is the Phase 5 gap: multi-source natural-language access exists, but
multi-source direct CLI access does not.

## Scope

In scope:

- new source-native command domains for Data Cloud products outside BigQuery:
  - `dcx looker`
  - `dcx spanner`
  - `dcx alloydb`
  - `dcx cloudsql`
- read-only `list` / `get` resource commands first
- profile-aware query or schema helpers where the product surface makes that
  practical and safe
- shared source adapter layer so command logic is not duplicated across
  products
- profile utility commands for listing, validating, and showing source profiles
- output, docs, skills, completions, Gemini manifest, and validation updates
  for the new command families

Out of scope:

- full replacement of `gcloud`, `bq`, or product-native CLIs
- broad write and delete coverage for AlloyDB, Spanner, Looker, or Cloud SQL
- mutation-heavy admin workflows such as failover, backup restore, IAM policy
  editing, or network configuration
- broad generated discovery coverage across every Google Cloud API in Phase 5
- changing the existing BigQuery dynamic command system into a universal
  generator for all products

Recommended implementation rule:

- Phase 5 should be read-only first
- every new command should validate locally before auth or network
- every new command should support `json`, `table`, and `text`
- profiles should remain the main context abstraction for cross-source work
- `dcx ca` remains the natural-language layer; Phase 5 adds direct command
  layers beside it

## Architecture Direction

**Updated (M3):** The original plan called for hand-written static commands
for Spanner, AlloyDB, and Cloud SQL. During M3 implementation, we discovered
that Google publishes Discovery documents for all three services
(`spanner/v1`, `alloydb/v1`, `sqladmin/v1`), making it possible to reuse the
same dynamic generation pipeline as BigQuery. This replaced ~1,300 lines of
hand-coded command/source modules with a single `ServiceConfig` abstraction.

The current split:

- **BigQuery, Spanner, AlloyDB, Cloud SQL**: Discovery-driven dynamic
  commands via `src/bigquery/dynamic/service.rs`. Each service has a
  `ServiceConfig` holding its namespace, allowlist, global param mapping,
  bundled JSON, and flatPath preference. BigQuery is top-level; others are
  namespaced (`dcx spanner ...`, `dcx alloydb ...`, `dcx cloudsql ...`).
- **Looker**: Hand-written static commands (the Looker API is not a Google
  Discovery document)
- **CA**: Stays source-aware and profile-based from Phase 4
- **Analytics, Jobs, Profiles**: Hand-written static commands

Key architectural decisions from M3:

- **flatPath vs path**: Spanner and AlloyDB use composite `parent` params in
  `path` (e.g., `v1/{+parent}/instances`); their `flatPath` decomposes them
  into individual params (e.g., `v1/projects/{projectsId}/instances`). The
  `use_flat_path` toggle in `ServiceConfig` controls this. BigQuery and
  Cloud SQL use standard `path`.
- **Recursive resource walking**: BigQuery resources are flat (one level),
  but Spanner/AlloyDB have deeply nested resources. `extract_methods` uses
  recursive traversal.
- **Global param normalization**: `projectsId` → `project_id` (strips
  plural `s` before `Id`). AlloyDB's `locationsId` maps to the global
  `--location` flag with "US" → "-" normalization.
- **Identifier validation**: All path parameters are validated via
  `validate_identifier()` before any network call.

Target modules (updated):

```text
src/
├── bigquery/dynamic/
│   ├── service.rs          # ServiceConfig for all 4 Discovery services
│   ├── model.rs            # Method extraction, flatPath parsing, kebab-case
│   ├── clap_tree.rs        # Dynamic clap::Command tree builder
│   ├── request_builder.rs  # URL template substitution
│   └── executor.rs         # Shared HTTP executor + response rendering
├── sources/
│   ├── mod.rs
│   └── looker/             # Hand-written Looker API client
│       ├── mod.rs
│       ├── client.rs
│       └── models.rs
├── commands/
│   ├── profiles/           # dcx profiles list|show|validate
│   ├── looker/             # dcx looker explores|dashboards
│   └── ...                 # analytics, ca, jobs, etc. (unchanged)
├── cli.rs
├── output.rs
└── main.rs
assets/
├── bigquery_v2_discovery.json
├── spanner_v1_discovery.json
├── alloydb_v1_discovery.json
└── sqladmin_v1_discovery.json
```

Note: The `src/commands/spanner/`, `src/commands/alloydb/`,
`src/commands/cloudsql/`, `src/sources/spanner/`, `src/sources/alloydb/`,
and `src/sources/cloudsql/` directories were deleted when the static
implementations were replaced by the Discovery-driven dynamic path.

## Proposed Command Surface

Phase 5 should add five new top-level domains:

- `dcx profiles`
- `dcx looker`
- `dcx spanner`
- `dcx alloydb`
- `dcx cloudsql`

Recommended v0.5 command surface:

```text
dcx profiles list
dcx profiles show --profile <name>
dcx profiles validate --profile <name>

dcx looker explores list --profile <name>
dcx looker explores get --profile <name> --explore <model/explore>
dcx looker dashboards list --profile <name>
dcx looker dashboards get --profile <name> --dashboard-id <id>

dcx spanner instances list
dcx spanner databases list --instance <id>
dcx spanner databases get --instance <id> --database <id>
dcx spanner schema describe --profile <name>

dcx alloydb clusters list
dcx alloydb clusters get --cluster <id>
dcx alloydb instances list --cluster <id>
dcx alloydb databases list --profile <name>

dcx cloudsql instances list
dcx cloudsql instances get --instance <id>
dcx cloudsql databases list --instance <id>
dcx cloudsql schema describe --profile <name>
```

Design intent:

- `profiles` makes the cross-source setup visible and inspectable
- `looker` focuses on explores and dashboards first because they map cleanly to
  the BI workflow story
- `spanner`, `alloydb`, and `cloudsql` start with inventory and schema-oriented
  commands instead of broad admin CRUD
- `dcx alloydb databases list --profile <name>` is intentionally profile-driven
  in the first cut: the profile resolves cluster, instance, and database
  context. A direct `--cluster` / `--instance` variant can be added later if it
  proves necessary for pure inventory workflows.

## Core Design

### 1. Profile Utilities Become First-Class

Phase 4 made profiles central but only exposed them indirectly through
`dcx ca ask --profile`.

Phase 5 should make profiles a user-facing command domain:

- `dcx profiles list`
- `dcx profiles show`
- `dcx profiles validate`

Why this matters:

- it improves local debugging for users and agents
- it gives the skill layer a clean entry point for setup validation
- it reduces silent profile mistakes before hitting CA or source-native APIs

Suggested behavior:

- `list` shows profile name, source type, family, and origin path
- `show` prints the resolved profile with secrets redacted
- `validate` performs structural validation only by default, with an optional
  `--check-access` mode later if needed

Profile discovery and precedence:

- explicit file path wins first:
  - `--profile path/to/file.yaml`
- named profile lookup then checks the user config directory:
  - `$XDG_CONFIG_HOME/dcx/profiles/` when `XDG_CONFIG_HOME` is set
  - otherwise `~/.config/dcx/profiles/`
- repo-local fixtures under `deploy/ca/profiles/` remain the fallback for
  development, examples, and tests

This matches the current Phase 4 behavior and keeps local fixtures useful
without making them the installed-CLI default.

### 2. Source Adapters, Not Product-Specific Command Sprawl

Phase 5 should not dump all network logic directly into command handlers.

Recommended internal pattern:

- one adapter per source family
- each adapter handles:
  - auth wiring
  - request construction
  - response normalization
  - source-specific pagination or field quirks
- command handlers stay thin and focus on CLI args plus rendering

Suggested shared traits:

- `ProfileResolver`
- `SourceInventoryClient`
- `SchemaClient`
- `DashboardClient` or `ExploreClient` for Looker

This keeps the command tree readable and makes future `gcloud data-cloud`
packaging easier if the project later adopts a hybrid distribution model.

### 2a. Auth Model by Source

Phase 5 should make the auth split explicit instead of hiding it behind a
generic "adapter" label.

| Source | Expected auth model | Notes |
|------|------|------|
| Looker | Looker instance auth via profile-provided credentials when needed | This is not the same as GCP IAM; it may require its own token exchange path or reuse of the current Looker CA credential shape. |
| Spanner | GCP IAM / ADC / existing `dcx` auth chain | Same family as current Google Cloud auth. |
| AlloyDB | GCP IAM / ADC / existing `dcx` auth chain | Keep aligned with current GCP auth flow. |
| Cloud SQL | GCP IAM for admin metadata calls; profile fields determine engine context | Do not assume the same auth shape as direct DB username/password access. |

Recommendation:

- Phase 5 should reuse the current `dcx` GCP auth chain for Google Cloud APIs
- Looker should keep a source-specific auth path, because its credential model
  is fundamentally different
- do not force Looker into the same auth abstraction if that makes the code less
  honest

### 2b. API Targets and Stability Contract

Phase 5 should pin the intended API targets up front to avoid scope drift.

Recommended targets for the first implementation:

- Looker API: current supported API version used by the chosen Rust client or
  direct HTTP adapter, pinned explicitly in code and docs
- Spanner: instance/database metadata surfaces from the stable admin API family
- AlloyDB: stable metadata surfaces where available; beta endpoints only if
  they are required and clearly called out in docs and tests
- Cloud SQL: stable admin metadata surfaces first

Rule:

- any beta or preview dependency should be called out in the command docs,
  tests, and release notes
- Phase 5 should prefer stable metadata APIs over broader but less stable
  coverage

### 3. Read-Only Metadata First

The easiest way to overreach Phase 5 is to treat it as a full admin CLI for
every Data Cloud product.

Do not do that.

Phase 5 should be grounded in three practical use cases:

- discover what source assets exist
- inspect the source structure or context before asking CA questions
- connect agent workflows to stable direct commands when natural language is too
  indirect

That is why the recommended first commands are:

- list/get resources
- describe schema
- inspect explores and dashboards

Not:

- create/delete/update everything

### 4. Profiles Remain the Cross-Source Context Layer

Profiles should stay the preferred entry point for:

- Looker commands
- schema helpers
- query helpers that need connection context

Direct flags are still useful for simple inventory commands such as:

- `dcx spanner instances list`
- `dcx cloudsql instances list`

But the default design should keep moving toward:

- one reusable profile
- many compatible commands

That lowers prompt burden for agents and lowers setup burden for humans.

### 5. Output Contract Stays Uniform

All new Phase 5 commands should keep the established CLI contract:

- `json` is default
- `table` is for short human-readable summaries
- `text` is concise and descriptive
- errors use the same predictable JSON envelope or structured command result
  path where available

For nested non-BigQuery responses:

- normalize to a stable top-level response type before rendering
- avoid leaking raw product-specific response shapes directly to the user unless
  the command is explicitly a passthrough

### 6. Skills Expand from CA Routing to Direct Source Workflows

Phase 4 added CA-centric multi-source skills.

Phase 5 should add direct command skills such as:

- `dcx-looker-explores`
- `dcx-looker-dashboards`
- `dcx-spanner`
- `dcx-alloydb`
- `dcx-cloudsql`
- `recipe-source-profile-validation`
- `recipe-cross-source-debugging`

That would take the skill count from 32 total in Phase 4 to roughly 39 total in
Phase 5, assuming all seven ship and no Phase 4 skills are removed.

These should not repeat product docs. They should encode:

- when to use direct commands instead of `ca ask`
- how profiles map to command families
- the safest first commands to run

## Milestones

### Milestone 1: Profile Utilities and Source Adapter Foundation — Complete

Deliverables:

- [x] top-level `dcx profiles` command domain
- [x] shared profile loader/resolver abstraction
- [x] common source adapter traits and result models
- [x] tests for profile listing, rendering, redaction, and validation

Implemented in PR #38.

### Milestone 2: Looker Native Commands — Complete

Deliverables:

- [x] `dcx looker explores list|get`
- [x] `dcx looker dashboards list|get`
- [x] source-specific output models and renderers

Implemented in PR #39. Hand-written commands using the Looker API
(not Discovery-driven, as Looker is not a Google Discovery API).

### Milestone 3: Spanner, AlloyDB, and Cloud SQL Inventory Commands — Complete

Deliverables:

- [x] `dcx spanner instances list|get`, `databases list|get|get-ddl`
- [x] `dcx alloydb clusters list|get`, `instances list|get`
- [x] `dcx cloudsql instances list|get`, `databases list|get`

Implemented in PR #40. **Architecture change from original plan:**
replaced hand-written static commands with Discovery-driven dynamic
generation. All three services use bundled Discovery documents
(`spanner/v1`, `alloydb/v1`, `sqladmin/v1`) processed through the same
pipeline as BigQuery. Key additions:

- `ServiceConfig` abstraction (`src/bigquery/dynamic/service.rs`)
- Recursive resource walking for nested Spanner/AlloyDB resources
- flatPath parameter extraction and normalization
- Multi-service namespace routing in `main.rs`
- Identifier validation for all path parameters before network calls
- AlloyDB `--location` global flag with "US" → "-" normalization

This eliminated ~1,300 lines of hand-coded command/source modules and
gives automatic coverage of all allowlisted API methods.

Validated against live GCP data (`test-project-0728-467323`) on 2026-03-27.

### Milestone 4: Schema and Query Helpers — Complete

Deliverables:

- [x] `dcx spanner schema describe --profile`
- [x] `dcx cloudsql schema describe --profile`
- [x] `dcx alloydb schema describe --profile`
- [x] `dcx alloydb databases list --profile`

Implementation notes:

- All four helpers use the CA QueryData API under the hood, routed by
  source profile. The `QueryDataExecutor` trait abstracts the network
  call for testability.
- Profile/source type compatibility is validated before auth or network
  via `resolve_profile_for_source()`.
- Output models: `SchemaDescribeResult` (with `SchemaRow` per column)
  and `DatabaseListResult` (with `DatabaseRow` per database). Both
  support `json`, `table`, and `text` formats.
- Schema prompts are source-specific: Spanner returns
  `table_name/column_name/data_type/is_nullable`; Cloud SQL adds
  `table_schema` and detects MySQL vs PostgreSQL from `db_type`.
- AlloyDB database listing uses a PostgreSQL-specific prompt to list
  non-template databases.
- Row extraction uses fuzzy key matching (e.g., `table_name` or `table`,
  `database_name` or `datname`) to handle LLM response variation.
- Helpers are wired into namespaced dynamic services via
  `augment_namespace_command()` and `try_run_namespace_helper()` in
  `main.rs`. Non-helper subcommands fall through to Discovery path.
- Model Armor `--sanitize` support included via
  `render_with_optional_sanitization()`.
- Code: `src/commands/database_helpers.rs`
- Tests: `tests/database_helper_command_tests.rs` + unit tests in module

### Milestone 5: Skills, Integrations, and Docs

Deliverables:

- new direct-command skills for non-BigQuery sources
- Gemini manifest updates for selected Phase 5 tools
- README and e2e docs updated for direct source commands
- one-pager adjusted to reflect the broader Data Cloud operations surface

Detailed tasks:

- add curated skills for Looker, Spanner, AlloyDB, and Cloud SQL direct usage
- update existing CA skills to route between CA and direct commands
- refresh Gemini manifest with a conservative subset of the most stable Phase 5
  tools
- update docs with before/after examples that show:
  - CA question
  - direct resource inspection
  - profile validation

Done when:

- agents have a clear skill surface for both conversational and direct command
  workflows across Data Cloud products

### Milestone 6: Release Closure and v0.5.0

Deliverables:

- version bump to `0.5.0`
- release notes for the new command domains
- validation matrix for each source family
- final README roadmap update

Detailed tasks:

- bump Cargo, npm, and Gemini versions together
- update release docs and smoke-install checks
- verify command examples for each new source family
- add one consolidated source matrix doc showing:
  - supported commands
  - required profile fields
  - output modes
  - known limitations

Done when:

- `dcx` can credibly present itself as a direct Data Cloud CLI, not only a
  CA-oriented CLI

## Testing Strategy

Phase 5 should keep the repo's current validation shape:

- unit tests for parsing, normalization, and validation
- wiremock integration tests for request shapes and source-specific branches
- snapshot tests for `json`, `table`, and `text`
- fixture capture for live responses where product APIs are verbose or unstable
- optional live validation docs for pre-release runs

Recommended new fixture layout:

```text
tests/fixtures/sources/
├── looker/
├── spanner/
├── alloydb/
└── cloudsql/
```

Recommended rule:

- do not let CI depend on live source access for the Phase 5 command families
- capture enough real responses early to keep the normalized output contract
  honest

## Open Decisions

### 1. Should Phase 5 include any write commands?

Recommendation:
no by default. Keep the first Phase 5 cut read-only. If a single write command
is unavoidable, it should be explicit, isolated, and heavily validated.

### 2. Should profile commands live under `ca` or top-level `profiles`?

Recommendation:
top-level `profiles`. Profiles are no longer CA-only once `dcx` adds direct
source commands.

### 3. Should non-BigQuery commands be generated or hand-written?

**Decided (M3): Discovery-driven dynamic generation** for Spanner, AlloyDB,
and Cloud SQL. Google publishes Discovery docs for all three, and the
`ServiceConfig` abstraction made it straightforward to reuse the BigQuery
pipeline. This eliminated ~1,300 lines of hand-written code. Looker remains
hand-written because the Looker API is not a Google Discovery document.

### 4. Should Looker queries be part of Phase 5?

Recommendation:
not as a hard exit criterion. Explore and dashboard inspection are enough for
the first iteration. Query execution can follow once the metadata layer is
stable.

### 5. Should `dcx` adopt a `gcloud data-cloud` wrapper during Phase 5?

Recommendation:
no as a hard requirement. Keep the engine and command model moving first.
Wrapper packaging can follow once the source-native command surface proves
itself.

## Risks

### 1. Scope creep into full admin CLI territory

Risk:
the moment `dcx` adds product-native command domains, it becomes tempting to
grow into full CRUD and infra management.

Mitigation:
keep Phase 5 read-only first, and enforce explicit out-of-scope boundaries in
README and release notes.

### 2. Uneven API quality across products

Risk:
Looker, Spanner, AlloyDB, and Cloud SQL will not feel equally clean to
integrate, which can lead to inconsistent UX.

Mitigation:
normalize aggressively at the CLI output layer and keep the command families
small in v0.5.

### 3. Profile sprawl

Risk:
as more direct commands depend on profiles, profile fields may become harder to
reason about.

Mitigation:
keep one shared profile loader and add strict `profiles validate` coverage so
profile complexity is visible and testable.

### 4. Confusion between CA and direct commands

Risk:
users may not know when to use `ca ask` versus a direct source command.

Mitigation:
make this distinction central in docs and skills:
- use `ca ask` for natural-language exploration
- use direct commands for deterministic inspection, inventory, and setup

## Definition of Done

Phase 5 is complete when:

- `dcx` has direct command families for Looker, Spanner, AlloyDB, and Cloud SQL
- users can inspect and validate profiles without invoking CA
- at least one stable read-only inventory flow exists for each non-BigQuery
  Data Cloud source family
- docs and skills explain when to use direct commands versus `ca ask`
- the version, package, and release surfaces are updated to `0.5.0`

Recommended minimum exit examples:

- `dcx profiles validate --profile spanner-finance`
- `dcx looker explores list --profile looker-sales`
- `dcx spanner databases list --instance finance-prod`
- `dcx alloydb clusters list`
- `dcx cloudsql instances list`

## Suggested First PRs

1. `feat(profiles): add top-level profile utilities and shared profile resolver`
2. `feat(looker): add explores list/get and dashboards list/get`
3. `feat(databases): add spanner/alloydb/cloudsql inventory commands`
4. `feat(schema): add profile-aware schema describe helpers`
5. `docs(skills): add direct Data Cloud command skills and update roadmap`
6. `release: bump dcx to 0.5.0 for Phase 5 closeout`

## Recommended Build Order

Build Phase 5 in this order:

1. profiles
2. Looker metadata
3. database inventory
4. schema helpers
5. skills and docs
6. release closure

That order keeps the context layer stable first, then adds one product family at
a time, and only broadens docs and release surfaces once the command model is
real.
