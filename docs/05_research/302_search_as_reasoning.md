# Search-as-Reasoning: Autonomous Search in Structured State Spaces
:PROPERTIES:
:ID:       paper-2025-search-reasoning
:PARENT:   [[301_research_papers]]
:TAGS:     research, agentic-search, state-transition, graph-pruning
:STATUS:   HARDENING
:END:

## 🧠 Core Theory: Search as State-Transition
Research (OrcaLoca 2025) formalizes repository exploration as a state-transition function within a **CodeGraph** $G=(V, E)$. This moves RAG from "One-Shot" to "Iterative Convergence".

## 🔬 Technical Breakdown (OrcaLoca Implementation)

### 1. Action Scheduler Queue (ASQ)
Instead of LLM calling tools linearly, actions are placed in a **Priority Queue**.
*   **Re-ranking**: The system dynamically reorders the queue based on LLM-guided urgency and structural relevance.
*   **Decomposition**: High-level actions (e.g., `view_file`) are decomposed into atomic sub-actions (e.g., `view_method`).

### 2. UID Structural Encoding
Nodes are identified via a hierarchical string: `path/to/file::class::method`.
*   **Wendao Alignment**: This matches our `structural_path` but adds cross-reference edges (calls/definitions).

### 3. Distance-Aware Context Pruning
To prevent reasoning degradation, the "searched context" is pruned using a **Graph Distance Heuristic**.
*   **Threshold**: Only nodes within $k$-hops of the current "Suspicious Node" are retained in the prompt.

## 🛠️ Wendao Implementation Blueprint
- [ ] **Feature**: Implement `wendao.agentic_nav` using a Priority Queue for path exploration.
- [ ] **Feature**: Integrate `Graph-Distance Pruning` in the LinkGraph engine to automatically trim context based on topological distance.

---
:RELATIONS:
:LINKS:    [[06_roadmap/401_project_sentinel]], [[01_core/101_triple_a_protocol]]
:ASSETS:   [[.data/research/papers/2025_orcaloca_agentic_search.pdf]]
:END:
