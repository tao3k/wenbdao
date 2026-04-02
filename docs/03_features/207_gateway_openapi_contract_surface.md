# Gateway OpenAPI Contract Surface

:PROPERTIES:
:ID: feat-gateway-openapi-contract-surface
:PARENT: [[index]]
:TAGS: feature, gateway, openapi, contracts, qianji
:STATUS: ACTIVE
:VERSION: 1.0
:END:

## Overview

`xiuxian-wendao` now ships one checked-in gateway `OpenAPI` artifact at
`resources/openapi/wendao_gateway.openapi.json` and exposes stable helpers in
`crate::gateway::openapi` so downstream contract lanes can consume the real
gateway surface without regenerating schemas during tests.

This gives `xiuxian-qianji` a file-backed input for `rest_docs` contract
feedback, keeps the runtime route inventory aligned with the bundled document,
and now supports both clean-surface validation and a deterministic persisted
downstream proof.

## Architecture Position

1. Route inventory: `src/gateway/openapi/paths.rs` defines stable route
   constants plus `WENDAO_GATEWAY_ROUTE_CONTRACTS`.
2. Runtime alignment: the gateway router uses those shared path constants
   instead of duplicating literal route strings.
3. Bundled artifact access: `src/gateway/openapi/document.rs` exposes:
   `bundled_wendao_gateway_openapi_document()`,
   `bundled_wendao_gateway_openapi_path()`, and
   `load_bundled_wendao_gateway_openapi_document()`.
4. Clean-surface validation: `xiuxian-qianji` runs
   `run_rest_docs_contract_feedback(...)` against the bundled artifact in
   `tests/integration/test_wendao_live_rest_docs_contract_feedback.rs`.
5. Persisted downstream validation: `xiuxian-qianji` derives a drifted copy of
   the bundled artifact and runs
   `run_and_persist_rest_docs_contract_feedback(...)` in
   `tests/integration/test_wendao_persisted_rest_docs_contract_feedback.rs` so
   Wendao-native entries are actually persisted through a sink.

## Contract Notes

- The bundled artifact is version-controlled and repository-local, so contract
  tests do not depend on runtime schema generation.
- The `/api/health` response now also carries `X-Wendao-Process-Id`, and the
  managed startup path writes `WENDAO_GATEWAY_PIDFILE` so process-compose can
  compare the header against the owned pidfile before treating the gateway as
  ready. The readiness probe also checks for `HTTP 200`, so a `503` response
  with the correct header still fails closed.
- The Valkey launch contract is shared now: both `process-compose` and the
  standalone `just valkey-*` path go through `scripts/channel/valkey-launch.sh`,
  and health checks go through `scripts/channel/valkey-healthcheck.sh`, so the
  ownership rule stays identical across launchers. The launcher is now fully
  environment-driven and no longer consumes `.config/xiuxian-artisan-workshop/valkey.conf`.
- The gateway inventory now also includes `GET /api/repo/sync`,
  `GET /api/repo/overview`, `GET /api/repo/module-search`, and
  `GET /api/repo/symbol-search`, `GET /api/repo/example-search`, and
  `GET /api/repo/doc-coverage`, plus `GET /api/docs/projected-gap-report`,
  `GET /api/docs/planner-item`, `GET /api/docs/planner-search`,
  `GET /api/docs/planner-queue`, `GET /api/docs/planner-rank`,
  `GET /api/docs/planner-workset`,
  `GET /api/docs/search`, `GET /api/docs/retrieval`,
  `GET /api/docs/retrieval-context`, `GET /api/docs/retrieval-hit`,
  `GET /api/docs/page`,
  `GET /api/docs/family-context`, `GET /api/docs/family-search`,
  `GET /api/docs/family-cluster`,
  `GET /api/docs/navigation`, `GET /api/docs/navigation-search`,
  `GET /api/repo/projected-pages`,
  `GET /api/repo/projected-page`, `GET /api/repo/projected-page-index-node`,
  `GET /api/repo/projected-gap-report`,
  `GET /api/repo/projected-retrieval-hit`,
  `GET /api/repo/projected-retrieval-context`,
  `GET /api/repo/projected-page-family-context`,
  `GET /api/repo/projected-page-family-cluster`,
  `GET /api/repo/projected-page-navigation`,
  `GET /api/repo/projected-page-navigation-search`,
  `GET /api/repo/projected-page-family-search`,
  `GET /api/repo/projected-page-index-tree`,
  `GET /api/repo/projected-page-index-tree-search`,
  `GET /api/repo/projected-page-search`, `GET /api/repo/projected-retrieval`,
  and
  `GET /api/repo/projected-page-index-trees`, which expose the Wendao Repo
  Intelligence source lifecycle, normalized repository overview, normalized
  module lookup, normalized symbol lookup, normalized example lookup,
  documentation coverage, deterministic docs-facing projected gap planning
  lookup, deterministic docs-facing planner-item opening, deterministic
  docs-facing planner gap discovery, deterministic docs-facing planner queue
  shaping, deterministic docs-facing planner priority ranking, deterministic
  docs-facing projected page search, deterministic docs-facing mixed retrieval
  lookup, deterministic docs-facing mixed retrieval-context lookup,
  deterministic docs-facing singular mixed-hit lookup, deterministic
  docs-facing projected page lookup, deterministic
  docs-facing projected page family-context lookup,
  deterministic docs-facing projected page family-search lookup,
  deterministic docs-facing projected page family-cluster lookup,
  deterministic docs-facing projected page navigation lookup, deterministic
  docs-facing projected page navigation search,
  deterministic Stage-2 projected page records,
  deterministic Stage-2 projected page lookup, deterministic Stage-2 projected
  page-index node lookup, deterministic deep-wiki projected gap planning
  lookup, deterministic singular mixed Stage-2 hit lookup,
  deterministic singular mixed Stage-2 context lookup, deterministic
  projected-page family context lookup, deterministic singular projected-page
  family cluster lookup, deterministic page-centric projected navigation
  bundle lookup, deterministic projected-page navigation bundle search,
  deterministic projected-page family cluster search,
  deterministic Stage-2 projected page-index tree lookup, deterministic
  Stage-2 projected page-index tree retrieval, deterministic Stage-2 projected
  page retrieval, deterministic mixed Stage-2 retrieval, and deterministic
  builder-native projected page-index trees through the same bundled OpenAPI
  contract surface instead of leaving them CLI-only.
