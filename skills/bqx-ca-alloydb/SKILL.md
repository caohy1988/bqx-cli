---
name: bqx-ca-alloydb
description: AlloyDB-specific Conversational Analytics setup, profile configuration, and operational troubleshooting patterns using bqx ca ask.
---

## When to use this skill

Use when the user wants to:
- Set up CA for an AlloyDB database
- Troubleshoot AlloyDB CA connectivity issues
- Run operational queries against AlloyDB via natural language
- Understand AlloyDB-specific prerequisites

## Prerequisites

Load the following skills: `bqx-ca-database`

See **bqx-shared** for authentication and global flags.

## AlloyDB profile

```yaml
name: ops-alloydb
source_type: alloy_db
project: my-gcp-project
location: us-central1
cluster_id: my-cluster
instance_id: my-primary
database_id: opsdb
```

### Required fields

| Field | Description |
|-------|-------------|
| `source_type` | Must be `alloy_db` |
| `project` | GCP project ID |
| `location` | Region (e.g., `us-central1`) |
| `cluster_id` | AlloyDB cluster name |
| `instance_id` | AlloyDB instance name (usually the primary) |
| `database_id` | PostgreSQL database name |

### Optional fields

| Field | Description |
|-------|-------------|
| `context_set_id` | Pre-authored context set for improved accuracy |

## AlloyDB prerequisites

Before using CA with AlloyDB, the instance must be configured:

### 1. Enable the AlloyDB API

```bash
gcloud services enable alloydb.googleapis.com
```

### 2. Enable Data API Access

```bash
# Via REST API (v1beta)
curl -X PATCH \
  "https://alloydb.googleapis.com/v1beta/projects/PROJECT/locations/REGION/clusters/CLUSTER/instances/INSTANCE?updateMask=dataApiAccess" \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  -d '{"dataApiAccess": "ENABLED"}'
```

### 3. Create an IAM database user

```bash
gcloud alloydb users create USER@DOMAIN \
  --cluster=CLUSTER \
  --region=REGION \
  --type=IAM_BASED
```

## Usage examples

```bash
# Operational queries
bqx ca ask --profile ops-alloydb.yaml "show all tables in the database"
bqx ca ask --profile ops-alloydb.yaml "what are the largest tables by row count?"
bqx ca ask --profile ops-alloydb.yaml "show active connections"

# Business queries
bqx ca ask --profile ops-alloydb.yaml "total orders by region last 7 days"
bqx ca ask --profile ops-alloydb.yaml --format text "who are our top customers?"

# Debugging
bqx ca ask --profile ops-alloydb.yaml "show recent slow queries"
bqx ca ask --profile ops-alloydb.yaml "are there any locks or blocked queries?"
```

## Troubleshooting

### Error: "Data API Access is default for this instance"

The AlloyDB instance needs Data API enabled. See step 2 above.

### Error: "Internal error encountered" (500)

Common causes:
- IAM database user not created (see step 3 above)
- Instance recently created — wait a few minutes and retry
- Network/VPC configuration issues

### Error: "context_set_id not found" (404)

The `context_set_id` in the profile references a context set that does not
exist. Either create the context set or remove `context_set_id` from the
profile to query without pre-authored context.

## Decision rules

- Always verify Data API Access and IAM user before first use
- Start without `context_set_id` to confirm basic connectivity
- Add `context_set_id` later for improved query accuracy with authored context
- Use `--format text` for interactive ops work, `--format json` for automation
- AlloyDB uses PostgreSQL dialect — the generated SQL will be PostgreSQL

## Constraints

- AlloyDB requires VPC networking (Service Networking API must be enabled)
- Data API Access must be explicitly enabled on the instance
- IAM authentication must be configured for the calling identity
- Data agent creation (`ca create-agent`) is not supported for AlloyDB
