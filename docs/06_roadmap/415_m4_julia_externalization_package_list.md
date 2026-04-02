# M4 Julia Externalization Package List

:PROPERTIES:
:ID: wendao-m4-julia-externalization-package-list
:PARENT: [[index]]
:TAGS: roadmap, migration, plugins, julia, m4
:STATUS: ACTIVE
:END:

## Purpose

This note is the first concrete `M4` deliverable for the Wendao
core/runtime/plugin migration program.

It defines the package list for externalizing Julia ownership into
`xiuxian-wendao-julia`.

Primary references:

- `[[06_roadmap/412_core_runtime_plugin_program]]`
- `[[06_roadmap/413_m2_core_extraction_package_list]]`
- `[[06_roadmap/414_m3_runtime_extraction_package_list]]`
- `[[06_roadmap/409_core_runtime_plugin_surface_inventory]]`
- `[[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]]`
- `[[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]`

## M4 Goal

Make `xiuxian-wendao-julia` the physical owner of Julia-specific plugin
behavior.

At the end of `M4`, the host should consume Julia through:

1. stable contracts from `core`
2. orchestration from `runtime`
3. Julia-owned plugin declarations and launch/artifact semantics from
   `xiuxian-wendao-julia`

## Externalization Rule

A Julia-related boundary belongs in `xiuxian-wendao-julia` if it is true that:

1. it expresses Julia-specific capability behavior
2. it encodes Julia-specific launch or CLI semantics
3. it owns Julia-specific deployment artifact meaning
4. it exists only to bridge Wendao contracts into Julia package behavior

If a boundary is only a stable contract record, it should move to `core`.
If it is only host orchestration, it should move to `runtime`.

## Current Blocking Reality

The first dependency blocker has now been removed:

1. `xiuxian-wendao-julia` no longer depends on `xiuxian-wendao` directly
2. repo-intelligence contracts and the Julia Arrow analyzer
   column/schema-contract surface now come from `xiuxian-wendao-core`
3. the former `M4` blockers are no longer Cargo-edge blockers; they have been
   reduced and then externalized as Julia-owned compatibility seams for launch
   semantics, deployment artifact meaning, package-path/default ownership, and
   the Julia rerank runtime record
4. the host now consumes Julia through a normal crate dependency instead of
   sibling-source inclusion under `src/analyzers/languages/mod.rs`

The remaining Julia work has now shifted from ownership externalization to
compatibility retirement and generic-surface cutover.

## Candidate Package List

### Package A: Julia Capability Declarations

Current source boundary:

- Julia plugin entry and capability declaration code
- Julia capability metadata that is currently bridged through the host crate

Target `xiuxian-wendao-julia` ownership:

1. Julia capability identifiers and declarations
2. Julia provider metadata
3. Julia-owned capability registration surfaces

Do not keep in host:

1. Julia-specific capability declaration assembly
2. Julia plugin entrypoint wiring as host-owned implementation

### Package B: Julia Launch and Manifest Semantics

Current source boundary:

- Julia launch-manifest semantics currently represented by
  `LinkGraphJuliaAnalyzerLaunchManifest`
- Julia service descriptor assembly
- Julia CLI argument meaning

Target `xiuxian-wendao-julia` ownership:

1. Julia launch manifest schema interpretation
2. Julia CLI ordering and package-owned defaults
3. Julia launcher path and package-owned startup semantics

Do not keep in host:

1. Julia-specific ordered CLI argument assembly as primary ownership
2. Julia service descriptor semantics as host-owned meaning

### Package C: Julia Deployment Artifact Semantics

Current source boundary:

- Julia deployment artifact meaning currently surfaced through
  `LinkGraphJuliaDeploymentArtifact`
- Julia-specific TOML/JSON deployment contract meaning

Target `xiuxian-wendao-julia` ownership:

1. Julia deployment artifact schema interpretation
2. Julia artifact metadata fields and package-owned defaults
3. Julia package-facing export contract

Host may still own:

1. generic artifact payload record shape
2. generic artifact resolution/routing
3. compatibility shims during migration

### Package D: Julia Runtime Defaults and Package Paths

Current source boundary:

- Julia-specific env-var defaults
- `.data/WendaoArrow` and `.data/WendaoAnalyzer` package path conventions
- Julia launch defaults that are currently host-side

Target `xiuxian-wendao-julia` ownership:

1. Julia package path conventions
2. Julia default launcher and script locations
3. Julia package-owned transport defaults where not part of generic contracts

Do not keep in host:

1. package-owned Julia script names as primary host constants
2. Julia package path knowledge as long-term host ownership

### Package E: Julia Compatibility Surface

Current source boundary:

- legacy Julia outward shims in gateway and Zhenfa compatibility seams
- historical note: the temporary crate-root shim `src/compatibility/julia.rs`
  was used during the `M4 -> M5` bridge and is now retired from the live tree

Target ownership after `M4`:

1. host keeps only thin compatibility wrappers at the explicit crate-root
   compatibility namespace, which is now `src/compatibility/link_graph.rs`
2. Julia package becomes the physical owner of Julia-specific meaning
3. all compatibility wrappers delegate into `core`/`runtime` + Julia plugin
   ownership

## Explicit Non-Julia-Package List

These boundaries must remain out of `xiuxian-wendao-julia`:

### Core-Owned

