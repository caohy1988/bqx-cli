# bqx: why BigQuery needs an agent-native CLI

**Author:** Haiyuan Cao

## Summary

I built `bqx` because I kept hitting the same problem: every time an agent
needs to use BigQuery, someone has to write a custom wrapper around `bq` or
the REST API. The output is inconsistent, the auth story is painful in
ephemeral environments, and there is no CLI path to Conversational Analytics
at all.

`bq` was built for humans typing database commands. `bqx` is built for a
world where agents and humans both use CLI as the control plane. That is a
different design target, and it needs a different tool.

## What exists today

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

The engineering is done. The question is whether BigQuery wants to own this
as a product direction.

## Why this matters strategically

BigQuery is already where agent-era data ends up: warehouse data, telemetry,
traces, evaluations, structured application state. But the interface layer
has not kept up.

Right now, if an agent platform like OpenClaw wants to use BigQuery, every
team builds their own wrapper. That means BigQuery is the backend, but
someone else owns the developer experience.

A strong agent-native CLI changes that. BigQuery stops being "the database
behind custom glue code" and becomes "the data system agents already know
how to use." That is a meaningful difference for adoption.

If BigQuery does not provide this surface, agent platforms will still use
BigQuery — but through third-party tooling and brittle wrappers that we do
not control.

## The gap with `bq`

`bq` works fine for what it was designed to do. But it was designed in a
different era.

The easiest way to see the gap is a concrete example. Say an OpenClaw-style
agent needs to answer: "what is the error rate for support_bot?"

**Today with `bq`**, the agent has to:

1. figure out which table to query
2. generate the SQL itself
3. call `bq query` and handle its text output
4. parse the response back into something structured
5. deal with auth and formatting inconsistencies

That works, but it is fragile. Every step is a place where the integration
can break.

**With `bqx`**, it is one command:

```bash
bqx ca ask "What is the error rate for support_bot?" --agent=agent-analytics
```

The agent gets back structured JSON with the generated SQL, results, and
explanation. No wrapper code. No output parsing.

For raw SQL access, the story is similarly cleaner:

```bash
bqx jobs query --query "SELECT ..." --format json
```

With `bq`, the agent invents the workflow. With `bqx`, the workflow is part
of the product.

## How `bqx` delivers

Four things make `bqx` work for agents in practice:

**JSON-first output.** Every command returns predictable, structured JSON.
Agents do not have to parse human-readable text.

**Workflow commands, not just primitives.** `bqx analytics evaluate`,
`bqx analytics drift`, `bqx analytics insights` — these are real operational
workflows, not raw API calls. An agent can run a health check or detect
regression without generating any SQL.

**Conversational Analytics via CLI.** `bqx ca ask` turns BigQuery into a
system agents can question directly in natural language. That is a capability
`bq` simply does not have.

**Reusable skill surface.** bqx ships 26 skills in the open SKILL.md format.
OpenClaw, Gemini CLI, Claude Code, and similar systems can integrate BigQuery
through one stable surface instead of each building their own.

This is the same thesis behind the GWS CLI effort: a strong CLI becomes a
shared control plane across humans, automation, and agents.

`bq` keeps BigQuery usable. `bqx` makes BigQuery agent-usable.

## Ask

Tomas, I would like your sponsorship to pilot `bqx` with the OpenClaw team
as a real integration test — give agents a proper BigQuery CLI and measure
whether it actually reduces wrapper complexity and improves reliability.

If that pilot validates the direction, the next step is to position `bqx`
as BigQuery's official agent-native CLI, not just an interesting side
project.

I am prepared to lead this effort.
