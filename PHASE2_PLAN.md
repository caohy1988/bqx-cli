# Phase 2 Plan: v0.1 to v0.2

## Goal

Move `dcx` from the Phase 1 target to the Phase 2 target defined in
[README.md](/Users/haiyuancao/bqx-cli/README.md):

- Discovery Document fetching and caching
- dynamic `clap::Command` generation for the BigQuery v2 API
- `dcx generate-skills`
- non-CA curated skills:
  - 1 shared
  - 6 BigQuery API service
  - 4 analytics helpers
  - 2 generic helpers
  - 2 CA-independent personas
  - 4 non-CA recipes
- Model Armor integration via `--sanitize`
- Gemini CLI extension registration

Authority note:
this plan targets the README Phase 2 roadmap in
[README.md](/Users/haiyuancao/bqx-cli/README.md), not earlier MVP or Phase 1
stopping points.

Phase 2 is not about adding broad new analytics features. It is about making
`dcx` dynamic, skill-generating, and integrable as a reusable agent tool
surface beyond the hand-written Phase 1 command set.

README alignment notes:

- the README Phase 2 roadmap is authoritative for scope and exit criteria
- the README's `generate-skills` examples are broader than the roadmap bullets;
  this plan interprets them narrowly for Phase 2:
  - generated output should cover BigQuery API service families and a small
    number of deterministic helper skills
  - analytics helper skills, personas, and recipes remain curated unless they
    can be generated from explicit templates without inventing workflow logic
- Gemini extension registration is a packaging/integration task after the
  dynamic command surface is already stable, not a driver of Phase 2 command
  design

## Baseline (pre-Phase 2)

Phase 1 established the static command and packaging foundation:

