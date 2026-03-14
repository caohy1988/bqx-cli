---
name: recipe-self-diagnostic-agent
description: Recipe for building an AI agent self-correction loop where agents use bqx to monitor their own performance and adjust behavior based on analytics.
---

## When to use this skill

Use when the user wants to:
- Build an agent that monitors its own performance
- Create a self-correction loop using bqx analytics
- Have an agent adjust its behavior based on latency, error rates, or tool failures
- Implement autonomous agent health awareness

## Prerequisites

Load the following skills: `bqx-analytics`, `bqx-ca`, `bqx-analytics-evaluate`

See **bqx-shared** for authentication and global flags.

## Recipe

### Concept

A self-diagnostic agent periodically checks its own performance metrics using
bqx, then adjusts its behavior. For example:
- If latency is high, switch to a faster model or simpler tool chain
- If error rate is rising, fall back to cached responses
- If a specific tool is failing, route around it

### Step 1: Give your agent access to bqx

Add bqx commands as tools available to your agent. The agent needs:

```bash
# Self-evaluation tool
bqx analytics evaluate \
  --evaluator=latency \
  --threshold=5000 \
  --agent-id=<SELF_AGENT_ID> \
  --last=1h \
  --format=json

# Self-insights tool
bqx analytics insights \
  --agent-id=<SELF_AGENT_ID> \
  --last=1h \
  --format=json

# Natural language self-query
bqx ca ask "What's my error rate in the last hour?" \
  --agent=agent-analytics \
  --format=json
```

### Step 2: Define the diagnostic check

The agent runs a health check before processing complex requests:

```python
import subprocess
import json

def self_diagnostic(agent_id: str) -> dict:
    """Agent checks its own health metrics."""
    result = subprocess.run(
        [
            "bqx", "analytics", "evaluate",
            "--evaluator=latency",
            "--threshold=5000",
            f"--agent-id={agent_id}",
            "--last=1h",
            "--format=json",
        ],
        capture_output=True, text=True,
    )
    return json.loads(result.stdout) if result.returncode == 0 else {"status": "error"}
```

### Step 3: Implement the correction loop

```python
def handle_request(request, agent_id: str):
    # Step 1: Check own health
    health = self_diagnostic(agent_id)

    # Step 2: Decide strategy based on health
    pass_rate = health.get("pass_rate", 1.0)

    if pass_rate < 0.5:
        # Critical: use simplest possible approach
        return handle_with_fallback(request)
    elif pass_rate < 0.8:
        # Warning: skip expensive tools
        return handle_with_reduced_tools(request)
    else:
        # Healthy: full capability
        return handle_normally(request)
```

### Step 4: Add deeper diagnostics for failing agents

When the quick check fails, use insights and CA for root cause analysis:

```python
def diagnose_issues(agent_id: str) -> str:
    """Use CA to understand what's going wrong."""
    result = subprocess.run(
        [
            "bqx", "ca", "ask",
            f"What tools are failing for {agent_id} in the last hour?",
            "--agent=agent-analytics",
            "--format=json",
        ],
        capture_output=True, text=True,
    )
    if result.returncode == 0:
        data = json.loads(result.stdout)
        return data.get("explanation", "No explanation available")
    return "Diagnostic unavailable"
```

### Step 5: Log diagnostic decisions

Record the agent's self-diagnostic decisions for observability:

```python
import logging

def handle_request_with_logging(request, agent_id: str):
    health = self_diagnostic(agent_id)
    pass_rate = health.get("pass_rate", 1.0)

    strategy = "normal" if pass_rate >= 0.8 else "reduced" if pass_rate >= 0.5 else "fallback"
    logging.info(f"agent={agent_id} pass_rate={pass_rate} strategy={strategy}")

    # The strategy decision itself becomes a traceable event
    # in the next analytics cycle
```

### Step 6: Set up periodic health monitoring

For agents that run continuously, add a background health loop:

```bash
# Run every 5 minutes as a background check
while true; do
  bqx analytics evaluate \
    --evaluator=error_rate \
    --threshold=0.10 \
    --agent-id=my-agent \
    --last=15m \
    --exit-code \
    --format=json > /tmp/agent-health.json 2>&1 || \
    echo "WARNING: Agent health degraded" >> /var/log/agent-health.log

  sleep 300
done
```

## Decision rules

- Run diagnostics before expensive operations, not on every request
- Use `--last=1h` for real-time decisions, `--last=15m` for fast detection
- Define clear thresholds: what pass_rate triggers fallback vs. reduced vs. normal
- Log strategy decisions so they appear in the next analytics cycle
- Use `--exit-code` in scripts to branch on health status
- Use CA (`bqx ca ask`) for root cause analysis, not just metric checks

## Constraints

- Self-diagnostic adds latency — only run on complex/expensive requests
- The agent must have IAM permissions to read its own analytics data
- Self-correction logic is application-specific — bqx provides data, not remediation
- Avoid tight feedback loops where diagnostic overhead worsens the metrics being measured
- CA queries require a configured data agent (see `recipe-ca-data-agent-setup`)
