# Semantic Auditor (wendao audit)

:PROPERTIES:
:ID: feat-semantic-auditor
:PARENT: [[index]]
:TAGS: feature, auditing, sentinel, integrity
:STATUS: STABLE
:VERSION: 2.8
:END:

## Overview

The Semantic Auditor is the core component of Project Sentinel. It performs deep structural and relational analysis across the entire knowledge base to identify "semantic rot".

## Check Categories

The auditor executes a multi-pass diagnostic flow:

1. **DeadLinks**: Verifies all `[[id]]` references.
2. **DeprecatedRefs**: Flags usage of nodes marked as `:STATUS: DEPRECATED`.
3. **IdCollisions**: Ensures global uniqueness of all manual `:ID:` entries.
4. **CodeObservations**: Validates `ast-grep` patterns against `xiuxian-ast`.
5. **HashAlignment**: Checks for content drift using content fingerprints.

## Diagnostic Protocol (XML)

Designed for machine processing, the XML output provides exact byte ranges for automated remediation.

:RELATIONS:
:LINKS: [[06_roadmap/401_project_sentinel]], [[03_features/204_code_observation]]
:END:
