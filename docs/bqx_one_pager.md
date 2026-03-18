# bqx: why BigQuery needs an agent-native CLI

**Author:** Haiyuan Cao

[Prototype](https://github.com/haiyuan-eng-google/bqx-cli/tree/main/src) — inspired by [GWS CLI](https://github.com/googleworkspace/cli)

## Summary

`bq` was built for humans typing database commands. `bqx` is built for a
world where agents and humans both use CLI as the control plane. That is a
different design target, and it needs a different tool.

`bq` keeps BigQuery usable. `bqx` makes BigQuery agent-usable. This is
the same thesis behind the GWS CLI effort: a strong CLI becomes a shared
control plane across humans, automation, and agents.

Recent signals from Perplexity and the industry trajectory toward coding
agents with VM execution environments confirm this direction. Local
orchestrators like OpenClaw demonstrate that the control plane is shifting.

### Why CLI instead of MCP?

MCP servers register every operation as a separate tool. Each tool
definition is sent to the LLM on every call — a per-turn tax that grows
with tool count. A CLI is equivalent to one tool: the agent already has
`bash`. Adding BigQuery via bqx costs zero additional tool definitions.

**Task:** `SELECT status, COUNT(*) FROM traces WHERE agent_id='support_bot' GROUP BY status`

| | BigQuery MCP server | MCP Toolbox for Databases | bqx CLI |
|---|---|---|---|
| **Tools registered** | 5 (list/get datasets, tables, execute\_sql) | 9 (+ search\_catalog, forecast, analyze\_contribution, ask\_data\_insights) | 0 extra (uses bash) |
| **Tool-def tokens per LLM call** | ~660 | ~1,880 | 0 |
| **Total tokens for one query** | ~1,570 | ~4,000 | ~460 |
| **Tool-def overhead in a 10-turn session** | ~13,200 | ~37,500 | 0 |

Token counts are calculated from the actual JSON tool schemas sent to the
LLM. MCP Toolbox pays the highest tax because all 9 tools — including
`forecast` and `analyze_contribution` with 5–6 parameters each — are
registered even when the agent only needs `execute_sql`. bqx avoids this
entirely: the agent calls `bqx jobs query --query "..." --format json`
through the bash tool it already has.

## Today's gaps in `bq` CLI

`bq` works fine for what it was designed to do. But it was designed in a
different era. The easiest way to see the gap is a concrete example. Say an
OpenClaw-style agent needs to answer: **"what is the error rate for
support_bot?"**

### Gap 1 — Skill support

Agents discover capabilities through skill files (SKILL.md). Without them,
an agent cannot know what a CLI can do or how to call it correctly.

| | `bq` CLI | `bqx` CLI |
|---|---|---|
| **Discovery** | No skill files. The agent must be pre-programmed with `bq` syntax, or parse `--help` text and guess. | Ships 26 skills in the open SKILL.md format. An agent reads the skill file and knows exactly what parameters to pass. |
| **Integration** | Every agent platform (OpenClaw, Gemini CLI, Claude Code) writes its own `bq` wrapper with hardcoded knowledge of which flags to use. | One stable skill surface. All agent platforms integrate BigQuery the same way — no per-platform wrapper code. |
| **Example** | Agent has no way to discover that `bq query` exists or what flags it needs. Team writes a custom tool definition for each agent framework. | Agent loads `skills/bqx-query/SKILL.md`, sees the command template, parameters, and output schema. Runs it directly. |

### Gap 2 — Formatting

Agents consume structured data. When output is human-readable text, every
agent has to write its own parser — and those parsers break whenever the
format changes.

| | `bq` CLI | `bqx` CLI |
|---|---|---|
| **Output** | ASCII tables and mixed text/status lines. No guaranteed schema. | Every command returns structured JSON with a predictable schema. |
| **Parsing** | Agent must regex-parse table borders, handle wrapped rows, strip status messages. Fragile across `bq` versions. | Agent calls `JSON.parse()` on stdout. Done. |
| **Example** | `bq query "SELECT ..."` returns a human-formatted table. Agent needs ~30 lines of parsing logic to extract rows, and breaks if column widths change. | `bqx jobs query --query "SELECT ..." --format json` returns `{"rows": [...], "schema": {...}}`. Zero parsing code. |

### Gap 3 — Extensibility

Agents need high-level workflows, not just raw API primitives. The right
architecture is: CLI commands map 1:1 to APIs, and Skills orchestrate those
commands into workflows. As new APIs land (e.g., agent ops), the CLI
surface grows automatically, and Skills define how agents use them.

| | `bq` CLI | `bqx` CLI |
|---|---|---|
| **Skills as orchestration** | No skill layer. Every agent team writes ad-hoc scripts to chain `bq` calls into workflows. No reuse across teams. | Skills (SKILL.md) orchestrate CLI commands into workflows. Example: the `bqx-analytics` skill tells an agent to call `bqx analytics doctor`, then `bqx analytics drift` if anomalies are found. New API → new CLI command → Skills compose it immediately. |
| **API → CLI → Skill pipeline** | Adding a workflow means writing a new script from scratch for each agent platform. | Once agent ops APIs land, bqx adds the corresponding CLI commands (one-line allowlist change via Discovery). Skills then define the orchestration — e.g., "run evaluate, if score drops below threshold run drift, then file a bug." No agent-side code changes. |
| **API coverage** | Fixed command set. New API methods require waiting for a `bq` release. | Dynamic commands generated from the BigQuery Discovery document. Adding a new API method is a one-line allowlist change — see example below. |

**Example — adding `datasets.delete` to bqx:**

bqx bundles Google's BigQuery Discovery document, which already describes
every API method: URL path, HTTP verb, parameters, and types. At startup,
bqx parses the document, builds clap subcommands for each allowlisted
method, and wires them to a generic HTTP executor. Today's allowlist covers
8 read-only methods. To add `datasets.delete`:

```rust
// src/bigquery/dynamic/model.rs — one line added
pub const ALLOWED_METHODS: &[&str] = &[
    "bigquery.datasets.list",
    "bigquery.datasets.get",
+   "bigquery.datasets.delete",   // ← this is the entire change
    "bigquery.tables.list",
    ...
];
```

No new handler, no new struct, no new tests for the command itself — the
Discovery document already defines the parameters (`projectId`,
`datasetId`, `deleteContents`) and bqx generates the CLI surface
automatically. `bqx datasets delete --dataset-id=foo` works immediately.

With `bq`, the agent invents the workflow. With `bqx`, the workflow is part
of the product.

## Prototype

This is not a proposal. I have already built and shipped a working prototype.

The prototype is at v0.3.0 with 347 tests, 26 agent skills, and release
binaries for 6 platforms (macOS, Linux, Windows — x64 and ARM64). It covers
three command domains:

- **Dynamic BigQuery API** — datasets, tables, routines, models, generated
  from the Discovery document
- **Agent Analytics** — doctor, evaluate, drift, insights, distribution,
  traces, HITL metrics, views. These prototype the workflow patterns that
  will migrate to Skills over agent ops APIs as those APIs land.
- **Conversational Analytics** — `ca ask`, `ca create-agent`,
  `ca add-verified-query`, `ca list-agents`

The CA integration currently targets BigQuery as the data source. I agree
with the feedback that a CA CLI tool should not be restricted to BigQuery —
it should support all CA agent data sources including Looker and external
databases. bqx is a natural place to prototype this broader CA CLI surface
and validate the interaction model before expanding to other sources.

It also ships with Model Armor integration, npm distribution, shell
completions, a Gemini extension manifest, and end-to-end validation against a
live GCP project.

## Ask

Tomas, I would like your sponsorship to pilot `bqx` with the OpenClaw team
as a real integration test — give agents a proper BigQuery CLI and measure
whether it actually reduces wrapper complexity and improves reliability.

If that pilot validates the direction, the next step is to position `bqx`
as BigQuery's official agent-native CLI.

I am prepared to lead this effort.
