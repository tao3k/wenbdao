use std::time::Instant;

use super::*;

#[test]
#[ignore = "heavy benchmark; run with --ignored to validate batch-native hybrid retrieval latency"]
fn test_link_graph_hybrid_batch_latency_on_2k_fixture() -> Result<(), Box<dyn std::error::Error>> {
    let tmp = tempdir()?;
    build_hybrid_fixture(tmp.path())?;

    let build_started = Instant::now();
    let index = LinkGraphIndex::build(tmp.path())
        .map_err(|err| format!("failed to build hybrid benchmark fixture: {err}"))?;
    let build_elapsed_ms = build_started.elapsed().as_secs_f64() * 1000.0;

    let anchor_ids = collect_leaf_anchor_ids(&index)?;
    let query_batches = build_anchor_batches(&anchor_ids)?;
    let options = QuantumFusionOptions {
        alpha: 0.6,
        max_distance: MAX_DISTANCE,
        related_limit: RELATED_LIMIT,
        ppr: None,
    };

    for batch in query_batches.iter().take(WARMUP_QUERY_COUNT) {
        let _ = index.quantum_contexts_from_anchor_batch(
            batch,
            "anchor_ref",
            "semantic_score",
            &options,
        )?;
    }

    let mut query_ms = Vec::with_capacity(QUERY_COUNT);
    let mut total_contexts = 0_usize;
    for batch in &query_batches {
        let started = Instant::now();
        let contexts = index.quantum_contexts_from_anchor_batch(
            batch,
            "anchor_ref",
            "semantic_score",
            &options,
        )?;
        let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
        assert_eq!(
            contexts.len(),
            BATCH_SIZE,
            "every benchmark batch should produce one context per semantic anchor"
        );
        total_contexts += contexts.len();
        query_ms.push(elapsed_ms);
    }

    let p50_ms = percentile(&query_ms, 50);
    let p95_ms = percentile(&query_ms, 95);
    let avg_ms = query_ms.iter().sum::<f64>() / query_ms.len() as f64;
    let avg_contexts = total_contexts as f64 / QUERY_COUNT as f64;
    let target_p95_ms = env_f64("XIUXIAN_WENDAO_HYBRID_TARGET_P95_MS", DEFAULT_TARGET_P95_MS);
    let enforce_target = env_flag("XIUXIAN_WENDAO_HYBRID_ENFORCE_TARGET");

    assert!(
        p95_ms <= HARD_SANITY_P95_MS,
        "hybrid batch benchmark p95 was {:.2}ms, expected <= {:.2}ms",
        p95_ms,
        HARD_SANITY_P95_MS
    );

    if enforce_target {
        assert!(
            p95_ms <= target_p95_ms,
            "hybrid batch benchmark p95 was {:.2}ms, expected <= {:.2}ms when XIUXIAN_WENDAO_HYBRID_ENFORCE_TARGET=1",
            p95_ms,
            target_p95_ms
        );
    }

    println!(
        "link_graph_hybrid_batch_benchmark: docs={}, hubs={}, queries={}, batch_size={}, build_ms={:.2}, p50_ms={:.2}, p95_ms={:.2}, avg_ms={:.2}, avg_contexts={:.2}, max_distance={}, related_limit={}, alpha={:.2}, enforce_target={}, target_p95_ms={:.2}",
        DOC_COUNT,
        HUB_COUNT,
        QUERY_COUNT,
        BATCH_SIZE,
        build_elapsed_ms,
        p50_ms,
        p95_ms,
        avg_ms,
        avg_contexts,
        MAX_DISTANCE,
        RELATED_LIMIT,
        options.alpha,
        enforce_target,
        target_p95_ms,
    );

    Ok(())
}
