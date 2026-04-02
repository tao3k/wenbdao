use std::sync::Arc;

use async_trait::async_trait;
use xiuxian_vector::{
    LanceDataType, LanceField, LanceFloat64Array, LanceRecordBatch, LanceSchema, LanceStringArray,
};
use xiuxian_wendao_runtime::transport::AttachmentSearchFlightRouteProvider;

use super::queries::AttachmentSearchQuery;
use crate::gateway::studio::router::{StudioApiError, StudioState};
use crate::gateway::studio::types::{AttachmentSearchHit, AttachmentSearchResponse};
use crate::link_graph::LinkGraphAttachmentKind;

pub(crate) async fn load_attachment_search_response_from_studio(
    studio: &StudioState,
    query: AttachmentSearchQuery,
) -> Result<AttachmentSearchResponse, StudioApiError> {
    let raw_query = query.q.unwrap_or_default();
    let query_text = raw_query.trim();
    if query_text.is_empty() {
        return Err(StudioApiError::bad_request(
            "MISSING_QUERY",
            "Attachment search requires a non-empty query",
        ));
    }

    let limit = query.limit.unwrap_or(20).max(1);
    let extensions = query
        .ext
        .iter()
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let kinds = query
        .kind
        .iter()
        .map(|value| LinkGraphAttachmentKind::from_alias(value))
        .collect::<Vec<_>>();
    studio.ensure_attachment_index_ready().await?;
    let hits = studio
        .search_attachment_hits(
            query_text,
            limit,
            extensions.as_slice(),
            kinds.as_slice(),
            query.case_sensitive,
        )
        .await?;

    Ok(AttachmentSearchResponse {
        query: query_text.to_string(),
        hit_count: hits.len(),
        hits,
        selected_scope: "attachments".to_string(),
    })
}

pub(crate) struct StudioAttachmentSearchFlightRouteProvider {
    studio: Arc<StudioState>,
}

impl StudioAttachmentSearchFlightRouteProvider {
    #[must_use]
    pub(crate) fn new(studio: Arc<StudioState>) -> Self {
        Self { studio }
    }
}

impl std::fmt::Debug for StudioAttachmentSearchFlightRouteProvider {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("StudioAttachmentSearchFlightRouteProvider")
            .finish_non_exhaustive()
    }
}

#[async_trait]
impl AttachmentSearchFlightRouteProvider for StudioAttachmentSearchFlightRouteProvider {
    async fn attachment_search_batch(
        &self,
        query_text: &str,
        limit: usize,
        ext_filters: &std::collections::HashSet<String>,
        kind_filters: &std::collections::HashSet<String>,
        case_sensitive: bool,
    ) -> Result<LanceRecordBatch, String> {
        let mut ext = ext_filters.iter().cloned().collect::<Vec<_>>();
        ext.sort();
        let mut kind = kind_filters.iter().cloned().collect::<Vec<_>>();
        kind.sort();
        let response = load_attachment_search_response_from_studio(
            self.studio.as_ref(),
            AttachmentSearchQuery {
                q: Some(query_text.to_string()),
                limit: Some(limit),
                ext,
                kind,
                case_sensitive,
            },
        )
        .await
        .map_err(|error| {
            error
                .error
                .details
                .clone()
                .unwrap_or_else(|| format!("{}: {}", error.code(), error.error.message))
        })?;
        build_attachment_hits_flight_batch(&response.hits)
    }
}

fn build_attachment_hits_flight_batch(
    hits: &[AttachmentSearchHit],
) -> Result<LanceRecordBatch, String> {
    let names = hits.iter().map(|hit| hit.name.clone()).collect::<Vec<_>>();
    let paths = hits.iter().map(|hit| hit.path.clone()).collect::<Vec<_>>();
    let source_ids = hits
        .iter()
        .map(|hit| hit.source_id.clone())
        .collect::<Vec<_>>();
    let source_stems = hits
        .iter()
        .map(|hit| hit.source_stem.clone())
        .collect::<Vec<_>>();
    let source_titles = hits
        .iter()
        .map(|hit| hit.source_title.clone())
        .collect::<Vec<_>>();
    let navigation_targets_json = hits
        .iter()
        .map(|hit| serde_json::to_string(&hit.navigation_target).map_err(|error| error.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let source_paths = hits
        .iter()
        .map(|hit| hit.source_path.clone())
        .collect::<Vec<_>>();
    let attachment_ids = hits
        .iter()
        .map(|hit| hit.attachment_id.clone())
        .collect::<Vec<_>>();
    let attachment_paths = hits
        .iter()
        .map(|hit| hit.attachment_path.clone())
        .collect::<Vec<_>>();
    let attachment_names = hits
        .iter()
        .map(|hit| hit.attachment_name.clone())
        .collect::<Vec<_>>();
    let attachment_exts = hits
        .iter()
        .map(|hit| hit.attachment_ext.clone())
        .collect::<Vec<_>>();
    let kinds = hits.iter().map(|hit| hit.kind.clone()).collect::<Vec<_>>();
    let scores = hits.iter().map(|hit| hit.score).collect::<Vec<_>>();
    let vision_snippets = hits
        .iter()
        .map(|hit| hit.vision_snippet.as_deref())
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        Arc::new(LanceSchema::new(vec![
            LanceField::new("name", LanceDataType::Utf8, false),
            LanceField::new("path", LanceDataType::Utf8, false),
            LanceField::new("sourceId", LanceDataType::Utf8, false),
            LanceField::new("sourceStem", LanceDataType::Utf8, false),
            LanceField::new("sourceTitle", LanceDataType::Utf8, false),
            LanceField::new("navigationTargetJson", LanceDataType::Utf8, true),
            LanceField::new("sourcePath", LanceDataType::Utf8, false),
            LanceField::new("attachmentId", LanceDataType::Utf8, false),
            LanceField::new("attachmentPath", LanceDataType::Utf8, false),
            LanceField::new("attachmentName", LanceDataType::Utf8, false),
            LanceField::new("attachmentExt", LanceDataType::Utf8, false),
            LanceField::new("kind", LanceDataType::Utf8, false),
            LanceField::new("score", LanceDataType::Float64, false),
            LanceField::new("visionSnippet", LanceDataType::Utf8, true),
        ])),
        vec![
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(source_ids)),
            Arc::new(LanceStringArray::from(source_stems)),
            Arc::new(LanceStringArray::from(source_titles)),
            Arc::new(LanceStringArray::from(
                navigation_targets_json
                    .iter()
                    .map(|value| Some(value.as_str()))
                    .collect::<Vec<_>>(),
            )),
            Arc::new(LanceStringArray::from(source_paths)),
            Arc::new(LanceStringArray::from(attachment_ids)),
            Arc::new(LanceStringArray::from(attachment_paths)),
            Arc::new(LanceStringArray::from(attachment_names)),
            Arc::new(LanceStringArray::from(attachment_exts)),
            Arc::new(LanceStringArray::from(kinds)),
            Arc::new(LanceFloat64Array::from(scores)),
            Arc::new(LanceStringArray::from(vision_snippets)),
        ],
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use xiuxian_wendao_runtime::transport::AttachmentSearchFlightRouteProvider;

    #[tokio::test]
    async fn studio_attachment_search_flight_provider_uses_attachment_contract() {
        let project_root = tempfile::tempdir().expect("attachment provider tempdir should build");
        std::fs::create_dir_all(project_root.path().join("docs/assets"))
            .expect("attachment provider docs asset dir should build");
        std::fs::write(
            project_root.path().join("docs/alpha.md"),
            "# Alpha\n\n![Topology](assets/topology.png)\n",
        )
        .expect("attachment provider source doc should write");

        let mut studio = crate::gateway::studio::search::handlers::tests::test_studio_state();
        studio.project_root = project_root.path().to_path_buf();
        studio.config_root = project_root.path().to_path_buf();
        studio.set_ui_config(crate::gateway::studio::types::UiConfig {
            projects: vec![crate::gateway::studio::types::UiProjectConfig {
                name: "kernel".to_string(),
                root: ".".to_string(),
                dirs: vec!["docs".to_string()],
            }],
            repo_projects: Vec::new(),
        });
        let studio = Arc::new(studio);
        let fingerprint = format!(
            "test:attachment:{}",
            blake3::hash(
                format!(
                    "{}:{}:{}",
                    studio.project_root.display(),
                    studio.config_root.display(),
                    studio.configured_projects().len()
                )
                .as_bytes()
            )
            .to_hex()
        );
        studio
            .search_plane
            .publish_attachments_from_projects(
                studio.project_root.as_path(),
                studio.config_root.as_path(),
                &studio.configured_projects(),
                fingerprint.as_str(),
            )
            .await
            .expect("attachment provider index should publish");

        let provider = StudioAttachmentSearchFlightRouteProvider::new(studio);

        let batch = provider
            .attachment_search_batch(
                "topology",
                5,
                &["png".to_string()].into_iter().collect(),
                &["image".to_string()].into_iter().collect(),
                false,
            )
            .await
            .expect("attachment provider should build a batch");

        assert_eq!(batch.num_rows(), 1);
        assert!(batch.column_by_name("attachmentPath").is_some());
        assert!(batch.column_by_name("navigationTargetJson").is_some());
    }
}
