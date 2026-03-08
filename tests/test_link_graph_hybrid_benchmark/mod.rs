//! Benchmark tests for batch-native `LinkGraph` hybrid retrieval.
//!
//! This benchmark is intentionally `ignored` by default because it builds a
//! multi-thousand-document markdown fixture and measures steady-state
//! `quantum_contexts_from_anchor_batch(...)` latency.

use std::cmp;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use tempfile::tempdir;
use xiuxian_wendao::{LinkGraphIndex, QuantumFusionOptions};

const DOC_COUNT: usize = 2_048;
const HUB_COUNT: usize = 16;
const QUERY_COUNT: usize = 32;
const WARMUP_QUERY_COUNT: usize = 6;
const BATCH_SIZE: usize = 12;
const MAX_DISTANCE: usize = 3;
const RELATED_LIMIT: usize = 6;
const HARD_SANITY_P95_MS: f64 = 500.0;
const DEFAULT_TARGET_P95_MS: f64 = 75.0;

fn write_note(path: &Path, body: &str) -> Result<(), Box<dyn std::error::Error>> {
    fs::write(path, body)?;
    Ok(())
}

fn note_id(i: usize) -> String {
    format!("note-{i:04}")
}

fn hub_id(i: usize) -> String {
    format!("hub-{i:02}")
}

fn build_hybrid_fixture(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..DOC_COUNT {
        let current = note_id(i);
        let next = note_id((i + 1) % DOC_COUNT);
        let jump = note_id((i + 97) % DOC_COUNT);
        let hub = hub_id(i % HUB_COUNT);
        let body = format!(
            "# {current}\n\n\
             synthetic root section keeps enough tokens to preserve page indexing fidelity for hybrid retrieval.\n\n\
             ## Details\n\n\
             details block {i} provides stable semantic structure and enough prose for page-index extraction across benchmark runs.\n\n\
             ### Leaf\n\n\
             leaf block {i} anchors hybrid retrieval while linking to neighboring notes and hub nodes for topology expansion.\n\n\
             [[{next}]] [[{jump}]] [[{hub}]]\n"
        );
        write_note(&root.join(format!("{current}.md")), &body)?;
    }

    for h in 0..HUB_COUNT {
        let hub = hub_id(h);
        let mut links = String::new();
        let stride = HUB_COUNT * 3;
        let mut idx = h;
        let mut emitted = 0_usize;
        while idx < DOC_COUNT && emitted < 160 {
            if !links.is_empty() {
                links.push(' ');
            }
            links.push_str("[[");
            links.push_str(&note_id(idx));
            links.push_str("]] ");
            emitted += 1;
            idx += stride;
        }
        let body = format!(
            "# {hub}\n\n\
             synthetic hub node {h} concentrates deterministic outbound links for related-cluster expansion.\n\n\
             {links}\n"
        );
        write_note(&root.join(format!("{hub}.md")), &body)?;
    }

    Ok(())
}

fn collect_leaf_anchor_ids(
    index: &LinkGraphIndex,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut anchor_ids = Vec::with_capacity(DOC_COUNT);
    for i in 0..DOC_COUNT {
        let page = index
            .page_index(&note_id(i))
            .ok_or_else(|| format!("missing page index for {}", note_id(i)))?;
        let anchor_id = page
            .first()
            .and_then(|root| root.children.first())
            .and_then(|section| section.children.first())
            .map(|leaf| leaf.node_id.clone())
            .ok_or_else(|| format!("missing leaf anchor for {}", note_id(i)))?;
        anchor_ids.push(anchor_id);
    }
    Ok(anchor_ids)
}

fn build_anchor_batches(
    anchor_ids: &[String],
) -> Result<Vec<RecordBatch>, Box<dyn std::error::Error>> {
    let schema = Arc::new(Schema::new(vec![
        Field::new("anchor_ref", DataType::Utf8, false),
        Field::new("semantic_score", DataType::Float64, false),
    ]));
    let mut batches = Vec::with_capacity(QUERY_COUNT);

    for turn in 0..QUERY_COUNT {
        let mut ids = Vec::with_capacity(BATCH_SIZE);
        let mut scores = Vec::with_capacity(BATCH_SIZE);
        for slot in 0..BATCH_SIZE {
            let index = ((turn * 37) + (slot * 17)) % anchor_ids.len();
            ids.push(anchor_ids[index].clone());
            scores.push((0.95 - (slot as f64 * 0.05)).clamp(0.2, 0.95));
        }
        batches.push(RecordBatch::try_new(
            Arc::clone(&schema),
            vec![
                Arc::new(StringArray::from(ids)),
                Arc::new(Float64Array::from(scores)),
            ],
        )?);
    }

    Ok(batches)
}

fn percentile(values: &[f64], percentile: u32) -> f64 {
    assert!(!values.is_empty(), "percentile requires at least one value");
    assert!(percentile <= 100, "percentile must be between 0 and 100");

    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let len = sorted.len();
    let percentile_usize = usize::try_from(percentile).unwrap_or(100);
    let rank = len
        .saturating_mul(percentile_usize)
        .div_ceil(100)
        .saturating_sub(1);
    sorted[cmp::min(rank, sorted.len() - 1)]
}

fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

fn env_f64(name: &str, default_value: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(default_value)
}

mod link_graph_hybrid_batch_latency_on_2k_fixture;
