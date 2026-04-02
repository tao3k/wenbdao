# Studio Search Unused Warning Roadmap

## Context

This note records the remaining `unused` / `dead_code` warnings in the Studio search stack after the
duplicate compatibility layer in `analyzers/service/helpers.rs` was removed.

The goal is to distinguish:

- duplicate or misleading residue that should be deleted immediately
- valuable features that already have real implementation pieces but are not fully wired into the
  active HTTP surface yet

## Resolved Residue

The following warning sources were removed because they were architectural residue rather than
deferred product work:

- incorrect re-exports in `zhenfa_router/native/semantic_check/docs_governance/mod.rs`
- duplicate compatibility / placeholder layer in `analyzers/service/helpers.rs`
- redundant alias in `analyzers/projection/navigation_bundle.rs`

These items had no standalone roadmap value and only increased the risk of reconnecting callers to
stub behavior.

## Remaining Warning Clusters With Product Value

### 1. Native definition resolution is implemented but not fully wired

Relevant files:

- `src/gateway/studio/search/definition.rs`
- `src/gateway/studio/search/observation_hints.rs`
- `src/gateway/studio/search/source_index.rs`
- `src/gateway/studio/router/mod.rs`
- `src/gateway/studio/search/handlers.rs`
- `tests/unit/gateway/studio/search.rs`

Current state:

- the definition resolver, AST index construction, and Markdown `:OBSERVE:` hint extraction all
  exist
- `search_definition` in `handlers.rs` is still stubbed
- there is contract drift between the active handler shape and the older unit-test expectations

Why this should not be deleted:

- it is a real feature slice with working lower-level components
- it directly supports source-biased navigation and Markdown-guided resolution
- it is a better explanation for several current warnings than “dead code”

Roadmap:

1. restore a single canonical `search_definition` HTTP contract
2. wire the handler to `StudioState::ast_index()`
3. apply `definition_observation_hints()` when the source document is Markdown
4. update the unit snapshots to the restored contract

### 2. Native AST / symbol / reference / autocomplete search indexes are built but not exposed

Relevant files:

- `src/gateway/studio/search/source_index.rs`
- `src/gateway/studio/search/project_scope.rs`
- `src/gateway/studio/search/handlers.rs`
- `src/gateway/studio/router/mod.rs`

Current state:

- AST and symbol index builders already exist
- cache accessors on `StudioState` already exist
- the public handlers for `search_ast`, `search_symbols`, `search_references`, and
  `search_autocomplete` are still stubbed

Why this should not be deleted:

- these indexes are the intended backend for Studio-native search
- they are the missing bridge between existing indexing code and the current search surface

Roadmap:

1. wire `search_ast` to `StudioState::ast_index()`
2. wire `search_symbols` to `StudioState::symbol_index()`
3. derive `search_references` / `search_autocomplete` from the same cached indexes
4. remove any helper code that remains unused after the handlers are live

### 3. Markdown property-drawer analysis is blocked on the search wiring above

Relevant files:

- `src/gateway/studio/analysis/service.rs`
- `src/gateway/studio/analysis/markdown/compile.rs`
- `src/gateway/studio/analysis/markdown/property_drawers.rs`

Current state:

- property-drawer extraction is implemented
- the compile pipeline is only valuable once the Studio-native search handlers consume the AST /
  Markdown analysis outputs

Why this should not be deleted:

- it is the mechanism behind Markdown `:OBSERVE:` driven disambiguation
- it becomes live automatically once definition / AST search is wired

Roadmap:

1. land the definition-resolution wiring
2. verify that property-drawer derived hints appear in resolved navigation results
3. then re-evaluate whether any helper inside the Markdown compile pipeline is still unused

## Triage Rule Going Forward

For future warning cleanup in this crate:

- delete duplicated compatibility layers immediately
- do not keep placeholder implementations that return empty / default semantic payloads
- when a warning comes from a real but unwired feature slice, document the missing connection and
  wire the surface instead of deleting the core logic blindly
