# Phase 4 Plan: v0.3 to v0.4 (Proposed)

## Goal

Move `bqx` from the current Phase 3 state to a proposed Phase 4 focused on
safe operations, broader distribution, and higher-confidence automation.

Unlike [PHASE1_PLAN.md](/Users/haiyuancao/bqx-cli/PHASE1_PLAN.md),
[PHASE2_PLAN.md](/Users/haiyuancao/bqx-cli/PHASE2_PLAN.md), and
[PHASE3_PLAN.md](/Users/haiyuancao/bqx-cli/PHASE3_PLAN.md), this plan is **not**
derived from a committed roadmap section in
[README.md](/Users/haiyuancao/bqx-cli/README.md). The README currently ends at
Phase 3, so this document is a proposed next-phase plan based on:

- the current repo state after Phase 3
- the existing architecture boundaries
- the most natural next steps for product value and release impact

Recommended Phase 4 theme:

- safe BigQuery write and mutation workflows
- broader installation surface beyond npm-only
- live CA validation in CI
- a lightweight watch mode for operational dashboards
- an initial migration/bootstrap workflow for common analytics setups

Versioning note:
the repo is currently at `0.3.0` in
[Cargo.toml](/Users/haiyuancao/bqx-cli/Cargo.toml). This proposal treats
Phase 4 as the path to `0.4.0`.

## Proposed Outcome

By the end of the proposed Phase 4, `bqx` should be able to:

- safely create, update, and delete a small curated set of BigQuery resources
- expose explicit mutation safety rails for both humans and agents
- install through Homebrew in addition to npm
- run a scheduled live CA smoke check in CI
- support a basic `watch` mode for recurring analytics views
- bootstrap or evolve a standard analytics dataset layout through an initial
  migration workflow

This phase should **not** attempt full BigQuery parity. The right target is a
small, high-signal operational surface that complements the current read,
analytics, and CA workflows.

## Baseline (post-Phase 3)

Phase 3 established a complete observability and CA baseline:

- static command tree for:
  - `jobs query`
  - `analytics doctor|evaluate|get-trace|list-traces|insights|drift|distribution|hitl-metrics|views`
  - `auth login|status|logout`
  - `ca ask|create-agent|list-agents|add-verified-query`
  - `generate-skills`
  - `completions`
- dynamic read-only BigQuery API commands generated from Discovery:
  - `datasets list|get`
  - `tables list|get`
  - `routines list|get`
  - `models list|get`
- JSON-first output plus `table` and `text`
- shared auth, config, sanitize, output, and BigQuery client layers under
  [src/](/Users/haiyuancao/bqx-cli/src)
- 26 checked-in skills under [skills/](/Users/haiyuancao/bqx-cli/skills)
- Gemini extension manifest and release automation
- npm distribution via `bqx`
- bash/zsh/fish completions
- live e2e validation docs in
  [docs/e2e-validation.md](/Users/haiyuancao/bqx-cli/docs/e2e-validation.md)

What the current repo still does **not** provide:

- safe write or delete coverage for BigQuery resources through the dynamic API
- an explicit mutation policy layer
- Homebrew distribution
- automated live CA validation in CI
- a first-class `watch` command for recurring analytics refresh
- migration/bootstrap commands for analytics schema and view setup

## Scope

In scope:

- a curated write/mutation surface for BigQuery resources
- mutation-specific safety UX and policy enforcement
- Homebrew distribution and release automation
- scheduled or manually triggered live CA e2e workflow in CI
- a narrow watch mode for existing analytics commands
- an initial migration/bootstrap workflow for common agent analytics resources
- docs, examples, and release validation for the Phase 4 surface

Out of scope:

- full CRUD coverage for every Discovery method
- arbitrary DDL orchestration for all BigQuery resource types
- replacing the current static analytics or CA command families
- long-running daemon infrastructure
- a general-purpose migration engine for every possible user schema
- additional conversational features beyond the existing CA flows

## Architecture Direction

Keep the existing split:

- dynamic resource-oriented BigQuery API commands remain the right place for
  generic BigQuery resource operations
- analytics and CA commands remain hand-written static commands
- mutation support should extend the Phase 2 dynamic system with policy and
  request-body support, not bypass it
- watch and migrate should remain static command domains because they encode
  product workflows, not raw API method exposure

Target modules:

