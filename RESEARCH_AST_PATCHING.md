# AST-Level Parsing in Code Patching: Robustness & Scenario Resolution

## 1. Executive Summary

This document evaluates the impact of introducing **AST-level parsing** (via `tree-sitter` or `ast-grep`) into the code patching pipeline (`codex-apply-patch` and `xiuxian-edit`). We explore whether this transition can "permanently" solve recurring scenario failures such as indentation mismatches, hallucinated line numbers, and context ambiguity.

## 2. The Problem: Line-Based Fragility

Current patching tools like `codex-apply-patch` primarily rely on line-by-line heuristic matching (`seek_sequence`):
*   **Indentation Sensitivity:** LLMs frequently produce slightly incorrect leading whitespace, causing exact line matches to fail.
*   **Context Ambiguity:** Identical lines in different parts of a file lead to "ambiguous context" errors or incorrect applications.
*   **Hallucination:** Models often hallucinate line numbers, which traditional `diff` tools rely on for anchoring.

## 3. The Solution: Structural Awareness

By moving to AST-level parsing, the patching engine transitions from "Line Matching" to "Structural Matching."

### 3.1 Key Advantages
*   **Indentation Insensitivity:** AST nodes represent logical structures (functions, loops, variables) regardless of their physical whitespace formatting.
*   **Semantic Anchoring:** Instead of searching for "Line 42," the engine searches for "the body of function `validate_user`." This is inherently more robust against code shifts.
*   **Safe Transformations:** Structural editing (using `ast-grep` patterns) ensures that the resulting code remains syntactically valid, preventing common errors like unclosed braces or broken expressions.

### 3.2 Current Implementation Status
*   **`codex-apply-patch`:** Already utilizes `tree-sitter` for parsing Bash-based patch invocations, but the core patching logic (`seek_sequence`) remains line-based.
*   **`xiuxian-edit` (The Surgeon):** Implements **`StructuralEditor`**, which uses `ast-grep` for surgical code modification. This is our existing "High-Tier" structural solution.

## 4. Can it solve scenario problems "once and for all"?

While AST-level parsing is a massive leap forward, it is not a "silver bullet":

| Scenario | Line-Based | AST-Based | Resolution Status |
| :--- | :--- | :--- | :--- |
| **Indentation Mismatch** | Fails | **Succeeds** | Resolved |
| **Line Number Shift** | Fails | **Succeeds** | Resolved |
| **Ambiguous Context** | High Failure | **Low Failure** | Mostly Resolved |
| **Broken Syntax in Patch** | Fails | Fails | Unresolved (Parser Error) |
| **Complex Multi-File Refactor** | Risky | **Predictable** | Mostly Resolved |

## 5. Engineering Strategy

To maximize robustness across all scenarios, we recommend:
1.  **Unified AST Core:** Transition the core of `codex-apply-patch` from `seek_sequence` to a structural matcher powered by `omni-ast`.
2.  **Fuzzy Structural Match:** Implement "structural similarity" where the engine can apply a patch if the AST topology matches, even if some leaf nodes (variable names) differ slightly.
3.  **Validation Gates:** Use the AST parser to verify that the file remains "parseable" immediately after the patch is applied, providing instant feedback to the Agent.

## 6. Conclusion

Introducing AST-level parsing is the most effective way to eliminate the "Line-Based Tax" that causes most scenario failures. While it introduces complexity (language-specific parsers), it provides the **Structural Intelligence** necessary for reliable, autonomous code evolution.
