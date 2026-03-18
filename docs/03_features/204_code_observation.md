# Code Observation (:OBSERVE:)

:PROPERTIES:
:ID: feat-code-observation
:PARENT: [[index]]
:TAGS: feature, ast, sgrep, non-invasive
:STATUS: STABLE
:VERSION: 2.7
:END:

## Overview

Non-Invasive Code Observation allows Wendao documents to "bind" to source code blocks using structural `ast-grep` patterns. This ensures that documentation and implementation remain semantically aligned without polluting the source code.

## Core Mechanism

- **Engine**: Powered by the internal `xiuxian-ast` library.
- **Trigger**: Property drawer attribute `:OBSERVE: lang:<language> "<pattern>"`.
- **Validation**: `wendao audit` triggers a native AST parse pass to verify pattern syntax and reachability.

## Multi-Observation Support

A single section can track multiple code entities:

```markdown
:PROPERTIES:
:OBSERVE_1: lang:rust "struct $NAME { $$$FIELDS }"
:OBSERVE_2: lang:rust "impl $NAME { $$$METHODS }"
:END:
```

## LLM-Friendly Diagnostics

If a pattern is syntactically invalid, the auditor provides granular feedback:

```xml
<issue code="ERR_INVALID_OBSERVER_SYNTAX" severity="ERROR">
  <message>Invalid pattern: Unexpected EOF</message>
  <suggestion>Ensure the sgrep pattern contains a complete structural unit.</suggestion>
</issue>
```

:RELATIONS:
:LINKS: [[01_core/101_triple_a_protocol]], [[03_features/205_semantic_auditor]]
:END:
