# dcx : why data cloud needs an agent-native CLI

 [Dqx cli prototype](https://github.com/haiyuan-eng-google/bqx-cli/tree/main/src)  inspired by [GWS cli](https://github.com/googleworkspace/cli), [CLI-Anything](https://github.com/HKUDS/CLI-Anything)

## Summary

`bq` was built for humans typing database commands. `dcx` is built for a world where agents and humans both use CLI as the control plane. That is a different design target, and it needs a different tool. `bq` keeps BigQuery usable. `dcx` makes data cloud agent-usable. This is the same thesis behind the GWS CLI effort: a strong CLI becomes a shared control plane across humans, automation, and agents.

Recent signals from Perplexity and the industry trajectory toward coding agents with VM execution environments confirm this direction. Local orchestrators like `Openclaw` demonstrate that the control plane is shifting. This is the same thesis behind Workspace open-sourcing an AI-focused CLI paired with agent skills: [GWS CLI](https://github.com/googleworkspace/cli)

### When CLI vs MCP

Both have real use cases.

MCP is a good fit when the agent does not have `bash`, when tool use must
go through a tightly controlled function boundary, or when the host
environment only allows API-style tools.

CLI is a better fit when the agent has a shell or VM and needs to do
iterative work across many operations. MCP servers register every
operation as a separate tool. Each tool definition is sent to the LLM on
every call — a per-turn tax that grows with tool count. A CLI is
equivalent to one tool: the agent already has `bash`. Adding Data Cloud
via `dcx` costs zero additional tool definitions.

**Task:** `SELECT status, COUNT(*) FROM traces WHERE agent_id='support_bot' GROUP BY status`

| | BigQuery MCP server | MCP Toolbox for Databases | dcx CLI |
|---|---|---|---|
| **Tools registered** | 5 (list/get datasets, tables, execute\_sql) | 9 (+ search\_catalog, forecast, analyze\_contribution, ask\_data\_insights) | 0 extra Data Cloud tools (uses bash) |
| **Tool-def tokens per LLM call** | ~660 | ~1,880 | ~0 additional Data Cloud tool-def tokens |
| **Total tokens for one query** | ~1,570 | ~4,000 | ~460 |
| **Tool-def overhead in a 10-turn session** | ~13,200 | ~37,500 | 0 |

Token counts are calculated from the actual JSON tool schemas sent to the
LLM. MCP Toolbox pays the highest tax because all 9 tools — including
`forecast` and `analyze_contribution` with 5–6 parameters each — are
registered even when the agent only needs `execute_sql`. `dcx` does not
add a separate per-command Data Cloud tool catalog: the agent calls
`dcx jobs query --query "..." --format json` through the bash tool it
already has. The shell surface itself is not literally free, but adding
`dcx` does not introduce the same per-turn tool-definition overhead as
MCP.

The right long-term model is not CLI-only or MCP-only. It is one shared
contract with two delivery modes: a CLI-first surface for agents that can
use `bash`, and API-oriented adapters for environments where shell access
is not available.

### Do agents need their own CLI?

Not necessarily a separate binary — but they do need a different **interface contract**. Humans and agents use the same commands, but they disagree on three things:

|  | What humans want | What agents want |
| :---- | :---- | :---- |
| **Output** | Readable ASCII tables | Structured JSON with a predictable schema |
| **Discovery** | `--help` text and man pages | Machine-readable skill files (SKILL.md) that declare when to use each command, what flags to pass, and what the output shape is |
| **Errors** | A descriptive message they can read | A JSON object with an error code they can branch on |

You could add `--format json` and skill files to `bq` — the interface requirements are not tied to a binary name. What matters is that **someone ships the agent-native interface layer**: JSON-first output, skill files for discovery, structured errors. dcx  is a working proof of what that layer looks like. Whether it ships standalone or as an agent mode on top of `bq` is an implementation decision that comes after validating the design.

## Today’s gaps in `bq cli` and the proposal

`bq cli` works fine for what it was designed to do. But it was designed in a different era. In comparison, the proposed dcx  is an agent-friendly, **open sourced** offering for agent use. The major differences are:

### Gap 1 — Skill support

Agents discover capabilities through skill files (SKILL.md). A skill file is a structured markdown document that tells an agent **when** to use a command, **how** to call it (flags, parameters, examples), and **what** the output looks like — all in a format every major agent platform already reads. Without skill files, an agent cannot know what a CLI can do or how to call it correctly.

|  | `bq` CLI | `dcx`  CLI |
| :---- | :---- | :---- |
| **Discovery** | No skill files. The agent must be pre-programmed with `bq` syntax, or parse `--help` text and guess. | Ships 26 skills in the open SKILL.md format. An agent reads the skill file and knows exactly what parameters to pass. |
| **Integration** | Every agent platform (OpenClaw, Gemini CLI, Claude Code) writes its own `bq` wrapper with hardcoded knowledge of which flags to use. | One stable skill surface. All agent platforms integrate data cloud the same way — no per-platform wrapper code. |
| **Example** | Agent has no way to discover that `bq query` exists or what flags it needs. Team writes a custom tool definition for each agent framework. | Agent loads `skills/dcx -query/SKILL.md`, sees the command template, parameters, and output schema. Runs it directly. |

**What a skill file looks like** — `skills/dcx -query/SKILL.md` (abridged):

```
---
name: dcx -query
description: Run raw data cloud SQL queries via dcx  CLI.
---
## When to use this skill
- "run this SQL through dcx "
- "dry-run this data cloud query"

## Core workflow
  dcx  jobs query --query "<SQL>" [--dry-run] [--format json|json-minified|table|text]

## Flags
| Flag         | Description                    |
|--------------|--------------------------------|
| --query      | SQL query string (required)    |
| --dry-run    | Show request without executing |
| --format     | json (default), table, or text |

## Output
JSON: {"total_rows": N, "rows": [...]}  — each row as key-value object.
```

An agent reads this file and immediately knows: what the command does, which flags to pass, what JSON shape to expect back. Compare this to `bq`, where every team reverse-engineers the same information from `--help` text and builds a bespoke wrapper. Skills like `dcx -analytics` go further — they include **routing tables** that tell the agent which subcommand to pick based on the user's goal (health check → `doctor`, threshold gate → `evaluate`, debug a session → `get-trace`), turning multi-step triage into a guided workflow.

### Gap 2 — Formatting

Agents consume structured data. When output is human-readable text, every agent has to write its own parser — and those parsers break whenever the format changes.

|  | `bq` CLI | `dcx`  CLI |
| :---- | :---- | :---- |
| **Output** | ASCII tables and mixed text/status lines. No guaranteed schema. | Every command returns structured JSON with a predictable schema. |
| **Parsing** | Agent must regex-parse table borders, handle wrapped rows, strip status messages. Fragile across `bq` versions. | Agent can parse stdout as JSON, pipe it to `jq`, redirect it to a file, or write code directly against the output. |
| **Example** | `bq query "SELECT ..."` returns a human-formatted table. Agent needs ~30 lines of parsing logic to extract rows, and breaks if column widths change. | `dcx jobs query --query "SELECT ..." --format json` returns `{"rows": [...], "schema": {...}}`. The agent can parse it directly or compose follow-on shell/code steps in a standard way. |

### Gap 3 — Extensibility

Agents need high-level workflows, not just raw API primitives. The right architecture is: CLI commands map 1:1 to APIs, and Skills orchestrate those commands into workflows. As new APIs land (e.g., agent ops), the CLI surface grows automatically, and Skills define how agents use them.

|  | `bq` CLI | `dcx`  CLI |
| :---- | :---- | :---- |
| **Skills as orchestration** | No skill layer. Every agent team writes ad-hoc scripts to chain `bq` calls into workflows. No reuse across teams. | Skills (SKILL.md) orchestrate CLI commands into workflows. Example: the `dcx -analytics` skill tells an agent to call `dcx  analytics doctor`, then `dcx  analytics drift` if anomalies are found. New API → new CLI command → Skills compose it immediately. |
| **API → CLI → Skill pipeline** | Adding a workflow means writing a new script from scratch for each agent platform. | Once agent ops APIs land, dcx  adds the corresponding CLI commands (one-line allowlist change via Discovery). Skills then define the orchestration — e.g., "run evaluate, if score drops below threshold run drift, then file a bug." No agent-side code changes. |
| **API coverage** | Fixed command set. New API methods require waiting for a `bq` release. | Dynamic commands generated from the data cloud Discovery document. Adding a new API method is a one-line allowlist change — see example below. |

**Example — adding `datasets.delete` to dcx :**

dcx  bundles Google's data cloud Discovery document, which already describes every API method: URL path, HTTP verb, parameters, and types. At startup, dcx  parses the document, builds clap subcommands for each allowlisted method, and wires them to a generic HTTP executor. Today's allowlist covers 8 read-only methods. To add `datasets.delete`:

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

No new handler, no new struct, no new tests for the command itself — the Discovery document already defines the parameters (`projectId`, `datasetId`, `deleteContents`) and dcx  generates the CLI surface automatically. `dcx  datasets delete --project-id=my-proj --dataset-id=foo` works immediately.

With `bq`, the agent invents the workflow. With `dcx` , the workflow is part of the product.

## Prototype

This is not a proposal. I have already built and shipped a working prototype.

The prototype is at v0.5.0 with 513 tests, 14 consolidated agent skills, and release binaries for 6 platforms (macOS, Linux, Windows — x64 and ARM64). It covers five command domains:

- **Dynamic Data Cloud API** — BigQuery (datasets, tables, routines, models), Spanner, AlloyDB, Cloud SQL, Looker — all generated from bundled Discovery documents
- **Agent Analytics** — 12 commands aligned with the upstream BigQuery Agent Analytics SDK: doctor, evaluate (6 evaluators), get-trace, list-traces, insights, drift, distribution, hitl-metrics, views, categorical-eval, categorical-views
- **Conversational Analytics** — `ca ask` (6 data sources: BigQuery, Looker, Looker Studio, AlloyDB, Spanner, Cloud SQL), `ca create-agent`, `ca add-verified-query`, `ca list-agents`
- **Looker Content** — explores and dashboards via per-instance Looker API
- **Profile Utilities** — list, show, validate source profiles

The CLI now includes full SDK alignment with automated drift detection: a CI contract-check job ensures the SDK contract stays current, and a weekly sync workflow opens PRs when the upstream SDK changes.