- Rust CLI scaffold with `clap` in [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- static command tree for:
  - `jobs query`
  - `analytics doctor`
  - `analytics evaluate`
  - `analytics get-trace`
  - `auth login|status|logout`
- shared config, auth, output, and BigQuery client layers under
  [src/](/Users/haiyuancao/bqx-cli/src)
- JSON-first output contract plus `table` and `text`
- test seams for command logic and BigQuery HTTP behavior
- skills shipped as hand-written `SKILL.md` files
- npm packaging and release automation for the Phase 1 binary/install story

What Phase 2 added (all complete — see milestones below):

- Discovery Document ingestion with bundled pinned copy (M1, PR #13)
- dynamic command model with read-only allowlist of 8 methods (M2, PR #14)
- generated BigQuery API commands: `datasets list/get`, `tables list/get`,
  `routines list/get`, `models list/get` (M2, PR #14)
- skill generation pipeline: `dcx generate-skills` (M3, PR #16)
- 19 non-CA skills: 4 generated + 15 curated (M4, PR #17)
- Model Armor integration via `--sanitize` with regional endpoints (M5, PR #18)
- Gemini extension manifest packaged and validated (M5, PR #18)
- docs, e2e validation, and exit-criteria closure (M6, PR #19)

## Scope

In scope:

- Discovery Document acquisition, pinning, and cache semantics
- dynamic command generation for BigQuery v2 resources
- execution of generated BigQuery API commands
- skill generation for generated command surfaces
- non-CA curated skill expansion
- request sanitization plumbing for `--sanitize`
- Gemini extension registration packaging
- docs and examples for the dynamic command surface

Out of scope:

- Conversational Analytics commands (`dcx ca *`)
- CA-dependent skills
- additional analytics commands listed for Phase 3
- broad plugin architecture beyond Gemini extension registration
- replacing or rewriting the static Phase 1 analytics/auth command flows
- full declarative policy engine for command generation

## Architecture Direction

Keep the existing Phase 1 layers and add a dynamic API layer beside the
hand-written command tree. Do not rewrite the analytics/auth commands into the
generated system in Phase 2.

Target modules:

```text
src/
├── auth/
├── bigquery/
│   ├── client.rs
│   ├── discovery.rs
│   ├── dynamic/
│   │   ├── mod.rs
│   │   ├── model.rs
│   │   ├── clap_tree.rs
│   │   ├── request_builder.rs
│   │   └── executor.rs
│   └── sanitize.rs
├── commands/
│   ├── analytics/
│   ├── auth/
│   ├── jobs_query.rs
│   └── generate_skills.rs
├── skills/
│   ├── generator.rs
│   ├── templates.rs
│   └── curated.rs
├── integrations/
│   └── gemini.rs
├── cli.rs
├── config.rs
├── output.rs
└── main.rs
tests/
├── fixtures/
│   ├── discovery/
│   └── dynamic_api/
├── snapshot_tests.rs
├── dynamic_command_tests.rs
└── gemini_tests.rs
```

Directory note:

- `src/skills/` is Rust implementation code for skill generation, templating,
  and curated-skill metadata
- `skills/` at the repo root remains the output tree containing checked-in
  `SKILL.md` files and `agents/openai.yaml` metadata
- Phase 2 should keep those roles separate so implementers do not confuse
  generator code with generated or curated skill artifacts

## Core Design

### 1. Discovery Model

Phase 2 should treat the BigQuery Discovery Document as versioned input data,
not as a runtime-only implementation detail.

Required behavior:

- fetch the BigQuery v2 Discovery Document from Google
- cache it locally under a deterministic path
- keep a bundled pinned fallback copy for offline and CI use
- expose explicit source control:
  - bundled
  - cache
  - remote

Recommended CLI surface:

- `--discovery bundled`
- `--discovery cache`
- `--discovery remote`
- optional `dcx discovery refresh`

Recommended flag scope:

- treat `--discovery` as a dynamic-command concern first, not a universal flag
  for every static command
- if it is implemented as a global flag for technical simplicity, document
  clearly that static Phase 1 commands ignore it

Recommended cache location:

- config/state directory under the user cache dir
- file name including API name and revision hash

Phase 2 should remain deterministic by default:

- default to bundled pinned document
- require explicit refresh or explicit remote mode to change behavior

### 2. Dynamic Command Model

Generated commands should be modeled as data first, then rendered into `clap`.

Recommended internal model:

- service
- resource
- method
- path
- HTTP verb
- required params
- optional params
- request body schema
- response schema metadata

Suggested structs:

- `ApiMethod`
- `ApiParam`
- `ApiSchemaRef`
- `GeneratedCommand`
- `GeneratedArgument`

Do not generate commands directly from JSON into `clap` in one step. Keep the
intermediate model stable and testable.

### 3. Static + Dynamic Command Split

Phase 1 commands remain hand-written:

- `auth *`
- `analytics doctor`
- `analytics evaluate`
- `analytics get-trace`
- `jobs query`

Phase 2 generated commands live beside them:

- `datasets list`
- `datasets get`
- `tables list`
- `tables get`
- `jobs get`
- other non-destructive, high-value BigQuery v2 methods

Do not attempt to dynamically generate every method on day one. Start with a
curated allowlist of stable read-focused methods, then broaden.

### 4. Output Model for Dynamic Commands

Generated commands should still respect the Phase 1 output contract:

- `json` remains the default
- `table` should work for list-like responses
- `text` is optional for generated commands in Phase 2 unless there is a
  compelling command-specific formatter

Recommended rule:

- dynamic commands must support `json`
- dynamic list/get commands should support `table`, but Phase 2 needs an
  explicit flattening rule for nested BigQuery API responses
- `text` may fall back to `table`-like or JSON-pretty behavior for generated
  commands until a better pattern exists

Recommended table strategy:

- flatten one level of common reference objects such as `datasetReference` and
  `tableReference`
- render scalar summary columns in `table`
- keep deeply nested schema payloads in `json` by default rather than forcing
  lossy table output

### 5. Skill Generation Model

`dcx generate-skills` should generate:

- `SKILL.md`
- `agents/openai.yaml`

for the generated command families that are useful to agents.

It should not try to infer sophisticated domain guidance from raw Discovery
metadata alone. Generated skills should be intentionally minimal and focus on:

- what command family does
- which arguments matter
- examples
- output conventions

Use templates plus Discovery metadata, not freeform generation.

## Milestones

### Milestone 1: Discovery and Dynamic API Model — Complete (PR #13)

Objective:
establish Discovery as deterministic input and parse it into a stable internal
command model before changing the user-facing CLI.

Tasks:

- add `src/bigquery/discovery.rs`
- define a `DiscoverySource` enum:
  - `Bundled`
  - `Cache`
  - `Remote`
- add a bundled pinned BigQuery v2 Discovery JSON asset
- implement cache read/write helpers plus explicit refresh behavior
- define structs for methods, params, schemas, and resources
- parse Discovery JSON into the internal model
- create an allowlist of Phase 2 methods to expose
- normalize BigQuery method names into CLI-safe names
- map Discovery metadata into generated argument metadata without touching
  `clap` yet

Recommended initial allowlist:

- `datasets.list`
- `datasets.get`
- `tables.list`
- `tables.get`
- `jobs.get`
- optionally `routines.get` or `models.get` only if the model remains clean

Done when:

- `dcx` can load Discovery without network access
- the parsed internal model is snapshot-tested and independent of `clap`
- CI can use bundled Discovery deterministically

### Milestone 2: Generated Command Surface — Complete (PR #14)

Objective:
turn the internal model into a runtime command tree and execute the first
generated BigQuery commands end to end.

Tasks:

- add `src/bigquery/dynamic/clap_tree.rs`
- add `src/bigquery/dynamic/request_builder.rs`
- inject generated subcommands into the top-level CLI at startup
- preserve static command precedence
- map generated arguments into:
  - REST path params
  - query params
  - request body where needed
- reuse auth, output, and error plumbing from Phase 1
- add tests for:
  - generated help output
  - required parameter enforcement
  - path/query composition
  - JSON response rendering
  - validation-before-auth behavior for generated commands

Recommended strategy:

- keep the existing typed `Cli` path for static Phase 1 commands
- use a hybrid runtime `clap::Command` path for generated BigQuery resources
- keep Phase 2 generated methods read-focused first

Done when:

- `dcx datasets list --help` works without any hardcoded `datasets` command in
  [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- `dcx datasets list` and `dcx tables get` work end to end from generated
  metadata

### Milestone 3: Skill Generation Pipeline — Complete (PR #16)

Objective:
ship `dcx generate-skills` for deterministic generation from the stable Phase 2
command surface.

Tasks:

- add `src/commands/generate_skills.rs`
- add template builders under [src/skills/](/Users/haiyuancao/bqx-cli/src/skills)
- generate:
  - `SKILL.md`
  - `agents/openai.yaml`
- allow output directory override
- support `--filter` so the CLI can regenerate only selected skill families
- generate only for command families that have explicit templates
- add snapshot tests for generated skill files

Recommended generated output in Phase 2:

- BigQuery API service skills such as `dcx-datasets`, `dcx-tables`, `dcx-jobs`
- deterministic helper skills only when backed by explicit templates
- no persona or recipe generation from raw Discovery metadata

Done when:

- `dcx generate-skills --output-dir=./skills` emits deterministic skill
  directories
- `dcx generate-skills --filter=dcx-analytics` has a clear and supported
  behavior
- generated skills are snapshot-tested and reviewable

### Milestone 4: Curated Non-CA Skill Expansion — Complete (PR #17)

Objective:
expand from the Phase 1 skill set to the full non-CA Phase 2 set without
mixing generated service skills and curated workflow skills.

Target additions from the README:

- 1 shared skill total
- 6 BigQuery API service skills
- 4 analytics helper skills
- 2 generic helper skills
- 2 CA-independent personas
- 4 non-CA recipes

Count clarification:

- the README's 19-skill figure is the total non-CA Phase 2 set, not 19 net-new
  skills on top of Phase 1
- Phase 1 already shipped 5 overlapping skills:
  - `dcx-shared`
  - `dcx-analytics`
  - `dcx-analytics-evaluate`
  - `dcx-analytics-trace`
  - `dcx-query`
- Phase 2 therefore adds 14 net-new skills while reorganizing the full 19-skill
  set into generated service skills plus curated workflow skills

Tasks:

- define the exact 19-skill Phase 2 list in the repo
- separate generated service skills from curated helper/persona/recipe skills
  on disk and in docs
- ensure every example references commands that actually exist in Phase 2
- add `agents/openai.yaml` for each curated skill
- add consistency review:
  - no CA dependencies
  - no unimplemented commands
  - examples align with Phase 2 output shapes

Scope rule:

- generated skills cover stable API/service families
- curated skills cover analytics workflows, personas, and recipes

Done when:

- 19 non-CA skills are present and internally consistent
- the generated-vs-curated split is explicit in the repo layout and docs

### Milestone 5: Sanitization and Integration Surfaces — Complete (PR #18)

Objective:
add the remaining Phase 2 integration hooks after the dynamic CLI and skill
surfaces are stable.

Tasks:

- define what `--sanitize` means in Phase 2:
  - response redaction first
  - request sanitization only if explicitly justified
- add `src/bigquery/sanitize.rs`
- integrate sanitization with:
  - generated BigQuery API commands
  - `jobs query`
  - analytics outputs where applicable
- surface when sanitization changed the output
- define Gemini extension manifest/assets
- add packaging helpers for Gemini registration
- generate extension registration metadata from the shipped command surface
- add smoke tests for:
  - one sanitization path
  - Gemini extension installation

Dependency note:

- Gemini extension registration depends on an external extension format/spec
- Phase 2 should target the current supported Gemini installation path, but the
  implementation should remain isolated enough that spec churn does not force a
  redesign of the dynamic command system

Done when:

- `--sanitize` is wired consistently through the supported command paths ✓
- Gemini extension manifest packaged and programmatically validated ✓
- `gemini extensions install` not yet tested live (spec evolving)

Actual outcomes:

- Model Armor integration verified end-to-end against live GCP project
- Regional endpoint requirement discovered and implemented during e2e testing
- `modelResponseData.text` request format corrected during e2e testing
- Prompt injection content correctly detected and redacted
- `--sanitize` + `--exit-code` interaction verified (CI gates preserved)

### Milestone 6: Docs, Validation, and Exit-Criteria Closure

Objective:
close the README Phase 2 exit criteria with reproducible docs and CI evidence.

Tasks:

- document discovery source behavior and fallback rules ✓
- document generated command limitations and allowlist scope ✓
- document `generate-skills` scope: ✓
  - what is generated
  - what remains curated
- add examples for: ✓
  - `datasets list`
  - `tables get`
  - `generate-skills`
  - `--sanitize`
- add CI checks for:
  - discovery snapshots ✓ (existing in CI)
  - generated command help ✓ (existing in CI)
  - generated skill snapshots ✓ (existing in CI)
  - extension manifest validation ✓ (gemini_tests.rs)
- align README examples with the shipped Phase 2 command surface ✓
- add reproducible e2e validation doc ✓ (docs/e2e-validation.md)

Done when:

- the README Phase 2 exit criteria can be demonstrated from CI artifacts or
  reproducible local commands ✓

## Recommended Build Order

### Stage 1

- Milestone 1: Discovery and Dynamic API Model

Reason:

everything else depends on Discovery being stable input and on the method
model existing before runtime parsing or execution.

### Stage 2

- Milestone 2: Generated Command Surface

Reason:

Phase 2 is not real until generated commands execute end to end from metadata.

### Stage 3

- Milestone 3: Skill Generation Pipeline
- Milestone 4: Curated Non-CA Skill Expansion

Reason:

generated service skills should be stable before the broader curated skill
surface is expanded around them.

### Stage 4

- Milestone 5: Sanitization and Integration Surfaces
- Milestone 6: Docs, Validation, and Exit-Criteria Closure

Reason:

these are late-stage integration tasks and should not drive the core command
or skill architecture.

## Detailed Task Breakdown

### Discovery

- choose bundled Discovery file location
- define cache invalidation semantics
- record the Discovery revision/version
- decide refresh behavior for CI and offline use

### Dynamic Command Generation

- normalize resource and method names to CLI-safe forms
- map Discovery params to required/optional CLI args
- decide how to expose nested resources and path params
- keep generated help human-readable
- define flattening rules for nested list/get responses rendered as `table`

### Execution

- build URL paths from Discovery path templates
- serialize request bodies only when required
- preserve auth and JSON output consistency
- avoid mutating methods until safety rules are explicit

### Skill Generation

- define deterministic SKILL templates
- define deterministic `openai.yaml` templates
- separate generated skills from curated skills on disk
- snapshot generated output

### Safety / Sanitization

- decide field-level redaction policy
- make sanitization explicit in output
- avoid surprising partial-mutation behavior

### Gemini Integration

- define the extension manifest shape
- define install path and packaging expectations
- add a smoke-testable install flow

## Open Decisions

### 1. Discovery Source of Truth

Decision needed:

- whether bundled Discovery JSON lives in source control or is generated as a
  build asset

Recommended default:

- keep a pinned bundled copy in source control
- refresh it intentionally with a script and review it like vendored API input

Why:

- deterministic builds and CI
- easy diff review when Google updates the Discovery doc

### 2. Static + Dynamic CLI Integration Strategy

Decision needed:

- whether to keep the current typed `Cli` parser and bolt on dynamic subcommands
  or migrate the entire CLI surface to explicit runtime `clap::Command`
  construction

Recommended default:

- keep static typed parsing for Phase 1 commands
- add dynamic fallback parsing for generated BigQuery API subcommands

Why:

- less risk to existing command behavior
- lets Phase 2 focus on the dynamic API surface

### 3. Phase 2 Generated Method Allowlist

Decision needed:

- whether generated commands start read-only or include mutation methods

Recommended default:

- read-focused allowlist only in Phase 2

Why:

- safer rollout
- less pressure to solve confirmations/guard rails immediately

### 4. Skill Generation Scope

Decision needed:

- whether `generate-skills` emits only service skills or also recipes/personas

Recommended default:

- generate service/API skills only
- keep recipes/personas curated by hand

Why:

- recipes and personas need opinionated workflow guidance that raw Discovery
  metadata cannot provide

### 5. `--sanitize` Semantics

Decision needed:

- whether sanitization applies to request input, response output, or both

Recommended default:

- response-focused first

Why:

- easier to reason about
- lower risk of silently changing API behavior

### 6. Gemini Extension Shape

Decision needed:

- whether Gemini registration should expose the entire dynamic surface or a
  curated subset

Recommended default:

- start with a curated Phase 2 subset

Why:

- smaller install surface
- easier help text and examples
- lower risk if the Gemini extension format changes underneath Phase 2

## Risks

### Risk 1: Discovery churn destabilizes the CLI

If command generation reflects upstream Discovery changes immediately, the CLI
surface will drift unpredictably.

Mitigation:

- bundled pinned Discovery by default
- explicit refresh path
- snapshots for generated help/skills

### Risk 2: Dynamic `clap` generation makes the CLI unreadable

If generated commands expose every API knob without curation, help text and
agent usage will degrade.

Mitigation:

- allowlist methods first
- normalize names carefully
- review generated help snapshots

### Risk 3: Skill generation becomes pseudo-AI content generation

If generated skills try to invent guidance from sparse API metadata, they will
be low quality and brittle.

Mitigation:

- use rigid templates
- keep recipes/personas curated

### Risk 4: `--sanitize` becomes a vague promise

If sanitization behavior is not explicit, users will not trust it.

Mitigation:

- document exact behavior
- make it opt-in
- emit explicit sanitization indicators

### Risk 5: Gemini extension work drags Phase 2 off course

Extension packaging can consume time without improving the command core.

Mitigation:

- do extension registration after Discovery, dynamic commands, and skills
- isolate Gemini-specific assets and manifest generation from the dynamic
  command engine
- if the external Gemini extension spec is unstable, treat registration as a
  thin packaging layer rather than a reason to change core Phase 2 architecture

## Definition of Done

Phase 2 is complete when all of the following are true:

- `dcx datasets list` works without any hardcoded command definition
- Discovery Document loading is deterministic by default
- `dcx generate-skills` emits valid `SKILL.md` files deterministically
- 19 non-CA curated skills exist and are internally consistent
- `--sanitize` works on the supported command paths
- Gemini extension registration succeeds
- generated command help and generated skills are snapshot-tested
- README Phase 2 examples match the shipped command surface

## Suggested First PRs

1. `feat(discovery): add bundled and cached Discovery Document loader`
2. `refactor(dynamic): add internal BigQuery API method model`
3. `feat(dynamic): generate clap tree for datasets/tables read commands`
4. `feat(dynamic): execute generated BigQuery API commands`
5. `feat(skills): add dcx generate-skills with deterministic templates`
6. `docs(skills): add curated non-CA Phase 2 skills`
7. `feat(sanitize): add --sanitize response redaction path`
8. `feat(gemini): add extension registration packaging`
9. `docs(phase2): align README and examples with generated command surface`
