use super::types::SearchPlaneService;
use crate::gateway::studio::types::UiProjectConfig;
use crate::search_plane::{
    AttachmentSearchError, KnowledgeSectionSearchError, LocalSymbolSearchError,
    ReferenceOccurrenceSearchError,
};

impl SearchPlaneService {
    pub(crate) fn ensure_local_symbol_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[UiProjectConfig],
    ) {
        crate::search_plane::local_symbol::ensure_local_symbol_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_local_symbols(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::gateway::studio::types::AstSearchHit>, LocalSymbolSearchError> {
        crate::search_plane::local_symbol::search_local_symbols(self, query, limit).await
    }

    pub(crate) fn ensure_knowledge_section_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[UiProjectConfig],
    ) {
        crate::search_plane::knowledge_section::ensure_knowledge_section_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_knowledge_sections(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<crate::gateway::studio::types::SearchHit>, KnowledgeSectionSearchError> {
        crate::search_plane::knowledge_section::search_knowledge_sections(self, query, limit).await
    }

    pub(crate) fn ensure_attachment_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[UiProjectConfig],
    ) {
        crate::search_plane::attachment::ensure_attachment_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_attachment_hits(
        &self,
        query: &str,
        limit: usize,
        extensions: &[String],
        kinds: &[crate::link_graph::LinkGraphAttachmentKind],
        case_sensitive: bool,
    ) -> Result<Vec<crate::gateway::studio::types::AttachmentSearchHit>, AttachmentSearchError>
    {
        crate::search_plane::attachment::search_attachment_hits(
            self,
            query,
            limit,
            extensions,
            kinds,
            case_sensitive,
        )
        .await
    }

    pub(crate) async fn autocomplete_local_symbols(
        &self,
        prefix: &str,
        limit: usize,
    ) -> Result<Vec<crate::gateway::studio::types::AutocompleteSuggestion>, LocalSymbolSearchError>
    {
        crate::search_plane::local_symbol::autocomplete_local_symbols(self, prefix, limit).await
    }

    pub(crate) fn ensure_reference_occurrence_index_started(
        &self,
        project_root: &std::path::Path,
        config_root: &std::path::Path,
        projects: &[UiProjectConfig],
    ) {
        crate::search_plane::reference_occurrence::ensure_reference_occurrence_index_started(
            self,
            project_root,
            config_root,
            projects,
        );
    }

    pub(crate) async fn search_reference_occurrences(
        &self,
        query: &str,
        limit: usize,
    ) -> Result<
        Vec<crate::gateway::studio::types::ReferenceSearchHit>,
        ReferenceOccurrenceSearchError,
    > {
        crate::search_plane::reference_occurrence::search_reference_occurrences(self, query, limit)
            .await
    }
}
