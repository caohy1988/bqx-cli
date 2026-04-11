# BigQuery CLI Benchmark Results: dcx vs bq

Systematic latency and correctness comparison of `dcx` against the standard
`bq` CLI across 12 BigQuery tasks covering metadata reads, SQL queries,
dry-run validation, and error handling.

## Key Numbers

| Metric | dcx | bq |
|--------|-----|-----|
| **Average task p50 (11 valid tasks)** | 571 ms | 2,748 ms |
| **Geometric mean speedup** | **6.0x faster** | baseline |
| **Correctness (warm trials)** | 33/33 (100%) | 30/33 (91%) |

One task (`bq-error-permission-denied`) is excluded from the primary
scorecard because it targets a public project and does not produce a
permission error for either CLI — see [Correctness Notes](#correctness-notes).
Including it: average p50 672 ms / 2,870 ms, geomean 5.5x, correctness
33/36 (92%) / 30/36 (83%).

Results were directionally stable across 3 warm trials per task; see the
[scorecard p95s](../benchmarks/results/scorecards/20260410-171926-cdc4d94.md)
for variance bounds.

**Benchmark contract:** Correctness is defined by the deterministic
validation rules checked into
[`benchmarks/tasks/bigquery_overlap.yaml`](../benchmarks/tasks/bigquery_overlap.yaml)
— per-variant `json_keys`, `json_parse`, `semantic_sql_result`, or
`exit_code_only` checks executed automatically by the runner. No manual
judgment is involved in pass/fail determination.

## Scope and Limits

This benchmark measures a specific, narrow slice:

- **Single machine:** macOS (Darwin 25.3.0, x86_64), no cross-platform data
- **Single project:** one GCP project with ADC auth, no service-account or
  token-based flows
- **12 tasks:** 4 metadata reads, 2 SQL queries, 1 dry-run, 5 error-handling
  — read-heavy, no mutations, no concurrent load
- **One known bad task:** `bq-error-permission-denied` targets a public
  project and does not actually produce a permission error for either CLI
- **3 warm trials per task:** sufficient for directional conclusions, not for
  percentile-level statistical claims
- **No cloud-side telemetry:** bytes processed, slot milliseconds, and cache
  hit rates were not captured in this run

The results are directionally strong but should be validated on Linux CI
hosts and with higher trial counts before quoting in external materials.

## Test Environment

| | |
|---|---|
| Run ID | `20260410-171926-cdc4d94` |
| dcx version | 0.5.0 (Rust, compiled release) |
| bq version | BigQuery CLI 2.1.28 (Python) |
| gcloud SDK | 559.0.0 |
| OS | macOS (Darwin 25.3.0, x86_64) |
| Auth | Application Default Credentials (ADC) |
| Region | us-central1 |
| Trials per task | 1 cold + 3 warm |

## Per-Task Results

### Metadata Operations

These are the bread-and-butter commands an agent uses to explore a project
before writing SQL: list datasets, inspect a dataset, list tables, get a
table's schema.

| Task | dcx p50 | bq p50 | Speedup | dcx | bq |
|------|--------:|-------:|--------:|:---:|:--:|
| List datasets | 966 ms | 3,005 ms | **3.1x** | PASS | PASS |
| Get dataset metadata | 725 ms | 2,814 ms | **3.9x** | PASS | PASS |
| List tables | 581 ms | 2,725 ms | **4.7x** | PASS | PASS |
| Get table schema | 572 ms | 2,607 ms | **4.6x** | PASS | PASS |

**Average metadata speedup: 4.1x.**

### SQL Queries

| Task | dcx p50 | bq p50 | Speedup | dcx | bq |
|------|--------:|-------:|--------:|:---:|:--:|
| Aggregate query (`COUNT`, `AVG`) | 856 ms | 3,528 ms | **4.1x** | PASS | PASS |
| Nested-field query (STRUCT access) | 973 ms | 3,201 ms | **3.3x** | PASS | PASS |

Query execution time is dominated by BigQuery server-side processing, so the
speedup here is ~3.7x rather than the higher ratios seen in metadata ops. In
this benchmark, dcx overhead appears small relative to total request time.

### Dry-Run (SQL Validation)

| Task | dcx p50 | bq p50 | Speedup | dcx | bq |
|------|--------:|-------:|--------:|:---:|:--:|
| Dry-run aggregate query | 90 ms | 2,990 ms | **33.2x** | PASS | PASS |

This is the most dramatic result. `dcx --dry-run` resolves entirely locally
(validates flags, builds the request, and returns the structured request body)
without any network call. `bq --dry_run` makes an API round-trip to BigQuery
servers and returns estimated bytes processed.

**Fairness note:** these are not identical product semantics. `dcx --dry-run`
previews the outbound request; `bq --dry_run` performs a server-side
validation pass. The 33.2x result is valid operationally — an agent using
dry-run to check flag/SQL structure before executing saves ~3 seconds per
step — but it is not a pure apples-to-apples API round-trip comparison.

### Error Handling

| Task | dcx p50 | bq p50 | Speedup | dcx | bq |
|------|--------:|-------:|--------:|:---:|:--:|
| Malformed SQL | 593 ms | 2,687 ms | **4.5x** | PASS | PASS |
| Nonexistent dataset | 646 ms | 2,823 ms | **4.4x** | PASS | PASS |
| Invalid auth | 185 ms | 2,979 ms | **16.1x** | PASS | FAIL |
| Invalid flag | 91 ms | 871 ms | **9.6x** | PASS | PASS |
| Permission denied | 1,784 ms | 4,215 ms | **2.4x** | * | * |

Error-handling speed matters because agent self-correction loops depend on
fast feedback. When an agent issues a bad query and needs to retry, `dcx`
returns the error 3–16x faster.

**Auth failure (16.1x):** `dcx` detects the invalid credential locally and
exits in 185 ms. `bq` with `--credential_file /dev/null` exits 0 and returns
data — the explicit credential override appears not to be honored in this
scenario, and `bq` falls back to ADC silently.

**Invalid flag (9.6x):** `dcx` validates flags locally (91 ms) before making
any network call. `bq` still takes ~870 ms to report the same error.

\* Permission-denied task targets `bigquery-public-data`, which is publicly
accessible. Both CLIs return exit 0 with data instead of a permission error.
This is a test-design issue, not a CLI issue.

## Architectural Factors (Inference)

The sections above report measured results. This section offers an
architectural explanation for the observed latency gap. These factors are
inferred from code-level knowledge of both CLIs, not directly measured by
this benchmark.

| Factor | dcx | bq |
|--------|-----|-----|
| **Language** | Compiled Rust binary | Python interpreter |
| **Startup** | ~5 ms (estimated) | ~300–500 ms (estimated) |
| **API calls** | Direct REST, minimal framing | Wraps `google-api-python-client` |
| **Auth** | Loads ADC/token directly | Routes through `gcloud auth` subsystem |
| **Validation** | Validates flags before network | Validates after starting API flow |

The Python interpreter startup cost appears in every `bq` invocation. In a
5-step agent workflow, this alone would add 1.5–2.5 seconds of overhead —
consistent with the ~14-second gap observed in the workflow rollup below.
Isolating startup from API latency was not done in this run.

## Agent Workflow Impact

Consider a typical agent exploration loop:

```
list datasets → pick dataset → list tables → get schema → dry-run query → execute query
```

That's 6 sequential CLI calls. Using warm p50 numbers:

| | dcx | bq |
|---|---|---|
| Total latency | 966 + 725 + 581 + 572 + 90 + 856 = **3,790 ms** | 3,005 + 2,814 + 2,725 + 2,607 + 2,990 + 3,528 = **17,669 ms** |
| Wall clock | **~4 seconds** | **~18 seconds** |

An agent using `dcx` completes the same exploration in **4 seconds vs 18
seconds** — a 4.7x end-to-end speedup. For iterative workflows where the
agent retries 2–3 times (common with self-correction), the gap widens to
~12 seconds vs ~54 seconds.

## Token Efficiency

Agent workflows pay for every byte of CLI output that enters the LLM context
window. This section estimates token cost using the approximation
**1 token ~ 4 bytes** (conservative for JSON with repeated keys).

### Per-Task Token Estimates

| Task | dcx stdout | ~dcx tokens | bq stdout | ~bq tokens |
|------|--------:|--------:|--------:|--------:|
| List datasets | 7,699 B | ~1,925 | 5,501 B | ~1,375 |
| Get dataset | 812 B | ~203 | 650 B | ~163 |
| List tables | 704 B | ~176 | 488 B | ~122 |
| Get table schema | 1,272 B | ~318 | 203 B | ~51 |
| Dry-run | 350 B | ~88 | 1,317 B | ~329 |
| Aggregate query | 599 B | ~150 | 302 B | ~76 |
| Nested query | 748 B | ~187 | 451 B | ~113 |

### Agent Workflow Token Budget

For the 6-step exploration loop (list datasets → get dataset → list tables →
get schema → dry-run → execute query):

| | dcx | bq |
|---|---:|---:|
| Total output | 11,436 B | 8,461 B |
| Estimated tokens | **~2,859** | **~2,115** |

`dcx` uses ~35% more tokens per workflow because it includes the
`items`/`source` envelope and richer field details in metadata responses.
However, both totals are small — under 3K tokens per full exploration, a
fraction of a typical 128K-token context window.

The tradeoff is **parseability vs compactness**. `dcx` normalizes all list
responses to a consistent envelope:

```json
{"items": [...], "source": "BigQuery", "next_page_token": "..."}
```

`bq` returns raw API shapes that vary by command (JSON array, nested
`datasets` key, free-text error strings on stdout). An agent using `bq` must
carry per-command parsing logic in its prompt or tool definitions, which
itself consumes tokens. `dcx`'s uniform envelope covers every list command;
get, query, dry-run, and error responses still have their own shapes, but
the list normalization alone reduces the per-command parsing burden for the
most common discovery operations.

### Error Output Token Cost

Error handling reveals a structural difference in where each CLI puts
diagnostic information:

| Error task | dcx stderr | ~tokens | bq stdout | ~tokens |
|-----------|--------:|--------:|--------:|--------:|
| Malformed SQL | 174 B | ~44 | 170 B | ~43 |
| Not found | 187 B | ~47 | 106 B | ~27 |
| Auth failure | 326 B | ~82 | 5,501 B* | ~1,375 |
| Invalid flag | 143 B | ~36 | 101 B | ~25 |

`dcx` emits errors as structured JSON on stderr (`{"error":"..."}`), keeping
stdout clean. `bq` mixes error text into stdout for some errors and uses
stderr for others, with no consistent format.

\* `bq` returns the full dataset listing (5.5 KB) on the auth-failure task
because it falls back to ADC — the agent receives ~1,375 tokens of
unintended successful response instead of an error signal.

## Correctness Notes

Three tasks had validation outcomes worth discussing:

1. **bq-error-auth-failure:** `bq` exits 0 despite being given
   `--credential_file /dev/null`. The explicit credential override appears
   not to be honored in this scenario; `bq` falls back to ADC and succeeds.
   `dcx` returns exit 1 with a structured error.

2. **bq-error-permission-denied:** Both CLIs exit 0 because the test targets
   `bigquery-public-data`, which grants public read access. The task spec
   should be updated to target a genuinely restricted project.

3. **Overall correctness:** Excluding the permission-denied design issue,
   `dcx` passes 33/33 trials (100%). `bq` passes 30/33 (91%), with the
   auth-failure task accounting for all 3 failures.

## Product Implications

For `bq` CLI:

- **Normalized machine-output mode.** `bq` list commands return varying JSON
  shapes. A `--format=machine-json` mode with a stable envelope (items array,
  pagination token, source tag) would make `bq` output parseable without
  per-command logic.
- **Credential override behavior.** `--credential_file /dev/null` is silently
  ignored and ADC is used instead. Clarifying or fixing this would make
  explicit credential overrides trustworthy for CI and testing scenarios.
- **Local-only dry-run preview.** `bq --dry_run` always makes a server
  round-trip. A local-only mode that shows the constructed request (method,
  URL, body) without network access would enable faster agent preflight
  checks and offline validation.

For `dcx`:

- **Output size.** `dcx` metadata responses are ~35% larger than `bq`
  equivalents due to the `items`/`source` envelope. For token-sensitive agent
  workflows, a compact output mode could reduce this overhead.
- **Permission-denied test gap.** The current benchmark does not exercise a
  real permission-denied scenario. Adding a task against a restricted project
  would validate dcx's error classification for this case.

## Next Benchmark

- Rerun on a Linux CI host (e.g., GitHub Actions runner) to confirm the
  speedup is not macOS-specific
- Rerun with a genuinely restricted project for the permission-denied task
- Add bytes-processed / job metadata appendix using
  `benchmarks/scripts/collect_bigquery_jobs.sql`
- Increase warm trials to 10+ for percentile-level statistical claims
- Test with `gcloud alpha bq` if relevant as an alternative baseline

## Artifacts

- [Scorecard](../benchmarks/results/scorecards/20260410-171926-cdc4d94.md)
- [Summary JSON](../benchmarks/results/raw/20260410-171926-cdc4d94/summary.json)
- [Raw results (NDJSON)](../benchmarks/results/raw/20260410-171926-cdc4d94/results.ndjson)
- [Environment snapshot](../benchmarks/results/raw/20260410-171926-cdc4d94/environment.json)
- [Benchmark methodology](cli_benchmark_plan.md)

## Reproduction

```bash
# Build dcx
cargo build --release
export PATH="target/release:$PATH"

# Seed benchmark data
benchmarks/scripts/seed_bigquery.sh YOUR_PROJECT_ID

# Configure manifest
sed -i '' 's/YOUR_PROJECT_ID/your-actual-project/' benchmarks/manifest.yaml

# Run (1 cold + 3 warm trials)
benchmarks/scripts/run_benchmarks.sh --tasks bigquery_overlap --trials 3 --cold-trials 1

# Generate scorecard
python3 benchmarks/scripts/score_results.py benchmarks/results/raw/<run-id>
```
