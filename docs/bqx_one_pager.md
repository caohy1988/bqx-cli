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

Agents need high-level workflows, not just raw API primitives. "Check agent
health" or "detect metric drift" should be single commands, not multi-step
scripts the agent has to invent each time.

| | `bq` CLI | `bqx` CLI |
|---|---|---|
| **Workflows** | Only exposes low-level CRUD. To answer "what is the error rate?", the agent must: find the table, write the SQL, run `bq query`, parse the text output. 4 fragile steps. | `bqx ca ask "What is the error rate for support_bot?" --agent=agent-analytics` — one command, structured JSON response with SQL, results, and explanation. |
| **Analytics** | No built-in analytics commands. Want drift detection? Write a SQL pipeline yourself. | `bqx analytics drift`, `bqx analytics evaluate`, `bqx analytics insights` — real operational workflows. An agent detects regression without writing any SQL. |
| **API coverage** | Fixed command set. New API methods require waiting for a `bq` release. | Dynamic commands generated from the BigQuery Discovery document. Adding a new API method to the CLI is a one-line allowlist change, not a new command implementation. |

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
  traces, HITL metrics, views
- **Conversational Analytics** — `ca ask`, `ca create-agent`,
  `ca add-verified-query`, `ca list-agents`

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
