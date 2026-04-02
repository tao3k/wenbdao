//! Criterion microbenchmarks for xiuxian-wendao performance trend analysis.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};

use criterion::{Criterion, Throughput, black_box};
use tempfile::{TempDir, tempdir};
use xiuxian_wendao::{
    LinkGraphHit, LinkGraphIndex, LinkGraphPprSubgraphMode, LinkGraphRelatedPprOptions,
    narrate_subgraph,
};

const NODE_COUNT: usize = 2_048;
const HUB_COUNT: usize = 32;
const RELATED_MAX_DISTANCE: usize = 4;
const RELATED_LIMIT: usize = 24;

fn note_id(i: usize) -> String {
    format!("note-{i:05}")
}

fn hub_id(i: usize) -> String {
    format!("hub-{i:03}")
}

fn write_note(path: &Path, body: &str) {
    if let Err(error) = fs::write(path, body) {
        panic!("write benchmark fixture note {}: {error}", path.display());
    }
}

fn build_fixture(root: &Path) {
    for i in 0..NODE_COUNT {
        let current = note_id(i);
        let next = note_id((i + 1) % NODE_COUNT);
        let jump = note_id((i + 97) % NODE_COUNT);
        let hub = hub_id(i % HUB_COUNT);
        let body = format!(
            "# {current}\n\nSynthetic benchmark note {i}.\n\nLinks: [[{next}]] [[{jump}]] [[{hub}]]\n"
        );
        write_note(&root.join(format!("{current}.md")), &body);
    }

    for i in 0..HUB_COUNT {
        let hub = hub_id(i);
        let mut links = String::new();
        let stride = HUB_COUNT * 2;
        let mut cursor = i;
        let mut emitted = 0_usize;
        while cursor < NODE_COUNT && emitted < 160 {
            if !links.is_empty() {
                links.push(' ');
            }
            links.push_str("[[");
            links.push_str(&note_id(cursor));
            links.push_str("]]");
            emitted += 1;
            cursor += stride;
        }
        let body = format!("# {hub}\n\nSynthetic benchmark hub {i}.\n\nOutbound links: {links}\n");
        write_note(&root.join(format!("{hub}.md")), &body);
    }
}

fn build_index_fixture() -> (TempDir, LinkGraphIndex, Vec<String>) {
    let tmp = match tempdir() {
        Ok(tmp) => tmp,
        Err(error) => panic!("create benchmark tempdir: {error}"),
    };
    build_fixture(tmp.path());
    let index = match LinkGraphIndex::build(tmp.path()) {
        Ok(index) => index,
        Err(error) => panic!("build benchmark index: {error}"),
    };
    let seeds = (0..192)
        .map(|turn| note_id((turn * 211) % NODE_COUNT))
        .collect();
    (tmp, index, seeds)
}

fn ppr_options() -> LinkGraphRelatedPprOptions {
    LinkGraphRelatedPprOptions {
        alpha: Some(0.9),
        max_iter: Some(30),
        tol: Some(1e-6),
        subgraph_mode: Some(LinkGraphPprSubgraphMode::Auto),
    }
}

fn bench_related_ppr(c: &mut Criterion) {
    let (_tmp, index, seeds) = build_index_fixture();
    let ppr = ppr_options();
    let cursor = AtomicUsize::new(0);

    let mut group = c.benchmark_group("search_related_ppr");
    group.throughput(Throughput::Elements(1));
    group.bench_function("related_with_diagnostics", |bench| {
        bench.iter(|| {
            let position = cursor.fetch_add(1, Ordering::Relaxed) % seeds.len();
            let seed = &seeds[position];
            let (rows, diagnostics) = index.related_with_diagnostics(
                black_box(seed),
                RELATED_MAX_DISTANCE,
                RELATED_LIMIT,
                Some(&ppr),
            );
            assert!(
                !(rows.is_empty() || diagnostics.is_none()),
                "benchmark fixture produced empty or diagnostics-free result"
            );
            black_box(rows.len())
        });
    });
    group.finish();
}

fn bench_narration_fusion(c: &mut Criterion) {
    let hits: Vec<LinkGraphHit> = (0..240)
        .map(|i| {
            let i_u32 = u32::try_from(i).unwrap_or(u32::MAX);
            LinkGraphHit {
                stem: format!("node_{i:04}"),
                score: 1.0 - (f64::from(i_u32) * 0.002),
                title: format!("Narration benchmark node {i}"),
                path: format!("docs/node_{i:04}.md"),
                doc_type: None,
                tags: vec!["benchmark".to_string()],
                best_section: None,
                match_reason: None,
            }
        })
        .collect();

    let mut group = c.benchmark_group("fusion_narration");
    group.throughput(Throughput::Elements(
        u64::try_from(hits.len()).unwrap_or(u64::MAX),
    ));
    group.bench_function("narrate_subgraph_240_hits", |bench| {
        bench.iter(|| black_box(narrate_subgraph(black_box(&hits))));
    });
    group.finish();
}

fn main() {
    let mut criterion = Criterion::default().configure_from_args();
    bench_related_ppr(&mut criterion);
    bench_narration_fusion(&mut criterion);
    criterion.final_summary();
}
