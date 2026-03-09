# BQX: An Agent-Native BigQuery CLI

**Type:** PRD + RFC Proposal  
**Status:** Draft  
**Date:** March 9, 2026  
**Owner:** Haiyuan Cao

## 1. Summary

`bqx` is a proposed agent-native CLI for BigQuery. It is designed for two
users at the same time:

- human operators who need a fast, scriptable BigQuery tool
- AI agents that need structured, composable, low-overhead access to BigQuery

The core thesis is simple: BigQuery is already the default data plane for AI
agent analytics, but its current CLI experience is optimized for humans from
the pre-agent era. `bq` remains useful, but it is not designed for:

- JSON-first machine consumption
- progressive disclosure for model context efficiency
- agent analytics workflows such as evaluation and trace inspection
- discoverable skills and recipes
- natural composition with other CLI tools

`bqx` fixes that gap. It starts as a focused BigQuery and agent-analytics CLI,
then expands toward dynamic API coverage, reusable skills, and optional
Conversational Analytics integration.

## 2. Why Now

AI agents increasingly use CLIs as their default operating surface. Recent
community writing and benchmarks are directionally consistent on three points:

- CLI tools often impose far less context overhead than MCP tool schemas.
- Models already understand shell and common CLIs from training data.
- Unix-style composition is a natural fit for multi-step agent workflows.

Examples:

- Jannik Reinhard's February 22, 2026 write-up reports a side-by-side task
  where a CLI approach used far fewer tokens than an MCP approach, and cites a
  benchmark showing higher task completion and better token efficiency for CLI
  on identical tasks.
- Cobus Greyling argues that for many common agent tasks, CLI is the interface
  agents already understand: it exists, is self-documenting through `--help`,
  and is composable through pipes.
- A March 2026 Reddit discussion in `r/AI_Agents` captures the emerging
  consensus: CLI is often the best default for developer workflows, while MCP
  still matters for enterprise governance, strict permissions, and shared tool
  delivery.

This should not be read as "MCP is obsolete." It should be read as:

- use CLI by default when the system already has a strong command surface
- use MCP when governance, SaaS multi-tenancy, or non-CLI tools require it

BigQuery is a strong candidate for a CLI-first agent interface because:

- it is already a central system in agent analytics stacks
- developers and data engineers already work in terminals and scripts
- BigQuery workflows compose naturally with `jq`, `xargs`, `grep`, CI jobs, and
  other CLIs
- the existing `bq` experience does not expose agent-native patterns well

## 3. Problem Statement

Today, teams use BigQuery for:

- agent traces
- evaluation results
- drift and quality monitoring
- ad hoc operational analysis
- dashboards and scheduled reporting

But the tooling gap is clear.

`bq` was designed as a human-first administrative CLI. For agent use, it has
four weaknesses:

- mixed output conventions instead of a consistent JSON-first contract
- static human-oriented help and text output that waste context budget
- no first-class workflows for agent analytics
- no extensibility model for skills, recipes, or agent guidance

The result is predictable: teams either force agents through generic shell
usage of `bq`, or they build one-off wrappers, SDK scripts, or MCP servers.
That creates fragmentation where there should be one clear agent-native
interface.

## 4. Product Goal

Deliver a BigQuery CLI that is better than `bq` for agent workflows while
remaining useful for humans.

Success means a team can demo the following story with one binary:

1. run a BigQuery query with structured output
2. validate an agent analytics dataset
3. evaluate recent sessions against a threshold
4. inspect one failing session trace

## 5. Non-Goals for the Initial MVP

The first demoable release should explicitly avoid broad scope.

Not in MVP:

- full dynamic generation from the Discovery Document
- full BigQuery API coverage
- skills packaging and distribution
- Conversational Analytics integration
- interactive auth flows
- LLM-as-judge, drift, insights, views, distribution
- npm distribution
- plugin architecture

These are good roadmap items, but they will slow down the first working demo.

## 6. Proposed Solution

`bqx` will be built as a Rust CLI with three long-term domains:

- `bqx <resource> <method>` for BigQuery API access
- `bqx analytics <command>` for agent analytics workflows
- `bqx ca <command>` for Conversational Analytics

For the MVP, only a small static subset is required:

### MVP Commands

- `bqx jobs query --query ...`
- `bqx analytics doctor`
- `bqx analytics evaluate --evaluator=latency|error-rate --threshold ...`
- `bqx analytics get-trace --session-id ...`

### MVP User Experience Principles

- JSON-first by default
- optional `--format=table` for human inspection
- flags that map cleanly to BigQuery concepts
- ADC-based auth only
- deterministic SQL-backed analytics commands

### Why This Scope Is Enough

This is the minimum slice that proves the product thesis:

- `jobs query` proves `bqx` is still a real BigQuery CLI
- `doctor` proves operational readiness and dataset awareness
- `evaluate` proves agent-native workflows that `bq` does not model well
- `get-trace` proves the debugging loop

That is enough for a team demo and enough to decide whether the project should
expand.

## 7. Design Overview

### CLI Structure

Use static `clap` commands first. Do not implement runtime discovery in the
MVP.

```text
bqx
├── jobs
│   └── query
└── analytics
    ├── doctor
    ├── evaluate
    └── get-trace
```

### Data Contract

Assume one analytics dataset and one primary table for the MVP:

- dataset: provided by `--dataset-id` or `BQX_DATASET`
- table: `agent_events`

Required columns:

- `session_id`
- `agent`
- `event_type`
- `timestamp`