```text
src/
├── bigquery/
│   ├── client.rs
│   ├── discovery.rs
│   ├── dynamic/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── clap_tree.rs
│   │   ├── request_builder.rs
│   │   ├── executor.rs
│   │   ├── body_builder.rs
│   │   └── policy.rs
├── commands/
│   ├── analytics/
│   ├── ca/
│   ├── jobs_query.rs
│   ├── generate_skills.rs
│   ├── watch.rs
│   └── migrate/
│       ├── mod.rs
│       ├── init.rs
│       ├── plan.rs
│       └── apply.rs
├── cli.rs
├── config.rs
├── output.rs
└── main.rs
brew/
└── bqx.rb                     # generated or checked-in formula, depending on release model
docs/
├── e2e-validation.md
├── homebrew.md
└── migrations.md
tests/
├── dynamic_mutation_tests.rs
├── watch_tests.rs
├── migration_tests.rs
└── ca_live_docs_checks.rs     # doc/example validation, not live API CI
```

Recommended implementation rule:

- Phase 4 should extend the current architecture, not introduce a fourth
  competing command-generation path
- every mutating path must have a non-executing preview mode
- every mutation must validate locally before auth and network
- release and CI automation should remain deterministic even if live CA tests
  are flaky or preview-limited

## Core Design

### 1. Mutation Safety Model

Phase 4 should begin by making the dynamic command layer safe for a curated set
of write operations.

Recommended command behavior for mutating dynamic commands:

- `--dry-run`: print the exact HTTP method, URL, query params, and request body
  without sending the request
- `--yes`: skip confirmation prompt in interactive contexts
- non-TTY mutation runs without `--yes` should fail with a clear error rather
  than implicitly proceeding
- JSON error envelope remains `{"error":"..."}`

Recommended policy metadata on generated commands:

- `read`
- `write`
- `delete`
- `destructive`

Suggested internal fields:

- `GeneratedCommand::safety_class`
- `GeneratedCommand::supports_body`
- `GeneratedCommand::requires_confirmation`

### 2. Curated Mutation Surface

Do not broaden Discovery coverage blindly. Start with a short allowlist that
maps to real operational tasks:

- `datasets insert`
- `datasets delete`
- `tables delete`
- `tables patch` for low-risk metadata updates
- `jobs cancel`

Optional second wave:

- `routines delete`
- `models delete`
- `tables insert` for simple logical views only

Recommended rule:

- Phase 4 should support only those write methods for which the CLI can expose
  a clear, validated, minimally surprising contract

### 3. Request Body Strategy

Phase 2 only needed path and query params. Phase 4 mutations require request
body support.

Recommended design:

- keep generated flags scalar and explicit for the first mutation wave
- avoid accepting raw JSON blobs as the primary UX for common operations
- use hand-written body builders for the initial write allowlist where the body
  shape would otherwise become confusing

Examples:

- `datasets insert --dataset-id analytics --location US`
- `tables patch --table-id agent_events --description "..." --expiration-ms 86400000`
- `jobs cancel --job-id ... --location us`

This keeps mutations understandable and safer than passing opaque payloads.

### 4. Watch Mode

The repo already has strong `text` and `table` renderers. Phase 4 can reuse
those for a lightweight refresh loop rather than inventing a dashboard stack.

Recommended initial command surface:

- `bqx watch insights --last=1h --every=60s`
- `bqx watch evaluate --evaluator latency --threshold 5000 --last=1h`
- `bqx watch list-traces --last=15m --limit=20`

Recommended behavior:

- clear and redraw for `text` and `table`
- disable `watch` for `json`
- support `--iterations` for CI/demo determinism

### 5. Migration Workflow

The migration story should stay narrow and opinionated at first.

Recommended initial command surface:

- `bqx migrate init`
- `bqx migrate plan`
- `bqx migrate apply`

Recommended first use cases:

- create or update the standard `agent_events` dataset/table contract
- create standard analytics views
- create or validate `golden_questions` support tables

Recommended state model:

- file-backed migration specs in a checked-in directory such as `migrations/`
- deterministic plan output before apply
- no automatic destructive schema rewrites in the initial version

### 6. Homebrew Distribution

M7/M8 from Phase 1 made npm the primary install path. Phase 4 should add one
more mainstream install surface without exploding packaging complexity.

Recommended approach:

- publish GitHub Release binaries as the source of truth
- generate or maintain a Homebrew formula that installs the released binary
- keep Homebrew as install-only; do not duplicate release build logic there

Recommended non-goal:

- skip apt/yum in Phase 4 unless Homebrew is already stable and cheap

### 7. Live CA Validation in CI

The repo already documents live CA validation. Phase 4 should automate a small
slice of it.

Recommended workflow:

