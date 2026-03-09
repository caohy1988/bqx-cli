# Phase 1 Plan: MVP to v0.1

## Goal

Move `bqx` from the current demoable MVP to the Phase 1 target defined in
[README.md](/Users/haiyuancao/bqx-cli/README.md):

- stable core CLI scaffold
- `analytics doctor`, `evaluate`, `get-trace`
- `--exit-code` for CI/CD
- `json`, `table`, and `text` output
- auth support for ADC, service account, and `bqx auth login`
- npm distribution via `npx bqx`
- 5 core skills

Authority note:
the current MVP scope in
[docs/bqx_prd_rfc.md](/Users/haiyuancao/bqx-cli/docs/bqx_prd_rfc.md) is
intentionally narrower than the README roadmap. This plan targets the
README Phase 1 scope in §7, not the PRD's MVP stopping point.

Phase 1 is not about adding more analytics features. The MVP already proves
the core workflow. Phase 1 is about making the CLI installable, testable, and
usable in CI.

## Current State

Implemented:

- CLI scaffold with `clap` in [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- core commands in [src/commands](/Users/haiyuancao/bqx-cli/src/commands)
- `--exit-code` for `analytics evaluate`
- `json` and `table` output in [src/output.rs](/Users/haiyuancao/bqx-cli/src/output.rs)
- CI for formatting, lint, build, and tests in
  [.github/workflows/ci.yml](/Users/haiyuancao/bqx-cli/.github/workflows/ci.yml)
- **Milestone 1 complete**: credential resolver, `bqx auth login|status|logout`,
  PKCE+state OAuth, OS keychain storage, 5-level precedence chain, refresh
  token support, cross-platform random generation, 16 tests (2 unit + 14
  integration). See PRs #5 and #6.

Missing for Phase 1:

- `text` output format
- installable npm distribution
- 5 `SKILL.md` files
- golden and mocked integration tests
- release automation

## Scope

In scope:

- auth hardening
- install and packaging
- renderer completion
- tests
- skills
- docs alignment

Out of scope:

- dynamic Discovery Document generation
- new BigQuery resource commands
- Conversational Analytics
- additional analytics commands such as drift, insights, or views
- plugin system

## Architecture Direction

Keep the current MVP architecture and harden it. Do not redesign the command
surface.

Target modules:

```text
src/
├── auth/
│   ├── mod.rs
│   ├── resolver.rs
│   ├── login.rs
│   └── store.rs
├── bigquery/
├── commands/
├── cli.rs
├── config.rs
├── output.rs
└── main.rs
tests/
├── golden/
└── integration/
skills/
├── bqx-shared/
├── bqx-analytics/
├── bqx-analytics-evaluate/
├── bqx-analytics-trace/
└── bqx-query/
```

### Auth Model

Credential resolution order:

1. `BQX_TOKEN`
2. `BQX_CREDENTIALS_FILE`
3. stored `bqx auth login` credentials
4. `GOOGLE_APPLICATION_CREDENTIALS`
5. default ADC / `gcloud auth application-default`

This should be implemented in one resolver path so every command behaves the
same way.

### Output Model

Supported formats:

- `json`
- `table`
- `text`

`json` remains the contract-first default.

`table` remains for human scanning.

`text` should be command-specific, not generic:

- `doctor`: status summary and warnings
- `evaluate`: pass/fail summary plus top failing sessions
- `get-trace`: readable event timeline
- `jobs query`: compact row summary or plain structured dump for small results

## Milestones

### Milestone 1: Auth Completion ✅

Status: **Complete** (PRs #5, #6)

Objective:
replace ADC-only auth with a credential resolver and ship `bqx auth login`.

Delivered:

- `src/auth/` module: `resolver.rs`, `login.rs`, `store.rs`
- credential resolver with 5-level precedence:
  BQX_TOKEN → BQX_CREDENTIALS_FILE → stored login → GOOGLE_APPLICATION_CREDENTIALS → default ADC
- `bqx auth login` with installed-app OAuth, PKCE S256, and CSRF state
- `bqx auth status` reports active source with token validation
- `bqx auth logout` clears keychain and config
- OS keychain storage via `keyring`, config directory via `directories`
- refresh token support: stored login and authorized_user credentials files
  use `Refreshable` tokens that exchange on each `token()` call
- legacy token migration: stored tokens without client_id/secret fall back
  to static token instead of crashing
- cross-platform random generation via `rand` crate
- `--token` and `--credentials-file` CLI flags respected by all commands
  including `auth status`
- 16 tests: 2 unit tests (refresh path, static token), 14 integration tests
  (precedence, CLI flags, credentials file handling, status reporting, refresh
  path exercise, cross-platform random, dry-run)

Exit criteria met:

- every command authenticates through the same resolver path ✅
- both token and service-account credentials work without code changes ✅
- a user can authenticate without `gcloud` (via `bqx auth login`) ✅
- `bqx auth status` explains which auth source is active ✅

#### Auth Smoke-Check Procedure

Manual verification steps (not automated in CI due to interactive/credential
requirements):

1. **Interactive login**:
   ```sh
   bqx auth login
   # Opens browser → complete Google OAuth → "Authenticated as: user@example.com"
   bqx auth status
   # Reports: "bqx auth login (user@example.com)", "Token: valid"
   ```

2. **Service account via credentials file**:
   ```sh
   bqx auth status --credentials-file /path/to/sa.json
   # Reports: "credentials file: /path/to/sa.json", "Token: valid"
   ```

3. **Explicit token**:
   ```sh
   BQX_TOKEN=$(gcloud auth print-access-token) bqx auth status
   # Reports: "BQX_TOKEN / --token", "Token: valid"
   ```

4. **Precedence verification**: set multiple sources, verify highest wins:
   ```sh
   BQX_TOKEN=my-token bqx auth status --credentials-file /path/to/sa.json
   # Should report BQX_TOKEN (highest priority), not credentials file
   ```

5. **Data command end-to-end**:
   ```sh
   bqx jobs query --query "SELECT 1"
   # Should succeed with any active credential source
   ```

### Milestone 3: Output Completion

Objective:
finish the output contract for `text` and tighten renderer behavior.

Tasks:

- add `Text` to `OutputFormat` in [src/cli.rs](/Users/haiyuancao/bqx-cli/src/cli.rs)
- replace the mostly generic table rendering path in
  [src/output.rs](/Users/haiyuancao/bqx-cli/src/output.rs) with command-aware
  renderers where needed
- add command-specific `text` renderers
- keep generic JSON serialization for machine use

Done when:

- all Phase 1 commands support `json`, `table`, and `text`
- renderer output is stable enough for snapshot testing

Text output sketches:

- `jobs query`
  ```text
  Query complete: 5 rows
  Columns: session_id, agent, event_type, timestamp
  Row 1: adcp-f5a6b8bd92e8 | yahoo_sales_agent | AGENT_COMPLETED | 2026-03-05 08:42:32.323 UTC
  Row 2: adcp-87820945dd00 | yahoo_sales_agent | AGENT_COMPLETED | 2026-03-05 08:58:10.590 UTC
  ```
- `analytics doctor`
  ```text
  Status: warning
  Table: my-project.agent_analytics.agent_events
  Rows: 296  Sessions: 12  Agents: 1
  Latest event: 2026-03-05 09:27:54.474 UTC
  Warning: No recent data — last event was 5659 minutes ago.
  ```
- `analytics evaluate`
  ```text
  Evaluator: latency  Threshold: 5000  Window: 30d
  Sessions: 12  Passed: 0  Failed: 12  Pass rate: 0.00
  Worst sessions:
  - adcp-a20d176b82af  yahoo_sales_agent  score=32135.0
  - adcp-affa5aab2ee0  yahoo_sales_agent  score=26848.0
  ```
- `analytics get-trace`
  ```text
  Session: adcp-a20d176b82af
  Agent: yahoo_sales_agent
  Events: 32  Errors: false
  2026-03-05 09:26:59.270 UTC  LLM_REQUEST         OK
  2026-03-05 09:27:03.208 UTC  LLM_RESPONSE        OK  latency=3938
  2026-03-05 09:27:17.494 UTC  INVOCATION_COMPLETED OK latency=32135
  ```

### Milestone 4: Testability Refactor

Objective:
make command handlers testable without live BigQuery.

Tasks:

- introduce a narrow `QueryExecutor` trait in the BigQuery layer
- inject the executor into command handlers instead of constructing
  `BigQueryClient::new()` inside each handler
- keep SQL generation in pure helper functions so query construction can be
  unit-tested independently of execution
- add a fixture-backed executor for command tests

Done when:

- `doctor`, `evaluate`, and `get-trace` can be tested without network access

### Milestone 5: Golden and Integration Tests

Objective:
add real verification beyond `cargo test`.

Tasks:

- add golden tests for `json`, `table`, and `text` output
- add mocked integration tests for BigQuery request/response flows
- cover polling, pagination, error mapping, and auth-header injection

Recommended test areas:

- duration parsing
- identifier validation
- auth source precedence
- `jobs query --dry-run`
- `analytics doctor` healthy/warning/error cases
- `analytics evaluate` latency and error-rate cases
- `get-trace` empty, normal, and error-heavy sessions

Done when:

- output regressions are caught by snapshots
- network behavior is validated in CI without live GCP

### Milestone 6: Skills

Objective:
ship the 5 core Phase 1 skills.

Skills to add:

- `bqx-shared`
- `bqx-analytics`
- `bqx-analytics-evaluate`
- `bqx-analytics-trace`
- `bqx-query`

Tasks:

- create `skills/` directory
- write `SKILL.md` for each skill
- keep examples aligned with the actual CLI behavior
- cross-link shared prerequisites and safety guidance

Done when:

- all 5 skills are installable and internally consistent with the codebase

### Milestone 7: npm Distribution

Objective:
make `bqx` installable via `npx bqx`.

Tasks:

- build release binaries for macOS, Linux, and Windows
- create npm wrapper package with platform-specific binary resolution
- add versioning and packaging metadata
- smoke-test `npx bqx --help`

Done when:

- a clean machine can run `npx bqx --help`
- packaged binaries map to the right platform automatically

### Milestone 8: Release and CI Completion

Objective:
close the README Phase 1 exit criteria with automation.

Tasks:

- add release workflow for binary artifacts
- add npm publish workflow
- add CI job or documented example proving `analytics evaluate --exit-code`
  works in GitHub Actions
- add test matrix if needed for binary install coverage

Done when:

- the project can produce tagged release artifacts
- the install story is tested, not just documented

## Recommended Build Order

### Week 1

- Milestone 1: Auth Completion

Reason:
auth decisions affect CLI structure, configuration, and CI.

### Week 2

- Milestone 3: Output Completion
- Milestone 4: Testability Refactor

Reason:
output and testability are tightly coupled. Do them before snapshot tests.

### Week 3

- Milestone 5: Golden and Integration Tests
- Milestone 6: Skills

Reason:
skills should reflect stable command behavior, not a moving target.

### Week 4

- Milestone 7: npm Distribution
- Milestone 8: Release and CI Completion

Reason:
packaging should come after auth, output, and test contracts are stable.

## Detailed Task Breakdown

### Auth

- add global flags: `--token`, `--credentials-file`
- support env vars: `BQX_TOKEN`, `BQX_CREDENTIALS_FILE`
- decide config file location for stored auth metadata
- define token refresh behavior and failure messaging
- add `auth status` output that names the active credential source

### CLI and Config

- add `auth` top-level command
- keep current command names stable
- document which commands require dataset context
- make help text reflect actual auth precedence

### Output

- add `text` to help output
- define stable key order for JSON where practical
- stop relying on incidental field order for table columns
- ensure all commands have intentionally designed human-readable output

### Testing

- add fixture JSON responses for BigQuery APIs
- snapshot `json`, `table`, `text`
- add test coverage for auth resolution precedence
- add failure-case tests for invalid identifiers and missing required flags

### Packaging

- define npm package layout
- add platform-specific binary naming convention
- document local release build steps
- verify version alignment between Cargo and npm package metadata

### Skills

- add one shared skill for auth and global flags
- add examples matching the current MVP/Phase 1 command set
- avoid references to unimplemented features

## Open Decisions

These should be resolved before the first implementation PR so early work does
not stall on tooling debates.

### 1. Auth crates

Decision needed:

- which crates handle interactive OAuth, stored secrets, and config paths

Recommended default:

- `gcp_auth` for ADC and service-account flows
- `keyring` for storing refresh tokens or secrets in the OS keychain
- `directories` for config/state file locations
- `oauth2` only if `gcp_auth` is not sufficient for installed-app login

Why:

- this keeps the existing auth path intact
- `keyring` is a pragmatic Phase 1 choice for credential storage
- `directories` avoids custom path logic across platforms

Open question:

- whether to implement full installed-app OAuth in Phase 1 or accept a simpler
  login flow that stores externally acquired credentials

Recommended answer:

- implement installed-app OAuth only if it stays small; otherwise ship
  `auth status` plus token/service-account flows first and keep `auth login`
  narrowly scoped

### 2. npm packaging layout

Decision needed:

- how to structure the npm package and platform-specific binaries

Recommended default:

- one thin root npm package named `@bigquery/bqx`
- one platform package per target, using optional dependencies
- a small JS launcher that resolves the current platform binary

Suggested layout:

```text
npm/
├── package.json
├── bin/bqx.js
└── packages/
    ├── bqx-darwin-arm64/
    ├── bqx-darwin-x64/
    ├── bqx-linux-x64/
    └── bqx-win32-x64/
```

Why:

- this follows the same basic model used by `esbuild` and `turbo`
- it keeps `npx bqx` simple for users
- it cleanly separates JS install logic from Rust release artifacts

Open question:

- whether npm metadata should live at the repo root or under `npm/`

Recommended answer:

- keep npm packaging under `npm/` until the release flow is stable, then move
  to root only if it simplifies publishing

### 3. Test framework choices

Decision needed:

- which tools to standardize for unit, snapshot, and mocked integration tests

Recommended default:

- built-in Rust test framework for unit tests
- `insta` for snapshot/golden tests
- `wiremock` for mocked HTTP integration tests
- fixture files under `tests/fixtures/`

Why:

- `insta` is well-suited to stabilizing CLI output
- `wiremock` matches the README testing strategy and is sufficient for the
  BigQuery HTTP surface
- this stack is minimal and conventional for Rust CLI projects

Open question:

- whether to snapshot rendered text directly or snapshot normalized structs and
  renderer output separately

Recommended answer:

- do both where it matters:
  snapshot normalized data for logic-heavy commands and renderer output for
  user-facing stability

## Risks

### Risk 1: Overbuilding auth

If `bqx auth login` turns into a full account management subsystem, Phase 1
will slip.

Mitigation:

- only implement login and status
- store one active credential set
- defer multi-profile support

### Risk 2: Renderer churn

If output formats remain generic and ad hoc, snapshot tests will be noisy and
skills will drift from reality.

Mitigation:

- make renderers command-aware
- treat JSON shape as a contract

### Risk 3: Packaging before stabilization

If npm packaging starts before auth and outputs are stable, the team will lock
in a moving install surface.

Mitigation:

- package last, not first

## Definition of Done

Phase 1 is complete when all of the following are true:

- `bqx analytics doctor`, `evaluate`, and `get-trace` are stable
- `bqx auth login` works without `gcloud`
- service-account auth works in CI
- all Phase 1 commands support `json`, `table`, and `text`
- `npx bqx --help` works on a clean machine
- the 5 Phase 1 skills exist under `skills/`
- `cargo test` includes real unit, golden, and integration coverage
- GitHub Actions can run `bqx analytics evaluate --last=1h --exit-code`

## Suggested First PRs

1. `refactor(auth): introduce credential resolver and service-account support`
2. `feat(auth): add bqx auth login and auth status`
3. `feat(output): add text renderer and stabilize command-specific output`
4. `test(cli): add golden snapshots and mocked BigQuery integration tests`
5. `docs(skills): add 5 core Phase 1 skills`
6. `build(npm): package bqx for npx installation`
7. `ci(release): add binary and npm release workflows`
