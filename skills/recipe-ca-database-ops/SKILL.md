---
name: recipe-ca-database-ops
description: Step-by-step recipe for setting up database CA profiles (AlloyDB, Spanner, Cloud SQL) and running operational queries via natural language.
---

## When to use this skill

Use when the user wants to:
- Set up CA for a database (AlloyDB, Spanner, or Cloud SQL)
- Run operational or business queries against databases using natural language
- Configure multiple database profiles for different environments

## Prerequisites

Load the following skills: `dcx-ca`, `dcx-ca-database`

See **dcx-shared** for authentication and global flags.

## Recipe: Spanner (simplest setup)

### Step 1: Enable the API

```bash
gcloud services enable spanner.googleapis.com --project=PROJECT_ID
```

### Step 2: Create a profile

```bash
cat > ~/.config/dcx/profiles/finance-spanner.yaml << 'EOF'
name: finance-spanner
source_type: spanner
project: my-gcp-project
location: us-central1
instance_id: my-spanner-instance
database_id: my-database
EOF
```

### Step 3: Query

```bash
dcx ca ask --profile finance-spanner.yaml "show all tables"
dcx ca ask --profile finance-spanner.yaml "total revenue by region"
```

## Recipe: AlloyDB

### Step 1: Enable APIs and configure instance

```bash
# Enable APIs
gcloud services enable alloydb.googleapis.com servicenetworking.googleapis.com

# Enable Data API Access (via REST API)
curl -X PATCH \
  "https://alloydb.googleapis.com/v1beta/projects/PROJECT/locations/REGION/clusters/CLUSTER/instances/INSTANCE?updateMask=dataApiAccess" \
  -H "Authorization: Bearer $(gcloud auth print-access-token)" \
  -H "Content-Type: application/json" \
  -d '{"dataApiAccess": "ENABLED"}'

# Create IAM database user
gcloud alloydb users create USER@DOMAIN \
  --cluster=CLUSTER --region=REGION --type=IAM_BASED
```

### Step 2: Create a profile

```bash
cat > ~/.config/dcx/profiles/ops-alloydb.yaml << 'EOF'
name: ops-alloydb
source_type: alloy_db
project: my-gcp-project
location: us-central1
cluster_id: my-cluster
instance_id: my-primary
database_id: opsdb
EOF
```

### Step 3: Query

```bash
dcx ca ask --profile ops-alloydb.yaml "show all tables"
dcx ca ask --profile ops-alloydb.yaml "what are the largest tables?"
```

## Recipe: Cloud SQL

### Step 1: Enable APIs and configure instance

```bash
# Enable Data API Access
gcloud sql instances patch INSTANCE_ID --data-api-access=ALLOW_DATA_API

# Enable IAM authentication
gcloud sql instances patch INSTANCE_ID --database-flags=cloudsql.iam_authentication=on

# Create IAM user
gcloud sql users create USER@DOMAIN --instance=INSTANCE_ID --type=CLOUD_IAM_USER
```

### Step 2: Create a profile

```bash
cat > ~/.config/dcx/profiles/app-cloudsql.yaml << 'EOF'
name: app-cloudsql
source_type: cloud_sql
project: my-gcp-project
location: us-central1
instance_id: my-app-db
database_id: myapp
db_type: postgresql
EOF
```

### Step 3: Query

```bash
dcx ca ask --profile app-cloudsql.yaml "show all tables"
dcx ca ask --profile app-cloudsql.yaml "what is 1 + 1?"
```

## Multi-environment setup

Create profiles for different environments:

```bash
# Production
cat > deploy/ca/profiles/prod-spanner.yaml << 'EOF'
name: prod-spanner
source_type: spanner
project: prod-project
location: us-central1
instance_id: prod-instance
database_id: prod-db
EOF

# Staging
cat > deploy/ca/profiles/staging-spanner.yaml << 'EOF'
name: staging-spanner
source_type: spanner
project: staging-project
location: us-central1
instance_id: staging-instance
database_id: staging-db
EOF
```

Then use the right profile per environment:

```bash
dcx ca ask --profile deploy/ca/profiles/prod-spanner.yaml "active users today"
dcx ca ask --profile deploy/ca/profiles/staging-spanner.yaml "active users today"
```

## Verification checklist

For each database profile, verify:
1. The API is enabled (`gcloud services list --enabled`)
2. Data API access is enabled (AlloyDB and Cloud SQL only)
3. IAM authentication is configured (AlloyDB and Cloud SQL only)
4. Basic query works: `dcx ca ask --profile PROFILE "what is 1 + 1?"`
5. Schema query works: `dcx ca ask --profile PROFILE "show all tables"`

## Constraints

- Database sources use the QueryData API, not Chat/DataAgent
- No data agent creation for databases — use profiles directly
- No visualization rendering for database sources
- Each database type has different prerequisites (see per-type recipes above)