- scheduled job using Workload Identity Federation
- dedicated test project, dataset, and CA agent
- run:
  - `bqx ca list-agents`
  - `bqx ca ask ...`
  - optionally `bqx ca add-verified-query` against a disposable agent

Recommended reliability rule:

- live CA checks should start as non-blocking scheduled or manually triggered
  workflows, not required PR gates

## Proposed Milestones

### Milestone 1: Mutation Foundation

Objective:
extend the dynamic API layer to support a safe curated set of mutation methods.

Tasks:

- add safety metadata to generated commands in
  [src/bigquery/dynamic/model.rs](/Users/haiyuancao/bqx-cli/src/bigquery/dynamic/model.rs)
- add request-body support in a new
  [src/bigquery/dynamic/body_builder.rs](/Users/haiyuancao/bqx-cli/src/bigquery/dynamic/body_builder.rs)
- add mutation policy checks in
  [src/bigquery/dynamic/policy.rs](/Users/haiyuancao/bqx-cli/src/bigquery/dynamic/policy.rs)
- extend dynamic clap generation for:
  - `--dry-run`
  - `--yes`
  - explicit mutation flags per allowlisted method
- keep validation-before-auth for all mutation inputs

Done when:

- at least 3 write/delete commands work end to end
- all mutating commands support `--dry-run`
- destructive commands do not proceed in TTY mode without explicit confirmation
- destructive commands fail in non-TTY mode without `--yes`

### Milestone 2: First-Class Mutation Commands

Objective:
ship a useful operational write surface, not just mutation plumbing.

Recommended first command set:

- `datasets insert`
- `datasets delete`
- `tables delete`
- `tables patch`
- `jobs cancel`

Tasks:

- add allowlist coverage and tests per method
- add explicit request-body builders where needed
- add `json`, `table`, and `text` output rules for mutation results
- add docs and examples for each new operation

Done when:

- all initial mutation commands work against live BigQuery
- error messages clearly identify missing confirmation or invalid body fields
- tests cover dry-run, confirm, and non-interactive failure cases

### Milestone 3: Homebrew Distribution

Objective:
make `bqx` installable through Homebrew in addition to npm.

Tasks:

- choose formula location:
  - in-repo formula
  - separate tap repo
- generate or update formula from release metadata
- add release automation for formula refresh
- document Homebrew install and upgrade flow

Done when:

- `brew install bqx` or `brew install <tap>/bqx` works on a clean macOS machine
- Homebrew install path uses the same released binaries as npm
- release docs explain npm vs Homebrew support

### Milestone 4: Live CA CI Validation

Objective:
automate the most important live CA smoke checks.

Tasks:

- provision a dedicated CI CA project/dataset/agent
- add scheduled or manual GitHub Actions workflow with WIF auth
- validate:
  - `ca list-agents`
  - `ca ask`
  - one verified-query workflow
- record failure handling and retry policy

Done when:

- CI can run a documented live CA workflow without local secrets
- failures are visible but do not block unrelated PRs by default
- docs distinguish unit/integration coverage from live CA smoke coverage

### Milestone 5: Watch Mode

Objective:
add a lightweight recurring refresh workflow for operations and demos.

Tasks:

