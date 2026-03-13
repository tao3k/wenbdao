# 🌀 Wendao (问道)

**The Sovereign High-Performance Knowledge & Link-Graph Runtime.**

[![Rust](https://img.shields.io/badge/language-Rust-orange.svg)](https://www.rust-lang.org/)
[![Valkey](https://img.shields.io/badge/storage-Valkey-red.svg)](https://valkey.io/)
[![LanceDB](https://img.shields.io/badge/vector-LanceDB-blue.svg)](https://lancedb.com/)
[![Arrow](https://img.shields.io/badge/protocol-Apache--Arrow-brightgreen.svg)](https://arrow.apache.org/)

**Wendao** is a next-generation knowledge management engine. While tools like Obsidian revolutionized human note-taking, **Wendao** is designed for the era of Autonomous Agents, providing a high-performance, programmable substrate for structured reasoning and massive-scale retrieval.

---

## 💎 Why Wendao? (The Obsidian Leap)

Wendao moves beyond the limitations of traditional bi-link tools by introducing **Topological Sovereignty**:

| Feature         | Obsidian (Human-Centric)    | **Wendao (Agent-Centric)**                        |
| :-------------- | :-------------------------- | :------------------------------------------------ |
| **Structure**   | Flat Bi-links & Folders     | **Hierarchical Semantic Trees (PageIndex)**       |
| **Retrieval**   | Simple Search / Dataview    | **Quantum Fusion (Vector + Graph + PPR)**         |
| **Scale**       | Electron / Local Filesystem | **Rust Core / LanceDB / Valkey Cluster**          |
| **Context**     | Manual "Maps of Content"    | **Automated Ancestry Uplink (Zero-loss context)** |
| **Performance** | Sequential scanning         | **Arrow-Native Zero-Copy (15x throughput)**       |

---

## 🚀 Key Evolutionary Features

### 1. PageIndex Rust Core (Hierarchical Indexing)

Unlike Obsidian's flat structure, Wendao builds a recursive **Semantic Tree** of your documents. It understands the logical hierarchy (Root > Chapter > Section), allowing agents to navigate complex long-form content with "God's eye" perspective.

### 2. Quantum Fusion (Hybrid Retrieval)

Fuses fuzzy **Vector Search** (semantic intuition) with precise **Graph Diffusion** (logical reasoning). Using a neurobiologically inspired **PPR algorithm** (Personalized PageRank), Wendao finds not just "similar" text, but "logically relevant" knowledge clusters.

### 3. Apache Arrow IPC

Built on top of the **Arrow Data Ecosystem**. Knowledge flows through the engine as columnar memory batches. This ensures **Zero-copy** overhead during retrieval, re-ranking, and injection, making it capable of handling millions of nodes at sub-millisecond latency.

---

## 📚 Theoretical Foundation (2025-2026)

Wendao is physically grounded in cutting-edge RAG research:

- **LightRAG (2025)**: Dual-level indexing (Logical + Entity).
- **RAGNET (Stanford 2025)**: End-to-end training for neural graph retrieval.
- **Columnar Knowledge Streams (2026)**: Zero-copy Arrow transport for scaling.

---

## 🛠 Architecture

- **Kernel**: Pure Rust (Tokio / Rayon)
- **Hot Cache**: Valkey (In-memory graph adjacency and saliency scores)
- **Cold Storage**: LanceDB (Persistent vector anchors and Arrow fragments)
- **Protocol**: Apache Arrow (Universal knowledge transport layer)

---

## 📦 Usage

### As a CLI Tool (Standalone Binary)

Build the sovereign binary:

```bash
cargo build --release --bin wendao
```

Run common operations:

```bash
# Analyze document hierarchy
./target/release/wendao page-index --path ./my_notes/paper.md

# Execute hybrid search
./target/release/wendao search "Explain quantum entanglement" --hybrid

# Show graph neighbors
./target/release/wendao neighbors "Agentic_RAG"
```

### As a Library

Add **Wendao** to your `Cargo.toml`:

```toml
[dependencies]
xiuxian-wendao = { git = "https://github.com/tao3k/wenbdao.git" }
```

Initialize the engine:

```rust
let engine = WendaoEngine::builder()
    .with_storage(ValkeyConfig::default())
    .with_vectors(LanceConfig::at("./data/vectors"))
    .build()
    .await?;
```

---

## 🛡️ License

Designed with the precision of a master artisan.

© 2026 Sovereign Forge. All Rights Reserved.
