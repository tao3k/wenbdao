# Project Sentinel v2: Neural FS Observation & Propagation

## 1. Objective

To maintain real-time semantic alignment between `src/` (Source of Truth) and `docs/` (Knowledge Representation).

## 2. Technical Architecture

### 2.1 FS Observation (`notify-8.2`)

- **Debouncing**: 1000ms stable window (Configurable via `SentinelConfig`).
- **Noise Suppression**: Ignores `mod.rs`, `lib.rs`, and generated build artifacts.

### 2.2 Phase 6: Semantic Change Propagation

- **O(1) Symbol Lookup**: Uses a `symbol_to_docs` inverted index mapping extracted symbols (fn, struct, class) to documentation nodes.
- **Drift Detection**: When a symbol in `src/` changes, Sentinel instantly generates a `SemanticDriftSignal` with varying confidence levels (High/Medium/Low).

### 2.3 CAS Consistency Protection

- **verify_file_stable()**: Ensures analysis only occurs on stable disk states, preventing partial-write analysis during IDE saves.
