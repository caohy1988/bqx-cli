# Plan: Keep `dcx analytics` Aligned with BigQuery Agent Analytics SDK

## Goal

Make `dcx analytics` track the
[BigQuery Agent Analytics SDK](https://github.com/haiyuan-eng-google/BigQuery-Agent-Analytics-SDK)
as the upstream product contract for BigQuery agent analytics.

The goal is not to blindly clone every Python library feature into the Rust CLI.
The goal is to make sure that when the SDK exposes a stable analytics workflow,
`dcx analytics` stays aligned in:

- command inventory
- flag names and semantics
- environment-variable behavior where applicable
- output schema and exit-code behavior
- evaluator names and execution semantics
- documentation and examples

## What Upstream Looks Like Today

Based on the SDK `README.md`, `SDK.md`, and `cli.py`, the upstream CLI surface
currently includes:

- `doctor`
- `get-trace`
- `evaluate`
- `insights`
- `drift`
- `distribution`
- `hitl-metrics`
- `list-traces`
- `views create-all`
- `views create`
- `categorical-eval`
- `categorical-views`

The upstream library also includes broader non-CLI capabilities:

- trace reconstruction and filtering
- LLM-as-judge
- multi-trial evaluation
- grader pipelines
- eval suite management
- eval validation
- long-horizon memory
- context graph
- remote function deployment
- continuous query templates

## Current `dcx analytics` Surface

`dcx analytics` currently ships:

- `doctor`
- `evaluate`
- `get-trace`
- `list-traces`
- `insights`
- `drift`
- `distribution`
- `hitl-metrics`
- `views create-all`

So the current gaps versus the SDK CLI are:

1. missing `views create`
2. missing `categorical-eval`
3. missing `categorical-views`
4. partial `evaluate` parity
5. partial flag parity on existing commands

## Alignment Principle

Treat the SDK as the source of truth for **BigQuery agent analytics**.

Treat `dcx` as the source of truth for:

- Rust CLI packaging and UX
- cross-source Data Cloud expansion outside BigQuery
- CLI-wide contracts such as `--sanitize`, JSON output, skill integration,
  shell completions, and Gemini manifest support

That means:

- SDK-defined analytics workflows should converge into `dcx analytics`
- `dcx` can add CLI-wide behavior, but should not silently drift on analytics
  semantics
- any intentional divergence must be documented explicitly

## Scope Boundary

To keep this tractable, split alignment into three layers.

### Layer 1: Required parity

These should stay aligned continuously:

- command names
- flag names
- evaluator names
- primary output fields
- exit-code semantics
- env-var shortcuts for project and dataset

### Layer 2: Fast-follow parity

These should land in `dcx` shortly after the SDK stabilizes them:

- new analytics CLI commands
- new evaluators and judge criteria
- new report fields that materially affect automation

### Layer 3: Optional CLI surfacing

These do not need immediate direct `dcx analytics` commands:

- multi-trial evaluation
- grader pipeline composition
- eval suite lifecycle APIs
- context graph APIs
- memory service
- deployment helpers such as remote functions and continuous queries

Those may later surface as:

- new `dcx analytics` commands
- doc-only references
- config-driven workflows
- or stay Python-library-only if they are not natural CLI operations

## Contract Model

Define one generated compatibility contract for analytics.

Recommended artifacts:

- `scripts/update_analytics_sdk_contract.sh`
- `tests/fixtures/upstream_sdk_latest/cli.py`
- `tests/fixtures/upstream_sdk_latest/SDK.md`
- `tests/fixtures/analytics_sdk_contract.json`
- `docs/analytics_sdk_contract.md`

It should record, per command:

- command name
- flags
- required vs optional flags
- env vars
- output format fields
- exit-code semantics
- upstream source file / doc reference
- parity status:
  - `exact`
  - `intentional divergence`
  - `missing`
  - `dcx extension`

This becomes the artifact reviewers use instead of ad hoc comparisons.

## Current Gap Map

### 1. Command parity gaps

Need to add:

- `dcx analytics views create`
- `dcx analytics categorical-eval`
- `dcx analytics categorical-views`

### 2. `evaluate` parity gaps

SDK supports evaluator values:

- `latency`
- `error_rate`
- `turn_count`
- `token_efficiency`
- `ttft`
- `cost`
- `llm-judge`

`dcx` currently supports a narrower set and uses kebab-case names such as
`error-rate`.

Plan:

- support both the SDK canonical names and `dcx` aliases where needed
- choose one canonical JSON output spelling
- document aliases explicitly

### 3. Existing command flag gaps

Examples from the SDK CLI:

- `get-trace` supports `--session-id` or `--trace-id`
- `insights` supports `--max-sessions`
- `distribution` supports `--mode` and `--top-k`
- `views create` exists alongside `views create-all`

Each existing `dcx` command should be audited against the SDK definition.

### 4. Output-shape gaps

Even when command names match, the automation contract may drift.

Need to align:

- top-level JSON field names
- evaluation summary fields
- per-session result fields
- trace payload shape
- error envelope expectations where analytics commands wrap SDK concepts

`dcx` can preserve its global `{"error":"..."}` envelope, but success payloads
should stay close to SDK report structures wherever practical.

## Proposed Architecture

### 1. Dynamic upstream fetch + checked-in cache

Add a small updater that fetches the latest upstream SDK contract from:

- `main/src/bigquery_agent_analytics/cli.py`
- `main/SDK.md`
- optionally `main/README.md`

Store the fetched files under:

- `tests/fixtures/upstream_sdk_latest/`

Then generate:

- `tests/fixtures/analytics_sdk_contract.json`
- `docs/analytics_sdk_contract.md`

Recommended operating model:

- **scheduled update job:** fetch latest SDK `main`, regenerate the contract,
  and open a PR when the mapping changes
- **normal CI:** use the checked-in generated files only

That gives `dcx` a dynamic mapping from the latest SDK without making routine
CI runs depend on live GitHub availability.

### 2. Generated compatibility table in-repo

Generate one compatibility table that maps:

- upstream SDK command -> `dcx analytics` command
- upstream flag -> `dcx` flag
- status and rationale

This should live in version control and be regenerated whenever:

- the upstream SDK changes
- `dcx analytics` changes

Reviewers should edit only intentional divergence notes, not the generated
command inventory itself.

### 3. Golden CLI contract tests

Add tests that assert:

- help output includes the expected commands and flags
- evaluator enum accepts the expected values
- JSON payload contains required keys
- exit codes match SDK semantics where defined

These should be local contract tests in Rust, driven by the checked-in
generated contract, not live SDK integration tests.

### 4. Intentional divergence registry

Some divergence is fine, but it must be explicit.

Examples likely to remain divergent:

- `--sanitize`
- `json|table|text` renderer details
- `DCX_*` environment variables vs `BQ_AGENT_*`
- cross-source Data Cloud concepts outside BigQuery analytics

Known current divergences to seed into the initial generated contract:

- `--table` (dcx) vs `--table-id` (SDK) — flag name mismatch
- `--location` defaults to `"US"` in dcx, `None` in SDK
- `drift --min-coverage` and `drift --exit-code` — dcx extensions, not in SDK
- infrastructure error exit code: SDK uses exit 2, dcx does not
- `--limit` missing on `evaluate`, `insights`, `drift`, `distribution` in dcx

Track these in the compatibility table with a reason.

## Dynamic Mapping Flow

### Updater inputs

The updater should parse the latest SDK sources for:

- command names from `@app.command` and `views_app.command`
- flags/options from Typer definitions
- evaluator names from `_CODE_EVALUATORS`
- judge criteria from `_LLM_JUDGES`
- env vars from shared options
- output and exit-code notes from `SDK.md`

### Updater outputs

The updater should emit:

1. `analytics_sdk_contract.json`
2. `analytics_sdk_contract.md`
3. a machine-readable diff summary such as:
   - new upstream commands
   - removed upstream commands
   - new flags
   - removed flags
   - changed evaluator values

### CI policy

- pull request CI should validate `dcx` against the checked-in generated
  contract
- a scheduled workflow should refresh from latest SDK `main`
- when upstream changes, the workflow should open or update a tracking PR

This is the safest way to get "latest SDK" mapping without making local builds
or PR CI flaky.

## Milestones

### Milestone A: Build the dynamic contract generator

Deliverables:

- add `scripts/update_analytics_sdk_contract.sh`
- add fetched upstream fixtures under `tests/fixtures/upstream_sdk_latest/`
- generate `docs/analytics_sdk_contract.md`
- generate `tests/fixtures/analytics_sdk_contract.json`
- classify every current difference as exact / missing / intentional

Done when:

- every `dcx analytics` command is mapped to an upstream SDK command or marked
  as `dcx`-specific by generated output

### Milestone B: Reach CLI command parity

Deliverables:

- add `dcx analytics views create`
- add `dcx analytics categorical-eval`
- add `dcx analytics categorical-views`

Done when:

- `dcx analytics` matches the current SDK command inventory for stable CLI
  analytics workflows

### Milestone C: Reach flag and evaluator parity

Deliverables:

- add missing evaluator values (`ttft`, `cost`, `llm-judge`)
- support SDK-compatible evaluator spellings
- add missing flags on `get-trace`, `insights`, `distribution`, and others as
  required by the contract table

Done when:

- the compatibility table shows no `missing` items for stable flags/evaluators

### Milestone D: Reach output and exit-code parity

Deliverables:

- align success JSON payloads where practical
- document any remaining intentional differences
- add regression tests for output keys and exit-code behavior

Done when:

- analytics automation examples from SDK docs can be translated to `dcx`
  without semantic surprises

### Milestone E: Automate drift detection

Deliverables:

- add a script that refreshes from latest upstream SDK `main`
- add a scheduled workflow that regenerates the mapping and opens a PR
- add a CI check that fails when `dcx` no longer matches the checked-in
  generated contract
- open a recurring tracking issue or scheduled check

Done when:

- SDK analytics changes produce a visible, reviewable generated delta in `dcx`

## Operational Policy

Use this rule for future changes:

1. if the SDK adds or changes a stable analytics CLI command, open a matching
   `dcx analytics` tracking issue within one release cycle
2. if `dcx` intentionally diverges, document it in the compatibility table in
   the same PR
3. do not merge analytics UX changes in `dcx` without checking the upstream SDK
   contract

## Recommended First PRs

### PR 1: Dynamic contract generator

- add updater script
- add generated contract JSON/Markdown
- add fetched upstream fixtures

### PR 2: Missing command parity

- add `views create`
- add `categorical-eval`
- add `categorical-views`

### PR 3: Evaluator parity

- add `ttft`
- add `cost`
- add `llm-judge`
- support SDK naming aliases

### PR 4: Output and exit-code audit

- align output keys
- align exit behavior
- add regression coverage

## Definition of Done

This alignment effort is complete when:

- `dcx analytics` covers the stable SDK CLI analytics commands
- evaluator and flag semantics are documented and tested
- all remaining differences are intentional and written down
- the latest SDK contract can be re-fetched and regenerated by script
- CI has a lightweight drift check so the two surfaces do not silently diverge
- a scheduled updater keeps `dcx` tracking latest upstream changes