1. plugin ids and selector record shapes
2. capability and artifact descriptor records
3. generic launch-spec and artifact-payload records

### Runtime-Owned

1. host config discovery and filesystem override resolution
2. transport client construction and fallback orchestration
3. gateway assembly
4. Zhenfa router execution
5. registry/bootstrap orchestration

## Dependency Rewrite Target

The desired dependency shape after `M4` is:

```text
xiuxian-wendao-julia
  -> xiuxian-wendao-core
  -> optional narrow runtime integration seam if unavoidable

xiuxian-wendao-runtime
  -> xiuxian-wendao-core
  -> xiuxian-wendao-julia

xiuxian-wendao
  -> facade / compatibility / transitional assembly
```

The key rule is:

`xiuxian-wendao-julia` must stop depending on the monolithic host crate as its
primary dependency surface.

## First Physical Externalization Cut

The first physical `M4` cut should aim to move:

1. Julia launch-manifest meaning
2. Julia deployment artifact meaning
3. Julia package path/default ownership

It should not attempt to remove every Julia compatibility shim in one landing.

Current implementation status:

1. `xiuxian-wendao-julia` now has a direct dependency on
   `xiuxian-wendao-core`
2. repo-intelligence contract imports in the Julia plugin entry, discovery,
   linking, project, sources, and transport modules now source stable records
   and traits from `xiuxian-wendao-core::repo_intelligence`
3. the monolithic host analyzer contract modules now re-export that same
   repo-intelligence slice from `xiuxian-wendao-core`, so the Julia package no
   longer depends on `xiuxian-wendao` for those stable contracts
4. the Julia Arrow analyzer column/schema contract also now lives in
   `xiuxian-wendao-core::repo_intelligence`, which removes the last direct
   `xiuxian-wendao` Cargo dependency from the Julia package
5. `xiuxian-wendao` now loads `xiuxian-wendao-julia` through a normal crate
   dependency instead of `#[path]` source inclusion, so Julia publication is
   no longer blocked by host-side source embedding
6. `xiuxian-wendao-julia::compatibility::link_graph` now owns the Julia
   plugin selector ids/helpers, `LinkGraphJuliaAnalyzerServiceDescriptor`,
   `LinkGraphJuliaAnalyzerLaunchManifest`,
   `LinkGraphJuliaDeploymentArtifact`, the Julia CLI-arg mapping for analyzer
   launch, and the conversion boundary between those Julia DTOs and
   `PluginLaunchSpec` / `PluginArtifactPayload`
7. the monolithic host now keeps `launch.rs` and `artifact.rs` only as
   compatibility re-export seams for those Julia-owned DTOs, while
   `runtime.rs` now delegates Julia analyzer-launch arg encoding back into the
   Julia crate
8. `xiuxian-wendao-julia::compatibility::link_graph` now also owns the Julia
   analyzer package-dir/default path slice through `paths.rs`, including the
   default analyzer launcher path and the default analyzer example-config path,
   so the monolithic host no longer carries those package-owned defaults in
   `runtime_config/constants.rs`
9. the host runtime/tests and integration fixtures now consume those
   Julia-owned path defaults instead of embedding raw
   `.data/WendaoAnalyzer/...` literals across the touched `M4` seams
10. `xiuxian-wendao-julia::compatibility::link_graph` now also owns
    `LinkGraphJuliaRerankRuntimeConfig` and its provider-binding / launch /
    artifact normalization methods through `runtime.rs`
11. the host `runtime.rs` and `conversions.rs` files now behave as
    compatibility seams over that Julia-owned runtime record, so the hard
    ownership blockers for `M4` are now cleared
12. the staged mixed-graph structural plugin contract now also follows the
    same ownership rule: Julia-specific graph-structural route names, draft
    schema-version defaults, request or response column inventories, and Arrow
    batch validation live in `xiuxian-wendao-julia`, while
    `xiuxian-wendao-runtime` stays limited to reusable Flight client and route
    normalization helpers

## Compatibility Plan

During `M4`:

1. deprecated Julia-named host exports may remain
2. host compatibility seams may remain, but only as wrappers
3. Julia package should increasingly become the source of truth for Julia
   behavior

`M4` is complete only when Julia-specific meaning lives physically outside the
monolithic crate.

## Acceptance Criteria

This package list is ready when:

1. Julia-owned boundaries are explicitly identified
2. non-Julia-owned boundaries are explicitly excluded
3. the dependency rewrite target is clear
4. the first externalization cut is intentionally narrow and executable

## Immediate Follow-Up

After this note lands, the next program move should be:

1. treat `M4` ownership externalization as functionally satisfied
2. move to `M5` generic artifact cutover and compatibility retirement
3. keep Julia-named outward surfaces as wrappers only while generic plugin
   artifact surfaces become canonical

:RELATIONS:
:LINKS: [[index]], [[06_roadmap/412_core_runtime_plugin_program]], [[06_roadmap/413_m2_core_extraction_package_list]], [[06_roadmap/414_m3_runtime_extraction_package_list]], [[06_roadmap/409_core_runtime_plugin_surface_inventory]], [[docs/rfcs/2026-03-27-wendao-core-runtime-plugin-migration-rfc.md]], [[.data/blueprints/wendao_arrow_plugin_core_runtime_migration.md]]
:END:

---
