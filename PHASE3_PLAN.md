# Phase 3 Plan: v0.2 to v0.3

## Goal

Move `bqx` from the current Phase 2 state to the Phase 3 target defined in
[README.md](/Users/haiyuancao/bqx-cli/README.md):

- `bqx ca ask`
- `bqx ca create-agent`
- `bqx ca add-verified-query`
- ship `deploy/ca/verified_queries.yaml`
- remaining CA-dependent skills:
  - `bqx-ca`
  - `bqx-ca-ask`
  - `bqx-ca-create-agent`
  - `persona-sre`
  - `recipe-ca-data-agent-setup`
  - `recipe-error-alerting`
  - `recipe-self-diagnostic-agent`
- remaining analytics commands:
  - `insights`
  - `drift`
  - `distribution`
  - `views`
  - `hitl-metrics`
  - `list-traces`
- shell completions for bash, zsh, and fish
- documentation and examples

Authority note:
this plan targets the README Phase 3 roadmap in
[README.md](/Users/haiyuancao/bqx-cli/README.md), not earlier MVP, Phase 1,
or Phase 2 checkpoints.

Phase 3 is not about broadening Discovery coverage again. It is about adding
the two remaining product layers that Phase 1-2 intentionally deferred:
Conversational Analytics and first-class higher-level analytics workflows.

Versioning note:
the roadmap labels use `v0.2` to `v0.3`, but the current crate version in
[Cargo.toml](/Users/haiyuancao/bqx-cli/Cargo.toml) is still `0.0.1`. This
plan treats the version bump and release-note cleanup as part of Milestone 6
rather than as a prerequisite for implementation.

README alignment notes:

- the README Phase 3 roadmap is authoritative for scope and exit criteria
- the README examples show a slightly broader CA surface than the roadmap
  bullets, including `bqx ca list-agents`
- this plan treats `list-agents` as a Phase 3 usability add-on if the CA API
  shape supports it cleanly, but not as a hard exit criterion
- the CA API is still preview-dependent, so Phase 3 must isolate CA failures
  from the stable Phase 1-2 command surface
- `generate-skills` remains a Phase 2 service-skill generator; Phase 3 should
  update its docs/tests only as needed for total skill-count and category
  accuracy, not turn it into a CA-skill generator

## Baseline (post-Phase 2)

Phase 2 established the reusable CLI platform:

- static command tree for:
  - `jobs query`
  - `analytics doctor`
  - `analytics evaluate`
  - `analytics get-trace`
  - `auth login|status|logout`
  - `generate-skills`
- dynamic BigQuery API commands generated from the bundled Discovery document:
  - `datasets list|get`
  - `tables list|get`
  - `routines list|get`
  - `models list|get`
- JSON-first output contract plus `table` and `text`
- shared auth, config, output, and BigQuery client layers under
  [src/](/Users/haiyuancao/bqx-cli/src)
- 19 non-CA skills shipped under [skills/](/Users/haiyuancao/bqx-cli/skills)
- Model Armor integration via `--sanitize`
- Gemini extension manifest packaged and validated
- release automation, npm distribution, and live Phase 2 e2e validation docs

What Phase 3 still needs:

