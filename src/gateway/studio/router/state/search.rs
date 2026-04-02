use std::time::Duration;

use crate::gateway::studio::router::error::StudioApiError;
use crate::gateway::studio::router::state::types::StudioState;
use crate::gateway::studio::types::{
    AstSearchHit, AttachmentSearchHit, AutocompleteSuggestion, ReferenceSearchHit, SearchHit,
};
use crate::link_graph::LinkGraphAttachmentKind;
use crate::search_plane::{SearchCorpusKind, SearchPlanePhase};

const LOCAL_CORPUS_READY_WAIT_ENV: &str = "XIUXIAN_WENDAO_LOCAL_CORPUS_READY_WAIT_MS";
const DEFAULT_LOCAL_CORPUS_READY_WAIT_MS: u64 = 15_000;
const LOCAL_CORPUS_READY_POLL_INTERVAL: Duration = Duration::from_millis(25);

impl StudioState {
    async fn wait_for_initial_local_corpus_ready(
        &self,
        corpus: SearchCorpusKind,
    ) -> Result<(), StudioApiError> {
        let timeout = local_corpus_ready_wait_duration();
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let status = self.search_plane.coordinator().status_for(corpus);
            if status.active_epoch.is_some() {
                return Ok(());
            }
            if matches!(status.phase, SearchPlanePhase::Failed) {
                return Err(StudioApiError::internal(
                    "SEARCH_INDEX_BUILD_FAILED",
                    format!("search corpus `{corpus}` failed to publish an index epoch"),
                    status.last_error.clone(),
                ));
            }
            if tokio::time::Instant::now() >= deadline {
                return Err(StudioApiError::index_not_ready(corpus.as_str()));
            }
            tokio::time::sleep(LOCAL_CORPUS_READY_POLL_INTERVAL).await;
        }
    }

    pub(crate) fn ensure_local_symbol_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio AST search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_local_symbol_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn ensure_local_symbol_index_ready(&self) -> Result<(), StudioApiError> {
        self.ensure_local_symbol_index_started()?;
        self.wait_for_initial_local_corpus_ready(SearchCorpusKind::LocalSymbol)
            .await
    }

    pub(crate) fn ensure_knowledge_section_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio knowledge search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_knowledge_section_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn ensure_knowledge_section_index_ready(&self) -> Result<(), StudioApiError> {
        self.ensure_knowledge_section_index_started()?;
        self.wait_for_initial_local_corpus_ready(SearchCorpusKind::KnowledgeSection)
            .await
    }

    pub(crate) async fn search_knowledge_sections(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<SearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_knowledge_sections(query, limit)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::KnowledgeSectionSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("knowledge_section"))
            }
            Err(error) => Err(StudioApiError::internal(
                "KNOWLEDGE_SECTION_SEARCH_FAILED",
                "Failed to query knowledge section search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) async fn search_local_symbol_hits(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<AstSearchHit>, StudioApiError> {
        match self.search_plane.search_local_symbols(query, limit).await {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::LocalSymbolSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("local_symbol"))
            }
            Err(error) => Err(StudioApiError::internal(
                "LOCAL_SYMBOL_SEARCH_FAILED",
                "Failed to query local symbol search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) async fn autocomplete_local_symbols(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<AutocompleteSuggestion>, StudioApiError> {
        match self
            .search_plane
            .autocomplete_local_symbols(prefix, limit)
            .await
        {
            Ok(suggestions) => Ok(suggestions),
            Err(crate::search_plane::LocalSymbolSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("local_symbol"))
            }
            Err(error) => Err(StudioApiError::internal(
                "LOCAL_SYMBOL_AUTOCOMPLETE_FAILED",
                "Failed to query local symbol autocomplete search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) fn ensure_attachment_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio attachment search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_attachment_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn ensure_attachment_index_ready(&self) -> Result<(), StudioApiError> {
        self.ensure_attachment_index_started()?;
        self.wait_for_initial_local_corpus_ready(SearchCorpusKind::Attachment)
            .await
    }

    pub(crate) async fn search_attachment_hits(
        &self,
        query: &str,
        limit: usize,
        extensions: &[String],
        kinds: &[LinkGraphAttachmentKind],
        case_sensitive: bool,
    ) -> Result<Vec<AttachmentSearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_attachment_hits(query, limit, extensions, kinds, case_sensitive)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::AttachmentSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("attachment"))
            }
            Err(error) => Err(StudioApiError::internal(
                "ATTACHMENT_SEARCH_FAILED",
                "Failed to query attachment search plane",
                Some(error.to_string()),
            )),
        }
    }

    pub(crate) fn ensure_reference_occurrence_index_started(&self) -> Result<(), StudioApiError> {
        let configured_projects = self.configured_projects();
        if configured_projects.is_empty() {
            return Err(StudioApiError::bad_request(
                "UI_CONFIG_REQUIRED",
                "Studio reference search requires configured link_graph.projects",
            ));
        }
        self.search_plane.ensure_reference_occurrence_index_started(
            self.project_root.as_path(),
            self.config_root.as_path(),
            configured_projects.as_slice(),
        );
        Ok(())
    }

    pub(crate) async fn ensure_reference_occurrence_index_ready(
        &self,
    ) -> Result<(), StudioApiError> {
        self.ensure_reference_occurrence_index_started()?;
        self.wait_for_initial_local_corpus_ready(SearchCorpusKind::ReferenceOccurrence)
            .await
    }

    pub(crate) async fn search_reference_occurrences(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ReferenceSearchHit>, StudioApiError> {
        match self
            .search_plane
            .search_reference_occurrences(query, limit)
            .await
        {
            Ok(hits) => Ok(hits),
            Err(crate::search_plane::ReferenceOccurrenceSearchError::NotReady) => {
                Err(StudioApiError::index_not_ready("reference_occurrence"))
            }
            Err(error) => Err(StudioApiError::internal(
                "REFERENCE_OCCURRENCE_SEARCH_FAILED",
                "Failed to query reference occurrence search plane",
                Some(error.to_string()),
            )),
        }
    }
}

fn local_corpus_ready_wait_duration() -> Duration {
    let parsed = std::env::var(LOCAL_CORPUS_READY_WAIT_ENV)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .filter(|value| *value > 0);
    Duration::from_millis(parsed.unwrap_or(DEFAULT_LOCAL_CORPUS_READY_WAIT_MS))
}
