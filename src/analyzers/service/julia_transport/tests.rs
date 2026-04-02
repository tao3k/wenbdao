#![cfg(test)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde_json::json;
use tokio::net::TcpStream;
use tokio::time::sleep;

use super::*;
use crate::analyzers::config::{
    RegisteredRepository, RepositoryPluginConfig, RepositoryRefreshPolicy,
};
use crate::analyzers::service::julia_transport_tests::fixtures::{
    invalid_response_missing_analyzer_score_batch, request_batch, response_batch,
    response_batch_with_trace_ids,
};

#[test]
fn build_plugin_arrow_request_batch_uses_contract_columns() {
    let batch = build_plugin_arrow_request_batch(
        &[
            PluginArrowRequestRow {
                doc_id: "doc-1".to_string(),
                vector_score: 0.3,
                embedding: vec![1.0, 2.0, 3.0],
            },
            PluginArrowRequestRow {
                doc_id: "doc-2".to_string(),
                vector_score: 0.4,
                embedding: vec![4.0, 5.0, 6.0],
            },
        ],
        &[9.0, 8.0, 7.0],
    )
    .expect("request batch should build");

    assert_eq!(batch.num_rows(), 2);
    assert_eq!(batch.schema().field(0).name(), JULIA_ARROW_DOC_ID_COLUMN);
    assert_eq!(
        batch.schema().field(1).name(),
        JULIA_ARROW_VECTOR_SCORE_COLUMN
    );
    assert_eq!(batch.schema().field(2).name(), JULIA_ARROW_EMBEDDING_COLUMN);
    assert_eq!(
        batch.schema().field(3).name(),
        JULIA_ARROW_QUERY_EMBEDDING_COLUMN
    );
}

#[test]
fn julia_arrow_request_schema_uses_contract_columns() {
    let schema = julia_arrow_request_schema(3);

    assert_eq!(schema.field(0).name(), JULIA_ARROW_DOC_ID_COLUMN);
    assert_eq!(schema.field(1).name(), JULIA_ARROW_VECTOR_SCORE_COLUMN);
    assert_eq!(schema.field(2).name(), JULIA_ARROW_EMBEDDING_COLUMN);
    assert_eq!(schema.field(3).name(), JULIA_ARROW_QUERY_EMBEDDING_COLUMN);
}

#[test]
fn julia_arrow_response_schema_optionally_includes_trace_id() {
    let base = julia_arrow_response_schema(false);
    let traced = julia_arrow_response_schema(true);

    assert_eq!(base.fields().len(), 3);
    assert_eq!(traced.fields().len(), 4);
    assert_eq!(traced.field(3).name(), JULIA_ARROW_TRACE_ID_COLUMN);
}

#[test]
fn build_plugin_arrow_request_batch_rejects_dimension_mismatch() {
    let error = build_plugin_arrow_request_batch(
        &[PluginArrowRequestRow {
            doc_id: "doc-1".to_string(),
            vector_score: 0.3,
            embedding: vec![1.0, 2.0],
        }],
        &[9.0, 8.0, 7.0],
    )
    .expect_err("dimension mismatch should fail");

    assert!(
        error.to_string().contains("embedding dimension mismatch"),
        "unexpected error: {error}"
    );
}

#[test]
fn decode_plugin_arrow_score_rows_materializes_doc_scores() {
    let rows = decode_plugin_arrow_score_rows(&[response_batch()]).expect("decode should work");

    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.get("doc-1"),
        Some(&PluginArrowScoreRow {
            doc_id: "doc-1".to_string(),
            analyzer_score: 0.9,
            final_score: 0.95,
            trace_id: None,
        })
    );
    assert_eq!(
        rows.get("doc-2"),
        Some(&PluginArrowScoreRow {
            doc_id: "doc-2".to_string(),
            analyzer_score: 0.7,
            final_score: 0.8,
            trace_id: None,
        })
    );
}

#[test]
fn decode_plugin_arrow_score_rows_materializes_optional_trace_id() {
    let rows = decode_plugin_arrow_score_rows(&[response_batch_with_trace_ids()])
        .expect("decode should work");

    assert_eq!(
        rows.get("doc-1").and_then(|row| row.trace_id.as_deref()),
        Some("trace-123")
    );
    assert_eq!(
        rows.get("doc-2").and_then(|row| row.trace_id.as_deref()),
        Some("trace-123")
    );
}

#[test]
fn decode_plugin_arrow_score_rows_rejects_missing_columns() {
    let batch = invalid_response_missing_analyzer_score_batch();

    let error = decode_plugin_arrow_score_rows(&[batch]).expect_err("decode should fail");
    assert!(
        error
            .to_string()
            .contains("missing required Float64 column `analyzer_score`"),
        "unexpected error: {error}"
    );
}

#[tokio::test]
async fn fetch_plugin_arrow_score_rows_for_repository_roundtrips_remote_scores() {
    let port = reserve_test_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let _service = spawn_real_wendaoarrow_service(port);
    wait_for_service_ready(&base_url)
        .await
        .expect("real WendaoArrow Flight service should become ready");
    let repository = RegisteredRepository {
        id: "demo".to_string(),
        path: None,
        url: None,
        git_ref: None,
        refresh: RepositoryRefreshPolicy::Fetch,
        plugins: vec![RepositoryPluginConfig::Config {
            id: "julia".to_string(),
            options: json!({
                "flight_transport": {
                    "base_url": base_url,
                    "route": "/rerank",
                    "schema_version": "v1"
                }
            }),
        }],
    };

    let rows = fetch_plugin_arrow_score_rows_for_repository(&repository, &[request_batch()])
        .await
        .expect("transport should succeed");

    assert_eq!(rows.len(), 2);
    assert_eq!(rows.get("doc-1").map(|row| row.analyzer_score), Some(0.3));
    assert_eq!(rows.get("doc-1").map(|row| row.final_score), Some(0.3));
}

struct ChildGuard {
    child: Child,
}

impl ChildGuard {
    fn new(child: Child) -> Self {
        Self { child }
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Ok(None) = self.child.try_wait() {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn reserve_test_port() -> u16 {
    std::net::TcpListener::bind("127.0.0.1:0")
        .and_then(|listener| listener.local_addr())
        .map(|address| address.port())
        .unwrap_or_else(|error| panic!("reserve test port: {error}"))
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../../")
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve repo root: {error}"))
}

fn wendaoarrow_script(name: &str) -> PathBuf {
    repo_root()
        .join(".data/WendaoArrow/scripts")
        .join(name)
        .canonicalize()
        .unwrap_or_else(|error| panic!("resolve WendaoArrow script `{name}`: {error}"))
}

fn spawn_real_wendaoarrow_service(port: u16) -> ChildGuard {
    let script = wendaoarrow_script("run_stream_scoring_flight_server.sh");
    let child = Command::new("bash")
        .arg(script)
        .arg("--port")
        .arg(port.to_string())
        .current_dir(repo_root())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap_or_else(|error| panic!("spawn real WendaoArrow service: {error}"));
    ChildGuard::new(child)
}

async fn wait_for_service_ready(base_url: &str) -> Result<(), String> {
    let socket_addr = base_url
        .strip_prefix("http://")
        .or_else(|| base_url.strip_prefix("https://"))
        .unwrap_or(base_url)
        .to_string();

    for _ in 0..150 {
        if TcpStream::connect(&socket_addr).await.is_ok() {
            return Ok(());
        }
        sleep(Duration::from_millis(200)).await;
    }

    Err("real Julia Flight service did not become ready in time".to_string())
}
