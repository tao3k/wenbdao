# Agentic Navigation (wendao.agentic_nav)

:PROPERTIES:
:ID: feat-agentic-nav
:PARENT: [[index]]
:TAGS: feature, search, discovery, agentic
:STATUS: STABLE
:VERSION: 2.4
:END:

## Overview

`wendao.agentic_nav` is a reasoning-driven discovery tool that acts as a "Structured GPS" for Agents. It bridges the gap between neural vector search and symbolic AST validation.

## Core Capabilities

1. **Skeleton Re-ranking**: Automatically prioritizes search hits that match the document's structural skeleton.
2. **Navigation Hints**: Returns a `<navigation_hint>` for each candidate, explaining its structural role (e.g., "Top-level section", "Deeply nested implementation details").
3. **Identity Verification**: Checks if the target `:ID:` is still valid in the live AST before returning it.

## Output Schema (LLM-Native)

The tool produces a structured XML response optimized for Agent parsing:

```xml
<agentic_nav_result>
  <query>refactor storage</query>
  <candidates>
    <candidate>
      <doc_id>README.md</doc_id>
      <anchor_id>#arch-v1</anchor_id>
      <navigation_hint>Top-level section - good entry point.</navigation_hint>
      <structural_path>
        <segment>Architecture</segment>
        <segment>Storage</segment>
      </structural_path>
    </candidate>
  </candidates>
</agentic_nav_result>
```

:RELATIONS:
:LINKS: [[01_core/101_triple_a_protocol]], [[05_research/302_search_as_reasoning]]
:END:
