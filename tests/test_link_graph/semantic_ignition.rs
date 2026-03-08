use super::quantum_fixture_support::{
    assert_quantum_fixture, build_hybrid_fixture, contexts_snapshot,
};
use serde_json::json;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use xiuxian_wendao::{
    QuantumAnchorHit, QuantumFusionOptions, QuantumSemanticIgnition, QuantumSemanticIgnitionError,
    QuantumSemanticIgnitionFuture, QuantumSemanticSearchRequest,
};

#[derive(Debug, Clone, PartialEq, Eq)]
struct StubIgnitionError(&'static str);

impl Display for StubIgnitionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl Error for StubIgnitionError {}

struct AssertingIgnition {
    anchor_id: String,
    calls: Arc<AtomicUsize>,
}

impl QuantumSemanticIgnition for AssertingIgnition {
    type Error = StubIgnitionError;

    fn backend_name(&self) -> &'static str {
        "stub-semantic"
    }

    fn search_anchors<'a>(
        &'a self,
        request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(self.backend_name(), "stub-semantic");
        assert_eq!(request.query_text, Some("leaf topic"));
        assert_eq!(request.query_vector, [0.1, 0.2, 0.3].as_slice());
        assert_eq!(request.limit, 1);
        assert_eq!(request.min_vector_score, None);

        let anchor_id = self.anchor_id.clone();
        Box::pin(async move {
            Ok(vec![QuantumAnchorHit {
                anchor_id,
                vector_score: 0.75,
            }])
        })
    }
}

struct ErrorIgnition;

impl QuantumSemanticIgnition for ErrorIgnition {
    type Error = StubIgnitionError;

    fn backend_name(&self) -> &'static str {
        "error-semantic"
    }

    fn search_anchors<'a>(
        &'a self,
        _request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        Box::pin(async { Err(StubIgnitionError("semantic backend failed")) })
    }
}

struct CountingIgnition {
    calls: Arc<AtomicUsize>,
}

impl QuantumSemanticIgnition for CountingIgnition {
    type Error = StubIgnitionError;

    fn backend_name(&self) -> &'static str {
        "counting-semantic"
    }

    fn search_anchors<'a>(
        &'a self,
        _request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Ok(Vec::new()) })
    }
}

struct StaticAnchorsIgnition {
    anchors: Vec<QuantumAnchorHit>,
}

impl QuantumSemanticIgnition for StaticAnchorsIgnition {
    type Error = StubIgnitionError;

    fn backend_name(&self) -> &'static str {
        "static-anchors"
    }

    fn search_anchors<'a>(
        &'a self,
        _request: QuantumSemanticSearchRequest<'a>,
    ) -> QuantumSemanticIgnitionFuture<'a, Self::Error> {
        let anchors = self.anchors.clone();
        Box::pin(async move { Ok(anchors) })
    }
}

#[tokio::test]
async fn test_quantum_contexts_from_semantic_ignition_delegates_and_recovers_trace()
-> Result<(), Box<dyn Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let ignition = AssertingIgnition {
        anchor_id,
        calls: Arc::clone(&calls),
    };

    let contexts = fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ignition,
            QuantumSemanticSearchRequest {
                query_text: Some("  leaf topic  "),
                query_vector: &[0.1, 0.2, 0.3],
                limit: 0,
                min_vector_score: None,
            },
            &QuantumFusionOptions {
                alpha: 0.7,
                max_distance: 2,
                related_limit: 2,
                ppr: None,
            },
        )
        .await?;

    let actual = json!({
        "calls": calls.load(Ordering::SeqCst),
        "result": contexts_snapshot(&contexts),
    });
    assert_quantum_fixture(
        "semantic_ignition/delegates_and_recovers_trace.json",
        &actual,
    );
    Ok(())
}

#[tokio::test]
async fn test_quantum_contexts_from_semantic_ignition_skips_empty_requests()
-> Result<(), Box<dyn Error>> {
    let fixture = build_hybrid_fixture()?;
    let calls = Arc::new(AtomicUsize::new(0));
    let ignition = CountingIgnition {
        calls: Arc::clone(&calls),
    };

    let contexts = fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ignition,
            QuantumSemanticSearchRequest {
                query_text: Some("   "),
                query_vector: &[],
                limit: 0,
                min_vector_score: None,
            },
            &QuantumFusionOptions::default(),
        )
        .await?;

    let actual = json!({
        "calls": calls.load(Ordering::SeqCst),
        "result": contexts_snapshot(&contexts),
    });
    assert_quantum_fixture("semantic_ignition/skips_empty_requests.json", &actual);
    Ok(())
}

#[tokio::test]
async fn test_quantum_contexts_from_semantic_ignition_respects_min_vector_score()
-> Result<(), Box<dyn Error>> {
    let fixture = build_hybrid_fixture()?;
    let anchor_id = fixture.alpha_leaf_anchor_id()?;
    let ignition = StaticAnchorsIgnition {
        anchors: vec![
            QuantumAnchorHit {
                anchor_id: anchor_id.clone(),
                vector_score: 0.82,
            },
            QuantumAnchorHit {
                anchor_id: "docs/beta".to_string(),
                vector_score: 0.31,
            },
        ],
    };

    let contexts = fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ignition,
            QuantumSemanticSearchRequest {
                query_text: Some("leaf topic"),
                query_vector: &[0.9, 0.1, 0.0],
                limit: 2,
                min_vector_score: Some(0.5),
            },
            &QuantumFusionOptions {
                alpha: 0.7,
                max_distance: 2,
                related_limit: 2,
                ppr: None,
            },
        )
        .await?;

    let actual = json!({
        "expected_anchor_id": anchor_id,
        "result": contexts_snapshot(&contexts),
    });
    assert_quantum_fixture("semantic_ignition/respects_min_vector_score.json", &actual);
    Ok(())
}

#[tokio::test]
async fn test_quantum_contexts_from_semantic_ignition_propagates_backend_errors()
-> Result<(), Box<dyn Error>> {
    let fixture = build_hybrid_fixture()?;
    let error = match fixture
        .index()
        .quantum_contexts_from_semantic_ignition(
            &ErrorIgnition,
            QuantumSemanticSearchRequest {
                query_text: Some("plain"),
                query_vector: &[0.4, 0.5],
                limit: 1,
                min_vector_score: None,
            },
            &QuantumFusionOptions::default(),
        )
        .await
    {
        Err(error) => error,
        Ok(contexts) => panic!(
            "semantic ignition should fail, but returned {} contexts",
            contexts.len()
        ),
    };

    let actual = match error {
        QuantumSemanticIgnitionError::Backend {
            backend_name,
            source,
        } => json!({
            "kind": "backend",
            "backend_name": backend_name,
            "source": source.to_string(),
        }),
        QuantumSemanticIgnitionError::Orchestration {
            backend_name,
            source,
        } => json!({
            "kind": "orchestration",
            "backend_name": backend_name,
            "source": source.to_string(),
        }),
    };
    assert_quantum_fixture("semantic_ignition/backend_error.json", &actual);
    Ok(())
}