- add `watch` command family in [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- implement:
  - `watch insights`
  - `watch evaluate`
  - `watch list-traces`
- add `--every`, `--iterations`, and `--no-clear`
- reuse existing `text` and `table` renderers

Done when:

- watch mode works in terminal for at least 3 analytics commands
- CI/demo can run deterministic watch tests with `--iterations`
- watch mode handles failures without leaking terminal state

### Milestone 6: Migrate Alpha + Docs Closeout

Objective:
ship a narrow migration/bootstrap workflow and close the proposed Phase 4 docs.

Tasks:

- add `migrate init|plan|apply`
- define migration file layout under `migrations/`
- support bootstrap of:
  - analytics dataset/table contract
  - standard views
  - optional golden questions support objects
- update:
  - [README.md](/Users/haiyuancao/bqx-cli/README.md)
  - [docs/e2e-validation.md](/Users/haiyuancao/bqx-cli/docs/e2e-validation.md)
  - new docs for Homebrew and migrations
- bump version to `0.4.0`

Done when:

- a user can initialize and apply the supported bootstrap migrations
- release docs cover npm, Homebrew, live CI validation, and mutation safety
- `0.4.0` release artifacts and package metadata are in sync

## Recommended Build Order

1. Mutation foundation
2. First-class mutation commands
3. Homebrew distribution
4. Live CA CI validation
5. Watch mode
6. Migrate alpha and release closeout

Rationale:

- mutation support is the highest product-value gap after Phase 3
- Homebrew is simpler once release artifacts are already stable
- live CA CI depends on the existing CA surface, not on watch or migrate
- watch mode is useful but not foundational
- migrate should come last because it is the most opinionated and easiest to
  overbuild

## Testing Strategy

Mutation coverage:

- unit tests for body builders and safety policies
- fixture-backed command tests for validation-before-auth behavior
- wiremock or mock executor tests for request shape
- live manual/e2e checks for the first allowlisted mutation set

Homebrew coverage:

- formula lint or install smoke check in CI
- install-from-release validation on macOS

CA live CI:

- scheduled workflow, not PR-blocking
- clear separation between mock tests and live smoke results

Watch mode:

- deterministic snapshot tests using `--iterations=1`
- renderer tests for cleared and non-cleared output

Migration coverage:

- plan/apply unit tests on fixture migration directories
- dry-run style tests before any live apply examples

## Open Decisions

### 1. Mutation breadth

Question:
how many write methods should Phase 4 actually ship?

Recommended answer:
start with 5 or fewer methods and prove the safety model before broadening.

### 2. Confirmation UX

Question:
should destructive commands prompt interactively, or should all mutations
require `--yes`?

Recommended answer:
prompt in TTY mode, require `--yes` in non-TTY mode.

### 3. Request body UX

Question:
should mutating commands accept raw JSON payloads?

Recommended answer:
not by default. Prefer explicit scalar flags for the first wave, with raw JSON
reserved for later escape-hatch workflows if needed.

### 4. Homebrew formula location

Question:
should the formula live in this repo or a separate tap?

Recommended answer:
start with an in-repo formula if the release workflow can update it cleanly;
switch to a tap only if Homebrew publishing mechanics require it.

### 5. Watch command shape

Question:
should `watch` be a top-level command or nested under `analytics`?

Recommended answer:
make it top-level. `watch` is execution behavior, not just another analytics
result type.

### 6. Migration scope

Question:
should `migrate` target general BigQuery schema evolution or only the bqx
analytics conventions?

Recommended answer:
keep it bqx-specific at first. General schema migration tooling is too broad.

## Risks

### 1. Destructive-command risk

Risk:
Phase 4 introduces real write/delete behavior, which is a different risk class
from the current repo.

Mitigation:

- keep a short allowlist
- require dry-run/confirm behavior
- validate locally before auth/network
- document destructive semantics clearly

### 2. Discovery/body-shape complexity

Risk:
write methods need request bodies and Discovery schemas are more complex than
the current read-only set.

Mitigation:

- use hand-written body builders for the first mutation wave
- avoid claiming generic body support until it is actually stable

### 3. Packaging sprawl

Risk:
adding Homebrew plus npm plus GitHub release automation can become fragile.

Mitigation:

- keep GitHub Release binaries as the single artifact source of truth
- reuse existing release jobs where possible

### 4. CA preview instability

Risk:
live CA CI may be noisy because the upstream API is still preview-dependent.

Mitigation:

- keep live CA checks scheduled/manual first
- do not gate normal PRs on preview API health

### 5. Overbuilding migrate

Risk:
migration tooling can easily become an entire product on its own.

Mitigation:

- keep Phase 4 migrate limited to analytics bootstrap and standard view setup
- defer broad schema-diff ambitions

## Definition of Done

This proposed Phase 4 is complete when:

- `bqx` supports a safe curated set of BigQuery mutation commands
- every mutating command has dry-run and confirmation behavior
- Homebrew install works from released binaries
- live CA smoke validation runs in CI on a scheduled or manual basis
- `watch` supports at least 3 analytics workflows
- `migrate init|plan|apply` works for the supported bootstrap layout
- docs and release automation reflect the `0.4.0` install and safety story

## Suggested First PRs

1. `feat(dynamic): add mutation policy metadata and dry-run/confirm plumbing`
2. `feat(dynamic): add datasets insert/delete and tables delete`
3. `feat(packaging): add Homebrew formula generation`
4. `ci(ca): add scheduled live CA smoke workflow`
5. `feat(watch): add watch insights and evaluate`
6. `feat(migrate): add migrate init and plan`

## Recommended Starting Point

If Phase 4 starts immediately, the right first PR is Milestone 1:

- add mutation safety metadata to the dynamic command model
- add request body support for a tiny allowlist
- add `--dry-run` and `--yes` semantics
- prove the pattern with one non-destructive write path before expanding

That gives the repo the highest-leverage new capability without committing yet
to broader packaging or migration work.
