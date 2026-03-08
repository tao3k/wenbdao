use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::cmp;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub(crate) use tempfile::tempdir;
pub(crate) use xiuxian_wendao::{LinkGraphIndex, QuantumFusionOptions};

pub(crate) const DOC_COUNT: usize = 2_048;
pub(crate) const HUB_COUNT: usize = 64;
pub(crate) const QUERY_COUNT: usize = 32;
pub(crate) const WARMUP_QUERY_COUNT: usize = 6;
pub(crate) const BATCH_SIZE: usize = 24;
pub(crate) const MAX_DISTANCE: usize = 2;
pub(crate) const RELATED_LIMIT: usize = 12;
pub(crate) const HARD_SANITY_P95_MS: f64 = 750.0;
pub(crate) const DEFAULT_TARGET_P95_MS: f64 = 35.0;

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

pub(crate) fn build_hybrid_fixture(root: &Path) -> Result<(), Box<dyn std::error::Error>> {
    for i in 0..DOC_COUNT {
        let current = note_id(i);
        let next = note_id((i + 1) % DOC_COUNT);
        let jump = note_id((i + 37) % DOC_COUNT);
        let hub = hub_id(i % HUB_COUNT);
        let body = format!(
            "# {current}\n\nSynthetic hybrid benchmark note {i}.\n\n## Summary\n\nThis section anchors the semantic page index for {current}.\n\n## Details\n\nLinks: [[{next}]] [[{jump}]] [[{hub}]]\n\nNeedle terms: architecture graph engine batch retrieval.\n"
        );
        write_note(&root.join(format!("{current}.md")), &body)?;
    }

    for h in 0..HUB_COUNT {
        let hub = hub_id(h);
        let mut links = String::new();
        let stride = HUB_COUNT;
        let mut idx = h;
        let mut emitted = 0_usize;
        while idx < DOC_COUNT && emitted < 96 {
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
            "# {hub}\n\nSynthetic hybrid hub {h}.\n\n## Summary\n\nHub summary for hybrid retrieval.\n\n## Links\n\n{links}\n"
        );
        write_note(&root.join(format!("{hub}.md")), &body)?;
    }

    Ok(())
}

pub(crate) fn collect_leaf_anchor_ids(
    index: &LinkGraphIndex,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let anchor_ids = index
        .semantic_documents()
        .iter()
        .filter(|document| document.anchor_id.contains('#'))
        .map(|document| document.anchor_id.clone())
        .collect::<Vec<_>>();
    if anchor_ids.is_empty() {
        return Err("hybrid benchmark fixture produced no leaf anchors".into());
    }
    Ok(anchor_ids)
}

pub(crate) fn build_anchor_batches(
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

pub(crate) fn percentile(values: &[f64], percentile: u32) -> f64 {
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

pub(crate) fn env_flag(name: &str) -> bool {
    matches!(
        std::env::var(name).ok().as_deref(),
        Some("1" | "true" | "TRUE" | "yes" | "YES")
    )
}

pub(crate) fn env_f64(name: &str, default_value: f64) -> f64 {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<f64>().ok())
        .filter(|value| *value > 0.0)
        .unwrap_or(default_value)
}