Expected optional columns:

- `status`
- `error_message`
- `content`
- `latency_ms`

### Auth

MVP uses Application Default Credentials only:

- existing local ADC
- `GOOGLE_APPLICATION_CREDENTIALS`
- `gcloud auth application-default login`

This avoids building an auth subsystem before the CLI proves its value.

### Output

- default: JSON
- optional: `table` for `evaluate` and `get-trace`

The output contract matters more than feature breadth. Agents need predictable
fields, not pretty text.

## 8. Detailed MVP Plan

### Milestone 1: Scaffold

Build the Rust project with:

- `clap`
- `tokio`
- `reqwest`
- `serde`
- `serde_json`
- `anyhow`

Done when:

- `bqx --help`
- `bqx jobs --help`
- `bqx analytics --help`

all work.

### Milestone 2: BigQuery Foundation

Implement:

- config parsing for project, dataset, location, format
- ADC token acquisition
- a thin BigQuery REST client

Done when:

- `bqx jobs query --query "SELECT 1"` succeeds against a real project

### Milestone 3: Analytics Readiness

Implement `bqx analytics doctor`.

Checks:

- dataset exists
- `agent_events` exists
- required columns exist
- recent row count is non-zero

Done when the command clearly returns `ok`, warnings, and missing schema
elements in JSON.

### Milestone 4: Session Evaluation

Implement `bqx analytics evaluate`.

Supported evaluators:

- `latency`
- `error-rate`

Behavior:

- query session aggregates from BigQuery
- compute pass/fail in Rust
- return summary plus failed sessions
- support `--exit-code`

Done when the command can fail a CI step and identify problematic sessions.

### Milestone 5: Trace Inspection

Implement `bqx analytics get-trace --session-id`.

Behavior:

- fetch ordered events for one session
- render JSON or table
- make it easy to pivot from `evaluate` to root-cause inspection

Done when one session from the evaluation output can be inspected live in the
demo.

### Milestone 6: Demo Packaging

Add:

- fixture queries
- a demo script
- sample expected outputs

Done when a 10-minute team demo can run end-to-end without improvisation.

## 9. SQL-First Implementation Strategy

Do not integrate the Python Agent Analytics SDK in the MVP.

Instead:

- implement analytics commands as SQL templates plus small Rust transforms
- keep logic visible and debuggable
- reduce moving parts

This is the right trade-off for a first demo because it optimizes for delivery
speed, determinism, and clarity.

Example analytics patterns:

- `doctor`: `INFORMATION_SCHEMA.COLUMNS` plus recent row counts
- `latency`: per-session aggregation over `--last`
- `error-rate`: derive session failure from `status`, `error_message`, or
  `_ERROR` event types
- `get-trace`: ordered event retrieval by `session_id`

## 10. Risks and Mitigations

### Risk 1: CLI Safety

CLI access is powerful. External commentary is correct to call out the tradeoff:
agents with shell access need boundaries.

Mitigation for `bqx`:

- keep the CLI narrow and purpose-built
- default to read-heavy commands in the MVP
- require explicit confirmation for destructive operations later
- use JSON output to reduce ambiguity

### Risk 2: Overbuilding Before Demo

The proposal can easily expand into discovery generation, skills, and CA before
the core value is proven.

Mitigation:

- gate all post-MVP work behind a successful internal demo
- ship only four commands first

### Risk 3: Confusing CLI vs MCP Positioning

If `bqx` is framed as anti-MCP, the architecture discussion will get derailed.

Mitigation:

- position `bqx` as the best default interface for BigQuery developer and
  agent workflows
- position MCP as complementary when strict governance or shared structured
  delivery is needed

## 11. Longer-Term RFC Direction

If the MVP succeeds, the next expansions are:

1. dynamic BigQuery API coverage from the Discovery Document, with a bundled
   pinned fallback for CI and offline reliability
2. curated `SKILL.md` files for agent discoverability
3. broader analytics commands such as drift, insights, and views
4. optional Conversational Analytics commands once the API is stable

This preserves the original `bqx` vision while keeping the first release
small enough to finish.

## 12. Decision

Proceed with a demo-first `bqx` MVP that proves:

- BigQuery queries can be exposed through an agent-native CLI surface
- agent analytics workflows deserve first-class commands
- JSON-first CLI patterns are a better default than ad hoc wrappers around `bq`

If the MVP demo lands well, expand toward the full proposal. If it does not,
the project still produces a useful internal tool with limited sunk cost.

## References

- README proposal in this repo: [README.md](/Users/haiyuancao/bqx-cli/README.md)
- Reddit discussion, "The Truth About MCP vs CLI":
  https://www.reddit.com/r/AI_Agents/comments/1rjtp3q/the_truth_about_mcp_vs_cli/
- Cobus Greyling, "Replace MCP With CLI, The Best AI Agent Interface Already Exists":
  https://cobusgreyling.substack.com/p/replace-mcp-with-cli-the-best-ai
- Jannik Reinhard, "Why CLI Tools Are Beating MCP for AI Agents" (February 22, 2026):
  https://jannikreinhard.com/2026/02/22/why-cli-tools-are-beating-mcp-for-ai-agents/

## Notes on Evidence

The external sources above are useful directional evidence, not formal product
validation. Their strongest value here is not the exact benchmark number; it is
the repeated pattern they describe:

- lower context overhead
- easier runtime discovery through `--help`
- better composability
- clearer fit for developer-facing agent workflows
