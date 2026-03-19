# Wendao DocOS Kernel: Map of Content

:PROPERTIES:
:ID: wendao-moc
:TYPE: INDEX
:STATUS: ACTIVE
:END:

Standardized documentation repository for the Wendao DocOS Kernel, leveraging AST-based identity and structured properties.

## 📁 01_core: Architecture & Foundation

:PROPERTIES:
:ID: core-foundation
:OBSERVE: lang:rust "pub enum ThisDoesNotExistAnywhere { $$$ }"
:CONTRACT: must_contain("Id", "Path", "Hash")
:END:

- [[01_core/101_triple_a_protocol]]: Identity-based addressing.
- [[01_core/102_atomic_mutation]]: Byte-level modification safety.

## 📁 03_features: Functional Ledger

:PROPERTIES:
:ID: functional-ledger
:OBSERVE: lang:rust "pub struct LinkGraphIndex { $$$ }"
:END:

- [[03_features/201_property_drawers]]: Metadata management.
- [[03_features/202_block_addressing]]: Paragraph-level granularity.
- [[03_features/203_agentic_navigation]]: Reasoning-driven discovery.
- [[03_features/204_code_observation]]: Non-invasive sgrep binding.
- [[03_features/205_semantic_auditor]]: Native sentinel engine.
- [[03_features/206_openai_semantic_ignition]]: OpenAI-compatible query ignition bridge.
- [[03_features/207_gateway_openapi_contract_surface]]: Stable gateway OpenAPI contract surface for `rest_docs`.

## 📁 05_research: Theoretical Hardening

- [[05_research/301_research_papers]]: Academic foundations.

## 📁 06_roadmap: Future Evolution

:PROPERTIES:
:ID: roadmap-sentinel
:OBSERVE: lang:rust "pub trait AuditBridge { $$$ }"
:CONTRACT: must_contain("generate_fixes", "apply_fixes")
:END:

- [[06_roadmap/401_project_sentinel]]: Project Sentinel (Auditing).

:RELATIONS:
:LINKS: [[01_core/101_triple_a_protocol]], [[06_roadmap/401_project_sentinel]]
:END:

---

:FOOTER:
:STANDARDS: v2.0
:LAST_SYNC: 2026-03-18
:END:
