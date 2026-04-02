use std::fs;
use std::process::{Command, Stdio};

use super::wendaoarrow_common::{
    WendaoArrowServiceGuard, repo_root, reserve_test_port, wait_for_health,
    wendaoarrow_package_dir, wendaoarrow_script,
};

pub(crate) struct WendaoArrowScoreRow<'a> {
    pub(crate) doc_id: &'a str,
    pub(crate) analyzer_score: f64,
    pub(crate) final_score: f64,
}

pub(crate) async fn spawn_wendaoarrow_custom_scoring_service(
    rows: &[WendaoArrowScoreRow<'_>],
) -> (String, WendaoArrowServiceGuard) {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let package_dir = wendaoarrow_package_dir();
    let generated_dir = package_dir.join("generated");
    fs::create_dir_all(&generated_dir)
        .unwrap_or_else(|error| panic!("create generated WendaoArrow example dir: {error}"));
    let generated_relative_path = format!("generated/custom_scoring_flight_server_{port}.jl");
    let generated_script = package_dir.join(&generated_relative_path);
    fs::write(&generated_script, processor_script(rows)).unwrap_or_else(|error| {
        panic!("write generated WendaoArrow custom scoring script: {error}")
    });

    let child = Command::new("julia")
        .arg(wendaoarrow_script("run_flight_example.jl"))
        .arg(generated_relative_path)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn WendaoArrow custom scoring service: {error}"));
    let guard = WendaoArrowServiceGuard::new(child);

    wait_for_health(base_url.as_str()).await;
    (base_url, guard)
}

fn processor_script(rows: &[WendaoArrowScoreRow<'_>]) -> String {
    let mut mappings = String::new();
    for row in rows {
        mappings.push_str(
            format!(
                "\"{}\" => ({}, {}),\n",
                row.doc_id, row.analyzer_score, row.final_score
            )
            .as_str(),
        );
    }

    format!(
        r#"
using WendaoArrow
using gRPCServer
using Tables

const SCORE_MAP = Dict(
{mappings}
)

function processor(stream)
    doc_ids = String[]
    analyzer_scores = Float64[]
    final_scores = Float64[]
    seen_doc_ids = Dict{{String, Int}}()
    row_offset = 0

    for batch in stream
        WendaoArrow.require_columns(
            batch,
            ("doc_id", "vector_score");
            subject = "custom Julia rerank request",
        )
        row_count = WendaoArrow.require_column_lengths(
            batch,
            ("doc_id", "vector_score");
            subject = "custom Julia rerank request",
        )
        WendaoArrow.require_unique_string_column(
            batch,
            "doc_id";
            subject = "custom Julia rerank request",
            seen = seen_doc_ids,
            row_offset = row_offset,
        )

        columns = Tables.columntable(batch)
        sizehint!(doc_ids, length(doc_ids) + row_count)
        sizehint!(analyzer_scores, length(analyzer_scores) + row_count)
        sizehint!(final_scores, length(final_scores) + row_count)

        for (row_index, (raw_doc_id, raw_vector_score)) in enumerate(zip(columns.doc_id, columns.vector_score))
            doc_id = WendaoArrow.coerce_string(
                raw_doc_id;
                column = "doc_id",
                subject = "custom Julia rerank request",
                row_index = row_index,
            )
            WendaoArrow.coerce_float64(
                raw_vector_score;
                column = "vector_score",
                subject = "custom Julia rerank request",
                row_index = row_index,
            )
            analyzer_score, final_score = get(SCORE_MAP, doc_id, (0.0, 0.0))
            push!(doc_ids, doc_id)
            push!(analyzer_scores, analyzer_score)
            push!(final_scores, final_score)
        end

        row_offset += row_count
    end

    return WendaoArrow.normalize_scoring_response((
        doc_id = doc_ids,
        analyzer_score = analyzer_scores,
        final_score = final_scores,
    ); subject = "custom Julia rerank response")
end

config = WendaoArrow.config_from_args(ARGS)

WendaoArrow.serve_stream_flight(
    processor;
    descriptor = WendaoArrow.flight_descriptor(("rerank",)),
    host=config.host,
    port=config.port,
)
"#
    )
}
