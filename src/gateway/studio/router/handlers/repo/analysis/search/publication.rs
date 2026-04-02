use crate::gateway::studio::router::GatewayState;
use crate::search_plane::SearchCorpusKind;

pub(crate) async fn repo_entity_publication_ready(
    state: &std::sync::Arc<GatewayState>,
    repo_id: &str,
) -> bool {
    state
        .studio
        .search_plane
        .repo_corpus_record_for_reads(SearchCorpusKind::RepoEntity, repo_id)
        .await
        .and_then(|record| record.publication)
        .is_some()
}
