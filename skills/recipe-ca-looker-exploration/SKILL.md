---
name: recipe-ca-looker-exploration
description: Step-by-step recipe for setting up and using Conversational Analytics with Looker explores for natural language data exploration.
---

## When to use this skill

Use when the user wants to:
- Set up CA for their Looker instance from scratch
- Explore Looker data using natural language questions
- Create a reusable Looker CA profile for their team

## Prerequisites

Load the following skills: `dcx-ca`, `dcx-ca-looker`

See **dcx-shared** for authentication and global flags.

## Recipe

### Step 1: Identify your Looker explores

You need:
- Your Looker instance URL (e.g., `https://mycompany.looker.com`)
- The LookML model name and explore name for each data source
- Optionally: Looker API client credentials (client_id + client_secret)

Explore references use the format `model_name/explore_name`. Find these in
your Looker instance under Explore > select an explore > check the URL.

### Step 2: Create a profile

```bash
cat > ~/.config/dcx/profiles/sales-looker.yaml << 'EOF'
name: sales-looker
source_type: looker
project: my-gcp-project
looker_instance_url: https://mycompany.looker.com
looker_explores:
  - sales_model/orders
  - sales_model/customers
  - sales_model/products
EOF
```

If your Looker instance requires API credentials:

```bash
cat > ~/.config/dcx/profiles/sales-looker.yaml << 'EOF'
name: sales-looker
source_type: looker
project: my-gcp-project
looker_instance_url: https://mycompany.looker.com
looker_explores:
  - sales_model/orders
  - sales_model/customers
looker_client_id: YOUR_CLIENT_ID
looker_client_secret: YOUR_CLIENT_SECRET
EOF
```

### Step 3: Validate the profile loads

```bash
# Should not produce errors
dcx ca ask --profile ~/.config/dcx/profiles/sales-looker.yaml "test"
```

If you see an error about the Looker instance, verify the URL and credentials.

### Step 4: Explore your data

```bash
# Start with broad questions
dcx ca ask --profile sales-looker.yaml "What are the top selling products?"

# Drill into specifics
dcx ca ask --profile sales-looker.yaml "Revenue by region last quarter"

# Get detailed results
dcx ca ask --profile sales-looker.yaml --format text \
  "Which customers have the highest lifetime value?"
```

### Step 5: Share with your team

```bash
# Copy the profile to a shared location or check it into your repo
cp ~/.config/dcx/profiles/sales-looker.yaml deploy/ca/profiles/

# Team members can use it directly
dcx ca ask --profile deploy/ca/profiles/sales-looker.yaml "monthly revenue trend"
```

## Tips

- Start with 1-2 explores and add more as needed (max 5)
- Use `--format text` for interactive exploration
- The CA API generates Looker queries — the `sql` field in the response
  may contain a Looker query URL instead of SQL
- If questions return unexpected results, check that the correct explores
  are referenced in the profile

## Constraints

- Maximum 5 explores per profile
- Looker CA requires the Chat/DataAgent API path
- OAuth credentials must be paired (both or neither)
- The Looker instance must be accessible from the CA API
