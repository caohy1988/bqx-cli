# Using dcx in GitHub Actions

## Install dcx

```yaml
- uses: actions/setup-node@v4
  with:
    node-version: "20"
- run: npm install -g dcx
```

## Authenticate

### Option A: Workload Identity Federation (recommended)

```yaml
- uses: google-github-actions/auth@v2
  with:
    workload_identity_provider: ${{ vars.WIF_PROVIDER }}
    service_account: ${{ vars.SA_EMAIL }}
```

dcx picks up the generated credentials automatically via Application Default Credentials.

### Option B: Service account key

```yaml
- name: Write credentials file
  run: echo '${{ secrets.GCP_SA_KEY_JSON }}' > /tmp/sa-key.json
  env:
    # Store the full service account JSON as a repository secret
    GCP_SA_KEY_JSON: ${{ secrets.GCP_SA_KEY_JSON }}

- name: Configure dcx credentials
  run: echo "DCX_CREDENTIALS_FILE=/tmp/sa-key.json" >> "$GITHUB_ENV"
```

`DCX_CREDENTIALS_FILE` must point to a file path on disk, not the JSON content itself.

## CI quality gate with `analytics evaluate --exit-code`

The `--exit-code` flag makes dcx return exit code 1 when sessions fail the threshold, which fails the CI step.

### Full workflow example

```yaml
name: Agent Quality Gate

on:
  schedule:
    - cron: "0 */6 * * *"  # every 6 hours
  workflow_dispatch:

jobs:
  evaluate:
    name: Evaluate agent quality
    runs-on: ubuntu-latest
    permissions:
      contents: read
      id-token: write  # required for Workload Identity Federation

    steps:
      - uses: actions/setup-node@v4
        with:
          node-version: "20"

      - run: npm install -g dcx

      - uses: google-github-actions/auth@v2
        with:
          workload_identity_provider: ${{ vars.WIF_PROVIDER }}
          service_account: ${{ vars.SA_EMAIL }}

      - name: Check latency compliance
        run: |
          dcx analytics evaluate \
            --project-id "${{ vars.DCX_PROJECT }}" \
            --dataset-id "${{ vars.DCX_DATASET }}" \
            --evaluator latency \
            --threshold 5000 \
            --last 1h \
            --exit-code

      - name: Check error rate
        run: |
          dcx analytics evaluate \
            --project-id "${{ vars.DCX_PROJECT }}" \
            --dataset-id "${{ vars.DCX_DATASET }}" \
            --evaluator error-rate \
            --threshold 0.05 \
            --last 1h \
            --exit-code
```

### Required configuration

| Variable / Secret | Description |
|---|---|
| `vars.WIF_PROVIDER` | Workload Identity Federation provider resource name |
| `vars.SA_EMAIL` | Service account email with `bigquery.dataViewer` + `bigquery.jobUser` |
| `vars.DCX_PROJECT` | GCP project ID |
| `vars.DCX_DATASET` | BigQuery dataset containing `agent_events` |

### What the exit codes mean

| Exit code | Meaning |
|---|---|
| 0 | All sessions passed the threshold |
| 1 | One or more sessions failed the threshold |

### Deployment gate pattern

Add the evaluate job as a dependency for your deploy job:

```yaml
jobs:
  evaluate:
    # ... (as above)

  deploy:
    needs: evaluate
    runs-on: ubuntu-latest
    steps:
      - run: echo "Deploying — agent quality checks passed"
```

## Supported platforms

| Platform | Runner |
|---|---|
| macOS ARM64 (Apple Silicon) | `macos-latest` |
| macOS x64 (Intel) | `macos-13` |
| Linux x64 | `ubuntu-latest` |
| Linux ARM64 | `ubuntu-latest` (with cross-compile) |
| Windows x64 | `windows-latest` |
| Windows ARM64 | `windows-latest` (with cross-compile) |

## Release process

1. Ensure `Cargo.toml` version and all `npm/*/package.json` versions match
2. Tag: `git tag v0.0.1 && git push origin v0.0.1`
3. The `Release` workflow builds binaries, packages npm tarballs, and creates a GitHub Release
4. The `Publish npm` workflow publishes all packages to the npm registry
5. The `Smoke Install` workflow verifies `npx dcx --help` works on macOS, Linux, and Windows