- The repo-intelligence gateway tests now also pin stable bad-request
  contracts for missing `repo`, missing `query`, and invalid repo-sync `mode`,
  so the shared router helper path cannot drift those payloads silently.
- The bundled OpenAPI artifact now also carries static success and bad-request
  examples for the repo-intelligence endpoints, sourced from the same snapshot
  lane that validates the Studio gateway payloads.
- The strict `rest_docs` lane requires non-empty summaries and descriptions,
  success and error response coverage, and request examples for non-trivial
  bodies.
- `POST /api/ui/config` keeps an explicit JSON example in the bundled document,
  and the bundled gateway routes now include documented error responses so the
  real artifact stays clean under `REST-R003`.
- The bundled artifact also now explicitly carries `GET /api/search/index/status`
  again, so the checked-in OpenAPI document stays aligned with the runtime route
  inventory instead of silently dropping a live search-status path.
- The first docs namespace route intentionally reuses the same projected gap
  payload as the repo inspection lane, so the docs surface starts as a naming
  and navigation boundary instead of splitting deterministic deep-wiki planning
  into two competing schemas.
- The next docs namespace planner route follows the same rule:
  `GET /api/docs/planner-item` composes the existing projected gap, retrieval-hit,
  and navigation contracts into one deterministic work-item opener instead of
  inventing a docs-only planner schema, so deep-wiki planning can open one
  stable gap into a concrete page bundle without starting materialized wiki
  storage or LLM generation.
- The next docs namespace planner discovery route follows the same rule:
  `GET /api/docs/planner-search` reuses stable projected gap records and ranks
  them by deterministic planner evidence instead of inventing a docs-only
  planner backlog schema, so deep-wiki planning can discover candidate work
  items before opening them through `planner-item`.
- The next docs namespace planner backlog route follows the same rule:
  `GET /api/docs/planner-queue` groups stable projected gap records by
  deterministic gap kind instead of inventing a second planner entity schema,
  so deep-wiki planning can shape a backlog preview without leaving the stable
  projected-gap contract family.
- The next docs namespace planner ranking route follows the same rule:
  `GET /api/docs/planner-rank` reuses stable projected gap records and adds
  only deterministic priority scoring instead of inventing a second planner
  ranking entity schema, so deep-wiki planning can order candidate work items
  before opening them through `planner-item` or `planner-workset`.
- The next docs namespace planner ranking explanation refinement follows the
  same rule: `GET /api/docs/planner-rank` now carries machine-readable
  priority-reason entries alongside the deterministic priority score instead of
  inventing a second planner-explanation schema, so planners and UIs can show
  why one gap outranks another without reverse-engineering score math.
- The next docs namespace planner batch-opening route follows the same rule:
  `GET /api/docs/planner-workset` now composes the deterministic planner queue,
  deterministic planner-rank selection, and existing `planner-item` bundles
  instead of inventing a second workset entity model, so deep-wiki planning can
  preserve backlog shape, show why a workset was chosen, group the selected
  ranked hits by gap kind, nest those grouped hits by projected page family,
  and open a bounded batch of stable work items without leaving the
  projected-gap and navigation contract family.
