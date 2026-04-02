#[cfg(test)]
use std::path::Path;

#[cfg(test)]
use super::types::SearchPlaneService;
#[cfg(test)]
use crate::gateway::studio::types::{AstSearchHit, UiProjectConfig};
#[cfg(test)]
use crate::search_plane::attachment::AttachmentBuildError;
#[cfg(test)]
use crate::search_plane::knowledge_section::KnowledgeSectionBuildError;
#[cfg(test)]
use crate::search_plane::local_symbol::LocalSymbolBuildError;
#[cfg(test)]
use crate::search_plane::reference_occurrence::ReferenceOccurrenceBuildError;

#[cfg(test)]
impl SearchPlaneService {
    pub(crate) async fn publish_local_symbol_hits(
        &self,
        fingerprint: &str,
        hits: &[AstSearchHit],
    ) -> Result<(), LocalSymbolBuildError> {
        crate::search_plane::local_symbol::publish_local_symbol_hits(self, fingerprint, hits).await
    }

    pub(crate) async fn publish_reference_occurrences_from_projects(
        &self,
        project_root: &Path,
        config_root: &Path,
        projects: &[UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), ReferenceOccurrenceBuildError> {
        crate::search_plane::reference_occurrence::publish_reference_occurrences_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }

    pub(crate) async fn publish_attachments_from_projects(
        &self,
        project_root: &Path,
        config_root: &Path,
        projects: &[UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), AttachmentBuildError> {
        crate::search_plane::attachment::publish_attachments_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }

    pub(crate) async fn publish_knowledge_sections_from_projects(
        &self,
        project_root: &Path,
        config_root: &Path,
        projects: &[UiProjectConfig],
        fingerprint: &str,
    ) -> Result<(), KnowledgeSectionBuildError> {
        crate::search_plane::knowledge_section::publish_knowledge_sections_from_projects(
            self,
            project_root,
            config_root,
            projects,
            fingerprint,
        )
        .await
    }
}
