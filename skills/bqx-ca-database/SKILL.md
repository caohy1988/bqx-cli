---
name: bqx-ca-database
description: Use Conversational Analytics with database sources (AlloyDB, Spanner, Cloud SQL). Set up database profiles and query databases using natural language.
---

## When to use this skill

Use when the user wants to:
- Ask natural language questions over AlloyDB, Spanner, or Cloud SQL data
- Set up a CA profile for a database source
- Understand how database CA differs from BigQuery/Looker CA
- Route to the right database-specific skill

## Prerequisites

Load the following skills: `bqx-ca`

See **bqx-shared** for authentication and global flags.

## How database CA works

Database sources use the **QueryData API**, which is different from the
Chat/DataAgent API used by BigQuery and Looker. The CLI handles this
routing automatically — you use `bqx ca ask --profile <db-profile>` and
the profile's `source_type` determines which API is called.

Key differences from BigQuery/Looker:
- Database sources use the QueryData API (not Chat/DataAgent)
- No data agent creation — `ca create-agent` is not supported for databases
- No visualization rendering for database sources
- Profiles require database-specific identifiers (instance, database, cluster)
- Optional `context_set_id` for pre-authored context

## Supported databases

| Source | `source_type` | Required fields |
|--------|--------------|-----------------|
| AlloyDB | `alloy_db` | `cluster_id`, `instance_id`, `database_id` |
| Spanner | `spanner` | `instance_id`, `database_id` |
| Cloud SQL | `cloud_sql` | `instance_id`, `database_id`, `db_type` |

All database profiles also require `project` and `location`.

## Database-specific skills

For detailed setup and troubleshooting per database type:
- **AlloyDB**: see `bqx-ca-alloydb`
- **Spanner**: see `bqx-ca-spanner`
- **Cloud SQL**: see below (setup is similar to AlloyDB)

## Cloud SQL profile setup

```yaml
name: app-cloudsql
source_type: cloud_sql
project: my-gcp-project
location: us-central1
instance_id: my-app-db
database_id: myapp
db_type: postgresql    # "mysql" or "postgresql"
```

### Cloud SQL prerequisites

1. Enable the Cloud SQL Data API:
   ```bash
   gcloud sql instances patch INSTANCE_ID --data-api-access=ALLOW_DATA_API
   ```
2. Enable IAM authentication:
   ```bash
   gcloud sql instances patch INSTANCE_ID --database-flags=cloudsql.iam_authentication=on
   ```
3. Create an IAM database user:
   ```bash
   gcloud sql users create USER@DOMAIN --instance=INSTANCE_ID --type=CLOUD_IAM_USER
   ```

## Usage

```bash
# Ask a question against a database profile
bqx ca ask --profile ops-alloydb.yaml "top error categories last 24h"

# Text format
bqx ca ask --profile finance-spanner.yaml --format text "total payments by region"

# JSON for scripting
bqx ca ask --profile app-cloudsql.yaml --format json "active users today" | jq '.results'
```

## Context sets (optional)

Database profiles can include a `context_set_id` for pre-authored context
that improves query accuracy:

```yaml
name: ops-alloydb
source_type: alloy_db
project: my-project
location: us-central1
context_set_id: my-context-set    # optional
cluster_id: ops-cluster
instance_id: primary
database_id: opsdb
```

When provided, the context set is sent as an `agentContextReference` in the
QueryData request. When omitted, the API queries the database directly without
pre-authored context.

## Response structure

Database QueryData responses include the same fields as BigQuery CA:
- `question` — the original question
- `sql` — the generated SQL query
- `results` — query result rows
- `explanation` — natural language explanation

## Decision rules

- Use `bqx-ca-alloydb` for AlloyDB-specific setup and troubleshooting
- Use `bqx-ca-spanner` for Spanner-specific query patterns
- Database sources cannot use `--agent` or `--tables` flags
- `--profile` is the only way to query database sources
- Use `--format text` for interactive exploration, `--format json` for scripts

## Constraints

- Data agent creation is not supported for database sources
- Visualization rendering is not supported for database sources
- `db_type` must be `mysql` or `postgresql` for Cloud SQL
- `context_set_id` must not be empty when provided (omit it instead)
- The CA API is preview — database source support may change
