use std::collections::{BTreeMap, BTreeSet};

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_content_chunk::build::types::{
    REPO_CONTENT_CHUNK_EXTRACTOR_VERSION, RepoContentChunkBuildPlan,
};
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchPlaneService, plan_repo_staged_mutation,
};

pub(crate) fn plan_repo_content_chunk_build(
    repo_id: &str,
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
    previous_publication: Option<&crate::search_plane::SearchRepoPublicationRecord>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> RepoContentChunkBuildPlan {
    let file_fingerprints = documents
        .iter()
        .map(|document| {
            (
                document.path.clone(),
                document.to_file_fingerprint(
                    REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
                    SearchCorpusKind::RepoContentChunk.schema_version(),
                ),
            )
        })
        .collect::<BTreeMap<_, _>>();

    let changed_documents = documents
        .iter()
        .filter(|document| {
            previous_fingerprints.get(document.path.as_str())
                != file_fingerprints.get(document.path.as_str())
        })
        .cloned()
        .collect::<Vec<_>>();
    let changed_paths = changed_documents
        .iter()
        .map(|document| document.path.clone())
        .collect::<BTreeSet<_>>();
    let deleted_paths = previous_fingerprints
        .keys()
        .filter(|path| !file_fingerprints.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();

    plan_repo_staged_mutation(
        repo_id,
        SearchPlaneService::repo_content_chunk_table_name(repo_id).as_str(),
        SearchCorpusKind::RepoContentChunk,
        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
        source_revision,
        previous_publication,
        previous_fingerprints,
        file_fingerprints,
        documents.to_vec(),
        changed_documents,
        changed_paths,
        deleted_paths,
    )
}

#[cfg(test)]
pub(crate) fn versioned_repo_content_table_name(
    repo_id: &str,
    file_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    source_revision: Option<&str>,
) -> String {
    crate::search_plane::repo_staging::versioned_repo_table_name(
        SearchPlaneService::repo_content_chunk_table_name(repo_id).as_str(),
        repo_id,
        file_fingerprints,
        source_revision,
        SearchCorpusKind::RepoContentChunk,
        REPO_CONTENT_CHUNK_EXTRACTOR_VERSION,
    )
}
