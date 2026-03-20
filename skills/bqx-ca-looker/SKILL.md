---
name: bqx-ca-looker
description: Use Conversational Analytics with Looker data sources. Set up Looker explore profiles and ask natural language questions over Looker models.
---

## When to use this skill

Use when the user wants to:
- Ask natural language questions over Looker data
- Set up a CA profile for a Looker instance
- Understand how Looker CA differs from BigQuery CA
- Troubleshoot Looker CA configuration issues

## Prerequisites

Load the following skills: `bqx-ca`

See **bqx-shared** for authentication and global flags.

## How Looker CA works

Looker data sources use the same Chat/DataAgent API as BigQuery, but with
Looker-specific datasource references. The CLI abstracts this — you use
`bqx ca ask --profile <looker-profile>` just like BigQuery.

Key differences from BigQuery:
- Looker requires a **profile** (no ad-hoc `--tables` or `--agent` flags)
- Profiles reference Looker explores via `model/explore` format
- A maximum of **5 explores** can be included per profile
- Optional OAuth credentials (`looker_client_id` / `looker_client_secret`)

## Profile setup

Create a YAML profile for your Looker instance:

```yaml
# ~/.config/bqx/profiles/sales-looker.yaml
name: sales-looker
source_type: looker
project: my-gcp-project
looker_instance_url: https://mycompany.looker.com
looker_explores:
  - sales_model/orders
  - sales_model/customers
```

### With OAuth credentials

If your Looker instance requires API credentials:

```yaml
name: sales-looker-oauth
source_type: looker
project: my-gcp-project
looker_instance_url: https://mycompany.looker.com
looker_explores:
  - sales_model/orders
looker_client_id: YOUR_CLIENT_ID
looker_client_secret: YOUR_CLIENT_SECRET
```

Both `looker_client_id` and `looker_client_secret` must be provided together
or both omitted.

## Usage

```bash
# Ask a question using the Looker profile
bqx ca ask --profile sales-looker.yaml "What were the top 10 orders last month?"

# Text format for interactive exploration
bqx ca ask --profile sales-looker.yaml --format text "Revenue by region"

# JSON format for scripting
bqx ca ask --profile sales-looker.yaml --format json "Average order value" | jq '.sql'
```

## Profile resolution

The `--profile` flag resolves in this order:
1. Exact file path (absolute or relative)
2. `~/.config/bqx/profiles/<name>`
3. `deploy/ca/profiles/<name>`

## Explore format

Explores must use the `model/explore` format:
- `sales_model/orders` (valid)
- `analytics_model/page_views` (valid)
- `just_an_explore` (invalid — missing model)
- `model/explore/extra` (invalid — too many segments)

## Constraints

- Maximum 5 explores per profile
- Explore names must be `model/explore` format (exactly one `/`)
- `looker_instance_url` is required and must not be empty
- At least one explore is required
- OAuth credentials must be paired — cannot provide only one of client_id/secret
- `--profile` cannot be combined with `--agent` or `--tables`
- Data agent creation (`ca create-agent`) is supported for Looker sources