- no `ca` command tree in [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- no CA client or request/response model under [src/](/Users/haiyuancao/bqx-cli/src)
- no checked-in `deploy/ca/verified_queries.yaml`
- no first-class analytics commands beyond `doctor`, `evaluate`, `get-trace`
- no CA-dependent skills
- no `persona-sre`
- no completion scripts
- no CA integration test harness or CA e2e validation docs

## Scope

In scope:

- hand-written CA command tree under `bqx ca`
- CA API client and request/response modeling
- stable JSON output contract for CA commands
- verified-query asset packaging
- first-class implementations of the remaining analytics commands
- the remaining 7 CA-dependent skills
- shell completions for bash, zsh, and fish
- documentation, examples, and validation for the full Phase 3 surface

Out of scope:

- broad expansion of dynamic Discovery coverage beyond the existing Phase 2 set
- replacing the dynamic BigQuery API system with a CA-driven system
- full write/mutation coverage for BigQuery Discovery resources
- speculative CA features not already implied by the README
- non-shell distribution channels such as Homebrew or apt
- redesigning Phase 1-2 auth, packaging, or skill formats

## Architecture Direction

Keep the existing split:

- dynamic resource-oriented BigQuery API commands stay in the Phase 2 system
- analytics and CA commands remain hand-written static commands
- CA should be added beside `analytics`, not folded into the Discovery layer

Target modules:

```text
src/
├── ca/
│   ├── client.rs
│   ├── models.rs
│   ├── executor.rs
│   └── verified_queries.rs
├── commands/
│   ├── analytics/
│   │   ├── doctor.rs
│   │   ├── evaluate.rs
│   │   ├── get_trace.rs
│   │   ├── insights.rs
│   │   ├── drift.rs
│   │   ├── distribution.rs
│   │   ├── views.rs
│   │   ├── hitl_metrics.rs
│   │   └── list_traces.rs
│   └── ca/
│       ├── ask.rs
│       ├── create_agent.rs
│       ├── add_verified_query.rs
│       └── list_agents.rs          # optional
├── completions.rs
├── cli.rs
├── output.rs
└── main.rs
completions/
├── bqx.bash
├── _bqx
└── bqx.fish
deploy/
└── ca/
    └── verified_queries.yaml
tests/
├── ca_tests.rs
├── analytics_phase3_tests.rs
├── completion_tests.rs
└── snapshots/
```

Recommended implementation rule:

- add new Phase 3 code without destabilizing the proven Phase 1-2 command
  paths
- CA loading and API wiring should be isolated so preview instability does not
  brick unrelated commands
- keep analytics implementation consistent with the current repo first:
  Phase 3 should extend [src/commands/analytics/](/Users/haiyuancao/bqx-cli/src/commands/analytics)
  directly, and only extract a separate `src/analytics/` domain layer later if
  the command logic becomes repetitive enough to justify a refactor

Directory note:

- `completions.rs` is implementation code that generates completion scripts
- `completions/` at the repo root is the checked-in output directory for the
  generated bash, zsh, and fish artifacts

## Core Design

### 1. Conversational Analytics Command Domain

Phase 3 introduces a third static command domain:

- `bqx ca ask`
- `bqx ca create-agent`
- `bqx ca add-verified-query`
- optional: `bqx ca list-agents`

Recommended command model:

- `ask` is the primary user-facing command and the Phase 3 anchor
- `create-agent` and `add-verified-query` are setup commands for the
  `agent-analytics` data agent workflow
- `list-agents` is useful for discoverability, but should remain optional
  until the primary CA flows are stable

Suggested internal types:

- `CaQuestionRequest`
- `CaQuestionResponse`
- `CaAgentSpec`
- `CaAgentSummary`
- `VerifiedQuerySpec`

Design constraints:

- JSON remains the default output format
- `text` should summarize the question, generated SQL, and top results
- `table` should render result rows when the CA response includes tabular data
- the CA client must preserve the generated SQL in output when the API returns
  it; that is part of the README exit criterion value proposition
- the existing global `--sanitize` flag should apply to CA responses too, so
  CA commands do not become the one unsanitized exception in the CLI

### 2. CA Output Contract

`bqx ca ask` needs a stable contract that agents can reason about.

Recommended JSON shape:

```json
{
  "question": "error rate for support_bot?",
  "agent": "agent-analytics",
  "sql": "SELECT ...",
  "results": [],
  "explanation": "..."
}
```

Recommended error behavior:

- preserve the existing CLI-wide `{"error":"..."}` envelope on failure
- distinguish validation errors from remote CA errors before any network call
- if the CA response is partial, surface the partial data rather than dropping
  fields silently

### 3. Verified Query Asset

The README makes the prebuilt `agent-analytics` data agent a core Phase 3
story. That requires a checked-in verified-query asset.

Required file:

- [deploy/ca/verified_queries.yaml](/Users/haiyuancao/bqx-cli/deploy/ca/verified_queries.yaml)

Recommended YAML contract:

- top-level `verified_queries`
- each entry includes:
  - `question`
  - `query`
  - optional `description`
  - optional named parameters metadata if the CA API needs it

Initial bundled query set should cover the README examples:

- error rate by agent
- p95 latency by agent
- top failing tools
- highest-latency sessions

Recommended implementation rule:

- parse and validate the YAML locally before attempting CA agent setup
- keep the asset deterministic and reviewable in git

### 4. Analytics Command Expansion

Phase 3 adds the remaining analytics commands as hand-written flows over the
existing `agent_events` model rather than over-generalizing the CLI.

Recommended command surface:

- `bqx analytics list-traces`
- `bqx analytics insights`
- `bqx analytics drift`
- `bqx analytics distribution`
- `bqx analytics hitl-metrics`
- `bqx analytics views`

Recommended implementation strategy:

- keep these commands SQL-first, similar to the Phase 1 analytics commands
- prefer pure SQL-builder helpers plus domain mappers, like the later Phase 1
  testability refactor
- only add subcommands where they are already implied by the README examples

Specific guidance:

- `list-traces`
  - list recent session summaries with filters such as `--last` and
    `--agent-id`
  - should compose naturally with `get-trace`
- `insights`
  - aggregate report over recent sessions: pass/fail rates, top errors, slow
    tools, trace counts
  - output should work especially well in `json` and `text`
- `drift`
  - formalize the current recipe/helper workflow into a first-class command
  - align with the README example using `--golden-dataset` and `--last`
- `distribution`
  - summarize question or event distribution patterns over time windows
  - keep the first version read-only and summary-oriented
- `hitl-metrics`
  - report human-review or escalation metrics only if those events exist in the
    current data model; otherwise return a clear no-data result rather than
    guessing
- `views`
  - this likely needs subcommands, with `create-all` as the first required one
    because the README already documents `bqx analytics views create-all --prefix=adk_`

### 5. Skill Completion

Phase 2 shipped 19 non-CA skills. Phase 3 closes the full 26-skill set.

Remaining skills to add:

- `bqx-ca`
- `bqx-ca-ask`
- `bqx-ca-create-agent`
- `persona-sre`
- `recipe-ca-data-agent-setup`
- `recipe-error-alerting`
- `recipe-self-diagnostic-agent`

Recommended rule:

- these remain curated skills, not generated skills
- they should point only to commands that actually ship in Phase 3
- `persona-sre` should move from recipe-level approximation to real command
  guidance now that `bqx ca ask` exists

### 6. Completion Scripts

Phase 3 should add shell completions as a packaging/output task, not as a new
architecture.

Recommended approach:

- use `clap_complete`
- generate bash, zsh, and fish completion scripts from the real CLI tree
- include dynamic command groups and new static Phase 3 commands

Recommended outputs:

- checked-in completion artifacts under `completions/`
- install instructions in the README

### 7. CA Stability Isolation

The README explicitly notes that the CA API is preview-dependent. Phase 3 must
therefore isolate CA risk.

Recommended safeguards:

- CA commands build and fail independently from the rest of the CLI
- CA API client is injectable for tests
- no CA network calls happen before local validation of required flags/files
- if CA API setup is unavailable, the existing Phase 1-2 commands still work

## Milestones

### Milestone 1: CA Client and `bqx ca ask`

Add the minimal CA client and the single most important user-facing command.

Tasks:

- add `Ca` to [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- implement `bqx ca ask <question>` with optional:
  - `--agent`
  - `--tables`
  - `--instructions` only if the API requires it
- add CA response models under [src/ca/](/Users/haiyuancao/bqx-cli/src/ca)
- add `json`, `table`, and `text` renderers for `ca ask`
- wire `--sanitize` through `ca ask` using the existing Phase 2 Model Armor
  path
- add preview-safe CA client injection seam for tests
- add unit and mocked integration tests for:
  - argument validation
  - request shaping
  - response mapping
  - output rendering

Done when:

- `bqx ca ask "error rate for support_bot?" --agent=agent-analytics`
  returns structured output in tests and local mocked runs
- CA failures do not affect unrelated command paths

### Milestone 2: Agent Setup Flows and Verified Queries

Make the `agent-analytics` setup flow concrete and reproducible.

Tasks:

- add `bqx ca create-agent`
- add `bqx ca add-verified-query`
- optional: add `bqx ca list-agents`
- add [deploy/ca/verified_queries.yaml](/Users/haiyuancao/bqx-cli/deploy/ca/verified_queries.yaml)
- add local YAML validation and loading helpers
- make `create-agent` accept:
  - `--name`
  - `--tables`
  - `--views`
  - `--verified-queries`
  - `--instructions`
- snapshot the bundled verified queries and agent-creation payloads

Done when:

- the repo contains a valid checked-in verified-query bundle
- `create-agent` and `add-verified-query` can be exercised against mocks
- the README example command shapes are supported by the CLI

### Milestone 3: Analytics Command Expansion I

Ship the lower-risk analytics commands that compose directly with the existing
Phase 1 workflows.

Tasks:

- add `bqx analytics list-traces`
- add `bqx analytics views create-all`
- add any supporting SQL builders and mappers
- define output contracts for:
  - trace listings
  - created/existing views
- add command tests and snapshots for all formats

Done when:

- users can list traces without dropping to raw SQL
- users can create event-type views through a first-class command path

### Milestone 4: Analytics Command Expansion II

Ship the summary/reporting analytics commands.

Tasks:

- add `bqx analytics insights`
- add `bqx analytics drift`
- add `bqx analytics distribution`
- add `bqx analytics hitl-metrics`
- align flags with README examples before adding extras
- add fixture-backed tests and snapshots for all commands

Recommended implementation order:

1. `insights`
2. `drift`
3. `distribution`
4. `hitl-metrics`

Done when:

- the analytics command set in the README is implemented
- all new analytics commands have deterministic `json`, `table`, and `text`
  outputs where appropriate

### Milestone 5: Remaining Skills

Close the 26-skill target with the CA-dependent set.

Tasks:

- add the 7 remaining curated skills
- update `generate-skills` documentation/tests only where Phase 3 changes total
  skill counts, category descriptions, or cross-links
- update skill cross-links so CA personas/recipes reference the real command
  surface
- ensure examples align with shipped Phase 3 commands
- validate `agents/openai.yaml` metadata against the existing Phase 2 schema

Done when:

- the repo contains all 26 README-listed skills
- `persona-sre` and the CA recipes no longer rely on hypothetical commands

### Milestone 6: Completions, Docs, and Exit-Criteria Closure

Close the remaining productization work after the command surface is stable.

Tasks:

- generate bash, zsh, and fish completions
- add completion install docs
- refresh [extensions/gemini/manifest.json](/Users/haiyuancao/bqx-cli/extensions/gemini/manifest.json)
  so the Phase 3 command surface is represented after the new commands settle
- update [README.md](/Users/haiyuancao/bqx-cli/README.md) to mark Phase 3
  complete only after validation is real
- add `docs/ca-e2e-validation.md` or expand
  [docs/e2e-validation.md](/Users/haiyuancao/bqx-cli/docs/e2e-validation.md)
  with CA coverage
- bump the crate/package version from `0.0.1` to the intended release version
  and align release notes/docs
- add reproducible validation commands for:
  - `ca ask`
  - `create-agent`
  - `add-verified-query`
  - all remaining analytics commands

Done when:

- `bqx ca ask "error rate for support_bot?"` returns SQL and results in live
  validation
- all analytics commands pass integration tests
- completion scripts are generated and documented

## Recommended Build Order

Build Phase 3 in this order:

1. CA client and `ca ask`
2. verified-query and agent-setup flows
3. low-risk analytics commands (`list-traces`, `views`)
4. report-style analytics commands (`insights`, `drift`, `distribution`,
   `hitl-metrics`)
5. CA-dependent skills
6. completions, docs, and live validation

Reasoning:

- `ca ask` is the Phase 3 anchor and highest-uncertainty dependency
- verified-query setup is required to make the README’s CA story real
- analytics commands should stabilize before skills and completions lock in
- docs and live validation should close the phase, not lead it

## Testing Strategy

Phase 3 should extend the Phase 1-2 testing model rather than invent a new one.

Required test layers:

- unit tests:
  - CA request builders
  - YAML verified-query parsing
  - analytics SQL builders and result mappers
  - completion generation smoke checks
- snapshot tests:
  - `ca ask` JSON/text/table output
  - verified-query bundle rendering or validation summaries
  - new analytics command output
- mocked integration tests:
  - CA client request/response handling
  - analytics commands through the shared executor seams
- live validation:
  - pre-release CA validation against a dedicated project if the preview API is
    available
  - live validation of the remaining analytics commands

Recommended test rule:

- no Phase 3 CI path should require live CA availability
- live CA validation should be documented and runnable, but optional in CI
  until the preview API stabilizes further

## Open Decisions

### 1. CA API Scope

Question:
should Phase 3 ship only the roadmap CA commands, or also `list-agents`
because the README examples already show it?

Recommendation:
ship `ask`, `create-agent`, and `add-verified-query` as required; add
`list-agents` only if it falls out naturally from the same client surface.

### 2. `ca ask` Context Flags

Question:
should `ca ask` prefer `--agent`, `--tables`, or both?

Recommendation:
support both.

- `--agent` is the primary happy path for the prebuilt `agent-analytics` flow
- `--tables` is useful for ad hoc use and appears in the README examples
- keep validation strict so users cannot accidentally provide conflicting
  context without a clear error

### 3. Analytics Command Depth

Question:
how feature-rich should the new analytics commands be in v0.3?

Recommendation:
ship the smallest version that matches the README examples and produces stable
output contracts. Avoid adding optional knobs that are not yet justified by
documented workflows.

### 4. `views` Command Shape

Question:
should `views` be a single command or a subcommand family?

Recommendation:
make it a subcommand family now.

- `views create-all` is already documented in the README
- a subcommand family leaves room for `list`, `drop`, or `refresh` later
  without breaking the surface

### 5. CA Skill Packaging

Question:
should the CA skills ship only after live CA validation, or can they land once
the commands are mocked and documented?

Recommendation:
land them after the command surface is implemented and tested, but before final
live validation. The final docs should still make clear which parts were
live-verified.

### 6. Live Validation Policy

Question:
how hard should Phase 3 depend on live CA e2e validation if the preview API is
unstable?

Recommendation:
keep the README exit criterion as the target, but document a fallback:

- required: mocked CA integration tests and reproducible live validation steps
- preferred: one successful live validation run before Phase 3 closeout
- if preview instability blocks CI, do not make CI depend on live CA

## Risks

### 1. CA Preview Instability

Risk:
the CA API may change shape or behavior during implementation.

Mitigation:

- isolate CA client code
- keep request/response models narrow
- prefer mocked integration tests over live CI coupling

### 2. Overdesigning Analytics Commands

Risk:
the remaining analytics commands could sprawl into mini-products.

Mitigation:

- anchor each command to a documented README workflow
- keep SQL builders explicit and testable
- postpone extra flags and subcommands unless already justified

### 3. Verified Query Drift

Risk:
the checked-in verified queries can diverge from the actual `agent_events`
schema or from CA expectations.

Mitigation:

- validate the YAML locally
- keep the initial bundle small and tied to README examples
- cover it with snapshot tests

### 4. Skill/Command Drift

Risk:
the 7 remaining skills can easily outpace the actual CLI surface.

Mitigation:

- write the commands first
- write the skills second
- require examples to use only existing flags and subcommands

### 5. Completion Drift

Risk:
checked-in completion scripts can become stale as the CLI evolves.

Mitigation:

- generate them from the real `clap` tree
- add a simple regeneration/check step in tests or CI

## Definition of Done

Phase 3 is done when all of the following are true:

- `bqx ca ask` is implemented and returns structured output with SQL and
  results
- `bqx ca create-agent` and `bqx ca add-verified-query` are implemented
- the repo ships [deploy/ca/verified_queries.yaml](/Users/haiyuancao/bqx-cli/deploy/ca/verified_queries.yaml)
- `insights`, `drift`, `distribution`, `views`, `hitl-metrics`, and
  `list-traces` are implemented
- the repo contains the remaining 7 CA-dependent skills, bringing the total to
  26
- bash, zsh, and fish completion scripts are generated
- the Gemini extension manifest is refreshed for the Phase 3 command surface
- the new command paths are covered by mocked integration tests and snapshots
- the README Phase 3 exit criterion is demonstrated with live validation or a
  clearly documented preview limitation

## Suggested First PRs

Recommended first PR sequence:

1. `feat(ca): add CA client and bqx ca ask`
2. `feat(ca): add create-agent, verified-query asset, and add-verified-query`
3. `feat(analytics): add list-traces and views create-all`
4. `feat(analytics): add insights, drift, distribution, and hitl-metrics`
5. `skills(ca): add remaining Phase 3 curated skills`
6. `docs(cli): add shell completions and Phase 3 validation docs`

This keeps the highest-risk dependency, CA, isolated first and delays doc and
skill lock-in until the command surface is stable.
