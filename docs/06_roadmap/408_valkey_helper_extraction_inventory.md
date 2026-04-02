# 408 Valkey Helper Extraction Inventory

## Goal

Inventory duplicated Valkey helper patterns inside `xiuxian-wendao` before any
shared-infra refactor.

This note follows
`407_valkey_ownership_and_layering.md`: semantic Valkey state remains owned by
Wendao. Only thin transport helpers are candidates for extraction.

## Audit Summary

Current duplication appears in four main helper families:

1. client construction and connection opening
2. URL and environment fallback resolution
3. key-prefix normalization
4. timeout and connection policy

Serialization wrappers are also repeated, but they are lower priority than the
runtime and connection helpers.

## Duplicated Helper Families

### 1. Client Construction

Repeated patterns:

- `src/search_plane/cache.rs`
- `src/link_graph/saliency/store/common.rs`
- `src/link_graph/agentic/store/common.rs`
- `src/link_graph/stats_cache/valkey.rs`
- `src/link_graph/context_snapshot.rs`
- `src/link_graph/index/build/cache/storage.rs`
- `src/link_graph/index/build/graphmem.rs`
- `src/graph/valkey_persistence.rs`
- `src/storage/crud.rs`

Observed duplication:

- direct `redis::Client::open(...)`
- small wrapper functions such as `redis_client(...)`
- direct connection creation with inconsistent policy

Extraction candidate:

- one thin helper for `open_client(url, error_context)`

Not for extraction:

- domain-specific command sequences
- graph/search/link-graph error messages that embed domain names

### 2. URL and Environment Resolution

Repeated patterns:

- `src/search_plane/cache.rs`
- `src/link_graph/runtime_config/resolve/cache.rs`
- `src/graph/valkey_persistence.rs`
- `src/storage/crud.rs`

Observed duplication:

- different fallback chains for `VALKEY_URL`, `REDIS_URL`, and feature-local
  env vars
- some lanes use settings plus env
- some lanes use env only
- legacy storage still falls back to `redis://127.0.0.1/`

Extraction candidate:

- one thin helper family for:
  - `first_non_empty_env(...)`
  - `resolve_optional_valkey_url(...)`
  - `resolve_required_valkey_url(...)`

Constraint:

- search-plane, link-graph, graph, and legacy storage must still choose their
  own precedence rules; only the primitive resolution mechanics should be
  shared.

### 3. Key Prefix Normalization

Repeated patterns:

- `src/link_graph/saliency/store/common.rs`
- `src/link_graph/stats_cache/runtime.rs`
- `src/link_graph/context_snapshot.rs`
- `src/graph/valkey_persistence.rs`

Observed duplication:

- trim optional prefix
- fallback to domain default
- return owned `String`

Extraction candidate:

- one helper such as `normalize_key_prefix(candidate, default_prefix)`

Not for extraction:

- actual key builders like `saliency_key`, `stats_cache_key`,
  `suggested_link_stream_key`, `graph_snapshot_key`, or repo-manifest keys

These key builders express domain schema and should remain local.

### 4. Timeout and Connection Policy

Observed split:

- `src/search_plane/cache.rs` uses async connection config with env-driven
  connect/response timeouts
- `src/link_graph/saliency/store/common.rs` uses blocking connect/read/write
  timeouts
- most other blocking lanes use default connections with no explicit policy

Extraction candidate:

- shared timeout config types and conversion helpers
- maybe a blocking connection helper that accepts explicit timeout policy

Not yet ready for blind unification:

- search-plane is async
- saliency is blocking and hot-path sensitive
- graph/storage legacy lanes may intentionally stay simple until migrated

This makes timeout policy a second-phase extraction, not the first slice.

### 5. Serialization Wrappers

Observed duplication:

- repeated `serde_json::to_string(...)`
- repeated `serde_json::from_str(...)`
- repeated payload cleanup after invalid data

Candidate:

- small encode/decode wrappers only if a new shared Valkey infra module appears

Priority:

- lower than client/runtime/prefix extraction

## Recommended Extraction Boundary

The first shared helper layer should stay inside Wendao and stay very thin.

Recommended scope:

- client open helper
- optional/required URL resolution primitives
- prefix normalization helper
- timeout config helper types

Do not include:

- search-plane cache key builders
- repo publication manifest keyspace logic
- link-graph saliency keys
- suggested-link stream keys
- graph snapshot keys
- payload schemas
- domain-specific command flows

## Suggested Landing Zone

If this extraction happens, the first landing zone should remain Wendao-local,
for example a tiny internal module such as:

- `src/storage/valkey_common.rs`
- or `src/infra/valkey.rs`

Do not create a new cross-crate shared Valkey crate yet. The duplication has
been inventoried, but the runtime policies are not uniform enough to justify
that move today.

## Recommended Sequence

1. Introduce one Wendao-local helper module for client open, env resolution,
   and prefix normalization.
2. Migrate one lane first:
   - search-plane client resolution
   - or link-graph saliency runtime resolution
3. Re-audit timeout policy after one lane is migrated.
4. Only then decide whether a workspace-shared Valkey infra crate is justified.

## Explicit Non-Goals

This inventory does not authorize:

- moving semantic Valkey state into `xiuxian-vector`
- centralizing domain keyspaces under one shared crate
- rewriting payload schemas
- forcing all Valkey lanes onto a single timeout policy

## Landed Slices

The first nine code slices are now landed:

1. search-plane URL and optional-client resolution
2. saliency client-open and key-prefix normalization
3. agentic suggested-link client-open
4. stats-cache client-open and key-prefix normalization
5. context-snapshot client-open and key-prefix normalization
6. graph persistence URL resolution, client-open, and key-prefix normalization
7. legacy storage URL resolution and client-open
8. link-graph index cache client-open
9. link-graph graphmem client-open

Current landed surface:

- crate-private thin helper module: `src/valkey_common.rs`
- migrated callers:
  - `src/search_plane/cache.rs`
  - `src/link_graph/saliency/store/common.rs`
  - `src/link_graph/agentic/store/common.rs`
  - `src/link_graph/stats_cache/runtime.rs`
  - `src/link_graph/stats_cache/valkey.rs`
  - `src/link_graph/context_snapshot.rs`
  - `src/graph/valkey_persistence.rs`
  - `src/storage/crud.rs`
  - `src/link_graph/index/build/cache/storage.rs`
  - `src/link_graph/index/build/graphmem.rs`

Current helper coverage:

- `first_non_empty_env(...)`
- `resolve_optional_client_from_env(...)`
- `open_client(...)`
- `normalize_key_prefix(...)`

These slices keep search-plane keyspace logic, saliency timeout policy,
suggested-link stream behavior, stats-cache runtime precedence, snapshot
rollback behavior, graph snapshot schemas, key builders, and all payload
contracts local, including storage entry versioning and CRUD command flow.
Link-graph index cache runtime config continues to own its own prefix
normalization and TTL semantics, and graphmem sync continues to own its
saliency seeding, edge synchronization, and runtime-config behavior. The narrow
lib-test lane for the ninth slice is now green again after unrelated
`semantic_check` and `sentinel` test-visibility drift was repaired in the
current worktree.

## Next Bounded Step

The next implementation slice should stay small and transport-only:

- migrate one more remaining Valkey caller in a link-graph or storage-adjacent
  lane
- reuse `open_client(...)` and optional URL-resolution primitives where possible
- continue to leave domain key builders, timeout policy, payload schema, and
  graph/search semantics untouched
- keep lib-test blocker repair scoped to unrelated modularization drift instead
  of widening the Valkey extraction boundary
