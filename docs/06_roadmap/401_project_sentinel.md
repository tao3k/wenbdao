# Project Sentinel: Semantic Consistency

:PROPERTIES:
:ID: wendao-sentinel
:PARENT: [[index]]
:TAGS: roadmap, auditing, sentinel
:STATUS: ACTIVE
:END:

## Current Status (v2.9)

Project Sentinel has transitioned from a vision to a functional governance kernel.

### Completed Features

- **Landed**: Native Rust Auditor (`wendao audit`).
- **Landed**: Non-invasive Code Observation via `xiuxian-ast`.
- **Landed**: Global ID Registry & Dead-link detection.
- **Landed**: Fuzzy Pattern Suggestion (v2.9).
  - If an `:OBSERVE:` pattern fails validation, the auditor searches for renamed or moved symbols.
  - Pattern skeleton extraction for structural similarity scoring
  - `xiuxian-ast` for source code scanning
  - Confidence threshold filtering (0.65 by default, configurable)
  - Generates replacement drawer content for batch application
- **Landed**: Integration tests pass, all functionality works correctly.
  - Unit tests for `fuzzy_suggest` module
  - Tests for pattern skeleton extraction, similarity scoring, suggestion finding
  - Tests for `semantic_check` integration
  - XML output with `<fuzzy_suggestion>` elements
- **Landed**: Configurable Confidence Thresholds (v2.9).
  - `fuzzy_confidence_threshold` parameter in `wendao.semantic_check`
  - Range: 0.0 - 1.0 (default: 0.65)
  - Lower values suggest more matches, higher values are more strict
- **Landed**: Audit Bridge for Batch Fixes (v2.9).
  - `audit_bridge.rs` module for qianji integration
  - `BatchFix` struct for batch repair operations
  - `AuditBridge` trait for extensibility
  - `generate_batch_fixes()` function for external tool integration

### Architecture (v2.9)

```text
┌─────────────────────────────────────────────────────────────────────┐
│                        Semantic Auditor                              │
├─────────────────────────────────────────────────────────────────────┤
│  ┌──────────────┐   ┌──────────────────┐   ┌────────────────────┐  │
│  │ Code         │   │ Fuzzy Suggester  │   │ Audit Bridge       │  │
│  │ Observations │──▶│ (fuzzy_suggest)  │──▶│ (audit_bridge)     │  │
│  │ (validate)   │   │                  │   │                    │  │
│  └──────────────┘   └──────────────────┘   └────────────────────┘  │
│         │                    │                        │             │
│         ▼                    ▼                        ▼             │
│  ┌──────────────┐   ┌──────────────────┐   ┌────────────────────┐  │
│  │ SemanticIssue│   │ FuzzySuggestion  │   │ BatchFix           │  │
│  │ (error/warn) │   │ (suggestion)     │   │ (for qianji)       │  │
│  └──────────────┘   └──────────────────┘   └────────────────────┘  │
└─────────────────────────────────────────────────────────────────────┘
```

### Usage

#### Basic Semantic Check

```python
result = wendao.semantic_check(
    doc="api-docs",  # Optional: check specific doc
    checks=["code_observations", "dead_links"],
    include_warnings=True,
)
```

#### With Fuzzy Pattern Suggestions

```python
result = wendao.semantic_check(
    doc="api-docs",
    source_paths=["src/lib.rs", "src/api/"],  # Scan for renamed symbols
    fuzzy_confidence_threshold=0.65,  # Optional: adjust threshold
)
# If patterns fail, suggestions are included in XML output:
# <fuzzy_suggestion>
#   <pattern>fn process_records($$$)</pattern>
#   <confidence>0.69</confidence>
#   <source_location>src/lib.rs:42</source_location>
#   <replacement_drawer>:OBSERVE: lang:rust "fn process_records($$$)"</replacement_drawer>
# </fuzzy_suggestion>
```

#### Batch Fix Generation

```python
# Get issues with fuzzy suggestions
issues = wendao.semantic_check(doc="api-docs", source_paths=["src/"])

# Generate batch fixes for qianji
fixes = audit.generate_batch_fixes(issues)
for fix in fixes:
    print(f"{fix.doc_path}:{fix.line_number}")
    print(f"  Replace: {fix.original_content}")
    print(f"  With: {fix.replacement}")
    print(f"  Confidence: {fix.confidence}")
```

## Future Evolution: v3.0 (Planned)

These features will enhance the auditing capabilities further:

1. **Better Source File Discovery**: Integrate with existing `dependency_indexer` or scan project root directory automatically
2. **Performance Caching**: Add caching layer for scan results to improve performance on large codebases
3. **Enhanced Snapshot Tests**: Add more snapshot tests for fuzzy suggestion edge cases
4. **Code Observation Parser**: Add `extract_skeleton()` helper method to CodeObservation for testing
5. **Multi-language Pattern Libraries**: Pre-built patterns for common frameworks and libraries
