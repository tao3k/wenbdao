mod attachment;
mod cache;
mod coordinator;
mod corpus;
mod knowledge_section;
mod local_symbol;
mod manifest;
mod project_fingerprint;
mod ranking;
mod reference_occurrence;
mod repo_content_chunk;
mod repo_entity;
mod repo_publication_parquet;
mod repo_staging;
mod service;
mod staged_mutation;
mod status;

pub(crate) use attachment::AttachmentSearchError;
pub(crate) use cache::SearchPlaneCacheTtl;
pub use coordinator::{BeginBuildDecision, SearchBuildLease, SearchPlaneCoordinator};
pub use corpus::SearchCorpusKind;
pub(crate) use knowledge_section::KnowledgeSectionSearchError;
pub(crate) use local_symbol::LocalSymbolSearchError;
pub(crate) use manifest::SearchRepoPublicationInput;
pub use manifest::{
    SearchFileFingerprint, SearchManifestKeyspace, SearchManifestRecord,
    SearchPublicationStorageFormat, SearchRepoCorpusRecord, SearchRepoCorpusSnapshotRecord,
    SearchRepoPublicationRecord, SearchRepoRuntimeRecord,
};
#[allow(unused_imports)]
pub(crate) use project_fingerprint::{
    ProjectScannedFile, fingerprint_note_projects, fingerprint_source_projects,
    fingerprint_symbol_projects, scan_note_project_files, scan_source_project_files,
    scan_symbol_project_files,
};
pub(crate) use reference_occurrence::ReferenceOccurrenceSearchError;
#[cfg(test)]
pub(crate) use repo_entity::publish_repo_entities;
pub(crate) use repo_entity::{
    search_repo_entity_example_results, search_repo_entity_import_results,
    search_repo_entity_module_results, search_repo_entity_symbol_results,
};
pub(crate) use repo_staging::{
    RepoStagedMutationAction, RepoStagedMutationPlan, plan_repo_staged_mutation,
};
pub(crate) use service::RepoSearchAvailability;
pub(crate) use service::RepoSearchPublicationState;
pub(crate) use service::RepoSearchQueryCacheKeyInput;
pub use service::SearchPlaneService;
pub(crate) use staged_mutation::delete_paths_from_table;
pub use status::{
    SearchCorpusIssue, SearchCorpusIssueCode, SearchCorpusIssueFamily, SearchCorpusIssueSummary,
    SearchCorpusStatus, SearchCorpusStatusAction, SearchCorpusStatusReason,
    SearchCorpusStatusReasonCode, SearchCorpusStatusSeverity, SearchMaintenancePolicy,
    SearchMaintenanceStatus, SearchPlanePhase, SearchPlaneStatusSnapshot, SearchQueryTelemetry,
    SearchQueryTelemetrySource, SearchRepoReadPressure,
};