- The next planner balancing refinement stays inside that same workset
  contract: `GET /api/docs/planner-workset` now also carries deterministic
  quota-band evidence for both populated gap-kind groups and populated
  page-family groups, including floor/ceiling target counts and
  `within_target_band` markers, so planner UIs can explain why one selected
  batch is considered balanced without inventing a second balancing schema.
- The next grouped-quota refinement also stays inside that same workset
  contract: `GET /api/docs/planner-workset` now carries explicit `quota` hints
  on each gap-kind lane and nested page-family lane, so planners can read
  stable per-group quota expectations directly from the grouped execution
  structure instead of joining the top-level balance summary back onto the
  lanes themselves.
- The next docs namespace route follows the same rule: `GET /api/docs/search`
  reuses the repo projected-page search payload instead of introducing a
  docs-only search schema, so the early deep-wiki surface stays contract-thin
  and planner-facing.
- The next docs namespace mixed retrieval route follows the same rule:
  `GET /api/docs/retrieval` reuses the repo projected mixed-retrieval payload
  instead of introducing a docs-only retrieval schema, so planner-facing docs
  discovery can span both projected pages and builder-native anchors without
  forking contracts.
- The next docs namespace mixed retrieval-context route follows the same rule:
  `GET /api/docs/retrieval-context` reuses the repo projected
  mixed-retrieval-context payload instead of introducing a docs-only
  retrieval-context schema, so planner-facing docs opening can expand one mixed
  hit into local related pages and optional node neighborhood without forking
  contracts.
- The next docs namespace singular mixed-hit route follows the same rule:
  `GET /api/docs/retrieval-hit` reuses the repo projected mixed-retrieval-hit
  payload instead of introducing a docs-only retrieval-hit schema, so
  planner-facing docs search and retrieval-context flows can reopen one stable
  mixed hit directly without dropping back to repo-prefixed inspection routes.
- The next docs namespace opening route follows the same rule:
  `GET /api/docs/page` reuses the repo projected-page lookup payload instead of
  introducing a docs-only page schema, so docs search and docs page can compose
  around one stable Stage-2 page contract.
- The next docs namespace family route follows the same rule:
  `GET /api/docs/family-context` reuses the repo projected-page family-context
  payload instead of introducing a docs-only grouped-family schema, so docs
  page opening can expand into planner-facing family groupings without leaving
  the deterministic Stage-2 contract family.
- The next docs namespace family discovery route follows the same rule:
  `GET /api/docs/family-search` reuses the repo projected-page family-search
  payload instead of introducing a docs-only grouped-family search schema, so
  docs discovery can return planner-facing family clusters without leaving the
  deterministic Stage-2 contract family.
- The next docs namespace family opening route follows the same rule:
  `GET /api/docs/family-cluster` reuses the repo projected-page family-cluster
  payload instead of introducing a docs-only grouped-family cluster schema, so
  docs discovery can reopen one requested family cluster without leaving the
  deterministic Stage-2 contract family.
- The next docs namespace context route follows the same rule:
  `GET /api/docs/navigation` reuses the repo projected-page navigation payload
  instead of introducing a docs-only navigation schema, so docs search, docs
  page, and docs navigation stay on one deterministic Stage-2 contract family.
- The next docs namespace discovery route follows the same rule:
  `GET /api/docs/navigation-search` reuses the repo projected-page navigation
  search payload instead of introducing a docs-only navigation-search schema,
  so docs discovery and docs opening can stay on one deterministic Stage-2
  contract family.
- The persisted downstream proof intentionally removes the `POST /api/ui/config`
  example from a temporary artifact copy so `REST-R007` produces one stable
  warning entry that can be persisted end-to-end through the Qianji sink path.

## Validation Targets

- `direnv exec . bash scripts/rust/xiuxian_wendao_live_openapi_contract_feedback.sh`
- `direnv exec . bash scripts/rust/xiuxian_wendao_contract_feedback_consumer.sh`
- `direnv exec . cargo test -p xiuxian-wendao --lib bundled_gateway_openapi_document_`
- `direnv exec . cargo test -p xiuxian-wendao --lib studio_repo_sync_api`
- `direnv exec . cargo test -p xiuxian-qianji --test wendao_live_rest_docs_contract_feedback`
- `direnv exec . cargo test -p xiuxian-qianji --test wendao_persisted_rest_docs_contract_feedback`

:RELATIONS:
:LINKS: [[03_features/203_agentic_navigation]], [[03_features/205_semantic_auditor]], [[03_features/206_openai_semantic_ignition]]
:END:
