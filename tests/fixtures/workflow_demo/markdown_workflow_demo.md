# Markdown Workflow Demo (Blueprint v2.4)

This demo showcases the Workflow-as-Document pattern using `:LOGBOOK:` execution drawers
to drive Qianji (千机) agent execution through `semantic_edit`.

## Overview

Blueprint v2.4 introduces the concept of "Workflow-as-Document", where:

1. **Markdown is the workflow carrier** - Tasks are defined in Markdown with property drawers
2. **Execution is tracked in :LOGBOOK:** - Agent actions are logged in real-time
3. **semantic_edit drives execution** - The edit tool can update task status and log progress

## Demo: Refactoring Authentication Module

### Initial State (Before Agent Execution)

```markdown
## Task: Refactor Authentication Logic

:PROPERTIES:
:ID: task-auth-001
:STATUS: PENDING
:WORKFLOW: auth_refactor_dag
:PRIORITY: HIGH
:END:

### Objective

Refactor the authentication module to use the new token-based system.

### Dependencies

- [[#config-module]] Configuration module must be updated first
- [[#db-schema-v2]] Database schema v2 must be deployed

### Acceptance Criteria

- [ ] All auth tests pass
- [ ] No breaking changes to public API
- [ ] Documentation updated

:LOGBOOK:
:END:
```

### During Agent Execution (Agent Updates via semantic_edit)

After the agent starts working on the task:

```markdown
## Task: Refactor Authentication Logic

:PROPERTIES:
:ID: task-auth-001
:STATUS: RUNNING
:WORKFLOW: auth_refactor_dag
:PRIORITY: HIGH
:END:

### Objective

Refactor the authentication module to use the new token-based system.

### Dependencies

- [[#config-module]] Configuration module must be updated first
- [[#db-schema-v2]] Database schema v2 must be deployed

### Acceptance Criteria

- [ ] All auth tests pass
- [ ] No breaking changes to public API
- [ ] Documentation updated

:LOGBOOK:

- [2026-03-15] Agent Started: Initiating structural audit.
- [2026-03-15] Step [audit] Found 3 files requiring updates.
- [2026-03-15] Step [analyze] Identified token validation logic in `auth.rs`.
  :END:
```

### After Completion

```markdown
## Task: Refactor Authentication Logic

:PROPERTIES:
:ID: task-auth-001
:STATUS: COMPLETED
:WORKFLOW: auth_refactor_dag
:PRIORITY: HIGH
:COMPLETED_AT: 2026-03-15
:END:

### Objective

Refactor the authentication module to use the new token-based system.

### Dependencies

- [[#config-module]] Configuration module must be updated first
- [[#db-schema-v2]] Database schema v2 must be deployed

### Acceptance Criteria

- [x] All auth tests pass
- [x] No breaking changes to public API
- [x] Documentation updated

:LOGBOOK:

- [2026-03-15] Agent Started: Initiating structural audit.
- [2026-03-15] Step [audit] Found 3 files requiring updates.
- [2026-03-15] Step [analyze] Identified token validation logic in `auth.rs`.
- [2026-03-15] Step [refactor] Updated `auth.rs` with new token system.
- [2026-03-15] Step [test] All 47 tests passed.
- [2026-03-15] Step [document] Updated API documentation.
- [2026-03-15] Agent Completed: Task finished successfully.
  :END:
```

## How semantic_edit Drives Execution

The agent uses `wendao.semantic_edit` to update the workflow document:

### 1. Update Task Status

```xml
<wendao.semantic_edit>
  <address>#task-auth-001</address>
  <operation>update_property</operation>
  <property>STATUS</property>
  <value>RUNNING</value>
</wendao.semantic_edit>
```

### 2. Append to LOGBOOK

```xml
<wendao.semantic_edit>
  <address>#task-auth-001</address>
  <operation>append_logbook</operation>
  <entry>
    <timestamp>2026-03-15</timestamp>
    <message>Step [audit] Found 3 files requiring updates.</message>
  </entry>
</wendao.semantic_edit>
```

### 3. Check Acceptance Criteria

```xml
<wendao.semantic_edit>
  <address>#task-auth-001</address>
  <operation>update_checkbox</operation>
  <item>All auth tests pass</item>
  <checked>true</checked>
</wendao.semantic_edit>
```

## LLM-Friendly Benefits

1. **Read Status Like Reading Document** - Agent can parse task status directly from Markdown
2. **Self-Healing References** - Using `[[#id@hash]]` ensures references survive content changes
3. **Audit Trail** - `:LOGBOOK:` provides complete execution history
4. **Human Readable** - Developers can review agent progress in their editor

## Integration with Agentic Navigation

Use `wendao.agentic_nav` to discover related tasks:

```xml
<wendao.agentic_nav>
  <query>authentication tasks pending</query>
  <limit>10</limit>
  <strict>true</strict>
</wendao.agentic_nav>
```

This returns structurally validated navigation candidates that the agent can use to
discover related work items and dependencies.
