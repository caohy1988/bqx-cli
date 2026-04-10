# Phase 6 Plan: Agent Contract Hardening and Bridge Layer (v0.5 to v0.6)

Status: implemented. All Phase 6 milestones (M1–M6) are merged on `main`.
The v0.6 release has not been cut yet; `Cargo.toml` is still at 0.5.0.

## Goal

Move `dcx` from a feature-complete Data Cloud CLI into a **high-reliability
agent execution surface**.

Phase 5 proved that `dcx` can cover the right product areas:

- Discovery-driven Data Cloud commands
- profile-aware Conversational Analytics
- analytics SDK command parity
- a consolidated skill layer

The next step is not "add more commands." The next step is to make the
existing command surface easier for agents to discover, compose, validate, and
execute safely across more runtimes.

Phase 6 should make `dcx`:

- more self-describing
- more machine-contract-driven
- safer in unattended execution
- easier to bridge into non-shell agent runtimes
- measurable with task-level agent evals

## Why this is the next step

Current `dcx` gaps are not mostly source coverage gaps anymore. They are
**agent contract** gaps:

- command definitions are spread across `clap`, skills, Gemini manifest, and
  docs rather than emitted from one machine-readable source of truth
- errors are JSON-wrapped but not yet typed into a stable machine contract
- dynamic API responses still leak raw service pagination and field shapes
- skills are thinner than before, but still duplicate command knowledge that
  should come from the CLI itself
- there is no agent-task eval harness measuring success, retries, or parse
  failures across real `dcx` tasks
- environments without `bash` still need a separate integration path

That is the Phase 6 opportunity: keep the command surface stable, but make the
contract around it much stronger.

## Research basis

This phase is based on current agent-tooling guidance:

- **OpenAI Structured Outputs / function calling**: tools work best when their
  contracts are strict and machine-readable rather than inferred from prose.
  `dcx` should expose explicit command schemas and stable output keys.
- **Anthropic Claude Code best practices**: repeated workflows should be
  elevated into explicit commands, slash commands, hooks, or MCP surfaces
  rather than rediscovered on every run.
- **Agent Skills best practices**: keep top-level skills short and routing-
  focused, move repeated logic into scripts, and return structured stdout.
- **MCP architecture**: tools, resources, and prompts are best built on top of
  one shared contract, not maintained as separate products.

Primary references:

- OpenAI Structured Outputs:
  <https://platform.openai.com/docs/guides/structured-outputs>
- OpenAI Function Calling:
  <https://platform.openai.com/docs/guides/function-calling>
- Anthropic Claude Code best practices:
  <https://www.anthropic.com/engineering/claude-code-best-practices>
- Agent Skills best practices:
  <https://agentskills.io/skill-creation/best-practices>
- Agent Skills scripting:
  <https://agentskills.io/skill-creation/using-scripts>
- Model Context Protocol:
  <https://modelcontextprotocol.io/>

## Phase 6 thesis

`dcx` should have **one shared command contract** that can power:

- the native CLI
- skills
- Gemini tool metadata
- future MCP exposure
- agent eval fixtures

That is the best leverage point. Once `dcx` can describe its own command
surface precisely, every integration gets simpler and less divergent.

## Scope

In scope:

- machine-readable command and output schemas for the existing `dcx` surface
- standardized typed error envelopes and pagination wrappers
- improved unattended / non-interactive execution semantics
- thinner skills generated from the shared command contract
- task-level agent eval harness for `dcx`
- optional bridge layer for non-shell runtimes

Out of scope:

- broad new product coverage
- mutation-heavy admin workflows as the main Phase 6 investment
- replacing skills with MCP or replacing CLI with MCP
- rewriting the Discovery pipeline

## Design principles

- **One source of truth**: command metadata should be emitted once and consumed
  by skills, manifests, tests, and bridge layers.
- **Strict machine contract**: stable output keys, typed errors, and explicit
  pagination beat prose.
- **CLI-first, not CLI-only**: `dcx` remains the best surface when the agent
  has shell access; bridge layers should exist for runtimes that do not.
- **Workflow knowledge belongs in code**: repeated agent steps should move into
  CLI helpers or scripts, not more markdown duplication.
- **Measure real agent success**: task success and retry rate matter more than
  raw command count.

## Proposed Milestones

### M1: Self-Describing Command Contract

Add a new metadata surface that emits the `dcx` command contract directly from
the CLI implementation.

Target commands:

```text
dcx meta commands
dcx meta describe analytics evaluate
dcx meta describe spanner databases get-ddl
dcx meta skill-map
```

Each contract object should include:

- contract version
- command path
- synopsis
- flag names, types, required/default semantics
- env var support
- output format support
- output key schema
- exit-code semantics
- examples
- related skills / references

Recommended output shape:

```json
{
  "contract_version": "1",
  "command": "dcx analytics evaluate",
  "flags": [
    {
      "name": "--evaluator",
      "type": "enum",
      "required": true,
      "values": ["latency", "error-rate", "turn-count", "token-efficiency", "ttft", "cost"]
    }
  ],
  "output": {
    "formats": ["json", "table", "text"],
    "json_schema_ref": "schemas/analytics/evaluate.json"
  },
  "exit_codes": {
    "0": "success",
    "1": "evaluation failure",
    "2": "infrastructure error"
  }
}
```

Why first:

- skills, manifests, docs, and bridge layers can all consume this
- it removes hand-maintained duplication
- it makes `dcx` much easier for agents to inspect before acting

Done when:

- `dcx meta commands` and `dcx meta describe ... --format json` exist
- core domains (`analytics`, `ca`, dynamic services, profiles, looker) emit
  machine-readable contracts
- the contract includes a stable `contract_version`
- contract evolution is additive-only within a major contract version
- contract fixtures are checked into the repo and regression-tested

### M2: Standard Machine Contract

Standardize runtime behavior so agents do not need per-command exceptions.

Target improvements:

- typed error envelope:

```json
{
  "error": {
    "code": "INVALID_IDENTIFIER",
    "message": "Invalid project-id: 'bad proj'",
    "hint": "Use alphanumeric characters, hyphens, or underscores.",
    "exit_code": 1,
    "retryable": false,
    "status": "error"
  }
}
```

- stable pagination wrapper for dynamic commands:

```json
{
  "items": [...],
  "next_page_token": "...",
  "source": "spanner"
}
```

- semantic exit-code contract outside analytics:
  - `0` = success
  - `1` = general / validation / command error
  - `2` = confirmation required
  - `3` = auth error
  - `4` = not found
  - `5` = conflict / already exists
- preserve analytics compatibility:
  - `dcx analytics` keeps SDK-aligned `0/1/2` semantics
  - the machine contract should declare per-command exit codes explicitly
- stdout/stderr discipline:
  - stdout is structured response data only
  - stderr is for warnings, progress, and human-readable notes
- standard support for:
  - `--limit`
  - `--page-token` / `--cursor`
  - `--page-all`
  - `--output-file`
  - `--format json|table|text|ndjson`
- default row limits for large result sets unless `--limit` is explicit
- explicit `warnings` array in JSON mode when flags are accepted but have no
  runtime effect yet

Done when:

- all JSON errors follow one typed envelope
- exit codes are explicit in both the process contract and JSON body
- dynamic command families normalize pagination into one wrapper
- large-result commands support sane default limits and page-all behavior
- NDJSON is available where streaming large outputs is useful
- machine-readable warnings are available in JSON output
- stdout is always machine-safe in JSON/NDJSON mode
- the docs stop needing per-command parsing caveats

### M3: Unattended Execution and Safety

Strengthen behavior for CI agents and long-running automation.

Target improvements:

- universal preflight validation before auth/network
- broader `--dry-run` / `--explain` support
- explicit confirmation contract for future mutations:
  - `--yes`
  - `--no-color`
  - non-interactive-safe stderr/stdout behavior
- structured confirmation envelope when `--yes` is absent:

```json
{
  "status": "confirmation_required",
  "changes": ["Would delete dataset 'staging'"],
  "confirm_command": "dcx datasets delete --project-id=X --dataset-id=staging --yes",
  "exit_code": 2
}
```

- idempotency contract for create/mutate commands:
  - `--if-not-exists` or equivalent where applicable
  - `--client-request-id` for retry-safe requests where supported
  - `5` / conflict semantics for already-exists conditions
- TTY detection:
  - suppress prompts, progress UI, and color automatically when not attached
    to a TTY
  - `--yes` remains the explicit confirmation override
- auth and profile preflight:

```text
dcx auth check
dcx profiles test --profile <name>
```

- `dcx auth check` should remain local-only and fast
- `dcx profiles test` should be a lightweight network validation of the target
  source
- timeout and retry metadata in machine-readable command contracts

Done when:

- every non-trivial command can be validated locally without side effects
- mutating commands expose one structured confirmation protocol
- idempotent mutation semantics are documented and testable
- CI-safe execution patterns are documented and tested
- non-TTY execution is quiet and deterministic by default
- auth/profile failures are distinguishable from command misuse

### M4: Contract-Driven Skills and Tool Metadata

Reduce the remaining duplication between the CLI and agent integrations.

Target work:

- generate skill command tables from `dcx meta describe`
- generate Gemini tool definitions from the same command contract
- move repeated recipes into scripts where appropriate
- keep router skill `SKILL.md` files short and routing-focused
- enforce agentskills.io constraints in generated skills:
  - trigger-condition descriptions
  - lowercase hyphenated names
  - max-length-safe names
  - top-level body stays within the spec token budget
  - progressive disclosure through references/resources

Result:

- skills become thinner
- manifests stay aligned automatically
- new commands become cheaper to expose to agents

Done when:

- at least router/API skills pull their command sections from generated data
- Gemini metadata no longer hand-copies flag semantics for supported commands
- generated skills comply with agentskills.io structural constraints
- skill drift is caught in CI

### M4b: Thin Skill Layer and Reference-First Generation

Keep the skill layer small, routing-focused, and resistant to re-growth.

Target work:

- keep the advertised skill set flat:
  - router skills remain the primary activation surface
  - recipe skills remain the only workflow-level additions
  - API detail moves into generated references instead of new top-level skills
- make every router `SKILL.md` a thin dispatcher:
  - when to use
  - when not to use
  - 3-6 decision rules
  - pointers to generated `references/commands.md` and related resources
- move exact command detail out of top-level skill bodies:
  - flag tables
  - constraints
  - examples
  - command matrices
- generate reference files from `CommandContract` so exact syntax stays in one
  place
- enforce a thin-skill budget in CI:
  - top-level `SKILL.md` line / token cap
  - no duplicated flag tables across router skills
  - trigger-condition wording starts with `Use when the user wants to...`
- replace repeated prerequisite boilerplate with one shared generated auth /
  globals reference
- add lightweight activation telemetry / eval hooks:
  - which skill was selected
  - whether generated references were loaded
  - wrong-skill activations
  - tasks completed without extra reference loads

Result:

- fewer tokens in the always-loaded skill layer
- less duplicated command prose
- lower pressure to add new skills as commands grow
- tighter separation between routing knowledge and exact CLI syntax

Done when:

- router skills are the only primary advertised domain skills
- API command details are emitted into generated references rather than copied
  into `SKILL.md`
- top-level router skills stay within a documented size budget
- CI fails if generated references drift from the command contract
- skill count stays flat unless activation data justifies a new top-level skill
- evals can distinguish router-only success from router-plus-reference fallback

### M5: Agent Eval Harness

Add a real measurement loop for agent performance on `dcx`.

Split this into two layers:

- **M5a: deterministic eval suite (CI-gated)** for command success, error
  recovery, exit codes, and JSON contracts without relying on an LLM
- **M5b: LLM benchmark suite (periodic)** was proposed as a future extension,
  but is not required for Phase 6 completion

Target eval tasks:

- fetch a dataset/table/schema
- debug a session trace
- run an evaluator gate
- inspect a Looker explore
- validate a profile and then query a source
- perform a dry-run on a generated API command

Metrics:

- task completion rate
- retries per task
- tool/CLI parse failures
- tool selection accuracy
- context durability over longer sessions
- prompt length needed
- command count per successful task
- wall-clock time to task completion

Recommended outputs:

- checked-in deterministic task suite for CI
- release gate on a small deterministic smoke subset
- optional future benchmark runs for:
  - Claude Code
  - Gemini CLI

Done when:

- `dcx` has task-level evals, not just unit/integration tests
- deterministic evals are separated cleanly from any future LLM benchmark work
- changes to skills/contracts can be judged by actual agent outcomes

### M6: Optional Bridge Layer for Non-Shell Runtimes

Once M1-M4 exist, expose the same command contract in an alternate delivery
mode for runtimes without `bash`.

Recommended form:

```text
dcx mcp serve
```

Design rule:

- same command contract
- same validation semantics
- same output schema
- same error semantics

Scope rule:

- do **not** rebuild Google-managed MCP servers for generic BigQuery,
  AlloyDB, Spanner, or Cloud SQL CRUD
- expose the differentiated `dcx` surface instead:
  - analytics commands
  - CA commands
  - cross-source profile routing
  - meta / contract introspection
- target a compact bridge surface of roughly 10-15 tools

This should be a bridge, not a second product.

Done when:

- a constrained agent runtime can access a useful subset of `dcx` without
  shell access
- bridge tools are generated from the same underlying contract as the CLI
- the bridge is clearly positioned as complementary to existing Google-managed
  servers, not a duplicate of them

## Recommended order

Build Phase 6 in this order:

1. self-describing command contract
2. standard machine contract
3. unattended execution / safety hardening
4. contract-driven skills and manifests
5. agent eval harness
6. optional bridge layer

This order matters. The bridge layer and skill generation should consume the
contract, not invent a second one.

## Implemented PR sequence

1. `feat(meta): add dcx meta commands and meta describe`
2. `test(contract): snapshot command contracts for analytics and dynamic APIs`
3. `feat(output): add typed error envelopes, retryable flag, and warnings array`
4. `feat(output): normalize pagination wrappers and add ndjson / page-all support`
5. `feat(safety): add confirmation envelope, tty-aware execution, and auth/profile preflight`
6. `docs(skills): generate skill command tables from meta describe`
7. `test(agent-evals): add deterministic task suite for dcx`
8. `feat(mcp): add dcx mcp serve`

## Success criteria

Phase 6 is complete when:

- `dcx` can describe its own command contract in machine-readable form
- errors, warnings, exit codes, and pagination are standardized across domains
- skills and tool manifests consume generated command metadata instead of
  copying it by hand
- `dcx` has deterministic agent evals gating command success, contract
  stability, and preflight behavior
- an optional non-shell bridge exists without creating a separate product

## Short version

Phase 5 made `dcx` broad enough.

Phase 6 should make it **legible and reliable enough for agents**.
