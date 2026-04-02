use std::collections::{BTreeMap, BTreeSet};

use crate::gateway::studio::repo_index::RepoCodeDocument;
use crate::search_plane::repo_entity::build::RepoEntityBuildPlan;
use crate::search_plane::repo_entity::schema::RepoEntityRow;
use crate::search_plane::{
    SearchCorpusKind, SearchFileFingerprint, SearchPlaneService, plan_repo_staged_mutation,
};

const REPO_ENTITY_EXTRACTOR_VERSION: u32 = 1;

pub(crate) fn plan_repo_entity_build(
    repo_id: &str,
    rows: &[RepoEntityRow],
    documents: &[RepoCodeDocument],
    source_revision: Option<&str>,
    previous_publication: Option<&crate::search_plane::SearchRepoPublicationRecord>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
) -> RepoEntityBuildPlan {
    let file_fingerprints = repo_entity_file_fingerprints(rows, documents);
    let changed_paths = file_fingerprints
        .iter()
        .filter_map(|(path, fingerprint)| {
            (previous_fingerprints.get(path) != Some(fingerprint)).then_some(path.clone())
        })
        .collect::<BTreeSet<_>>();
    let changed_rows = rows
        .iter()
        .filter(|row| changed_paths.contains(row.path()))
        .cloned()
        .collect::<Vec<_>>();
    let deleted_paths = previous_fingerprints
        .keys()
        .filter(|path| !file_fingerprints.contains_key(*path))
        .cloned()
        .collect::<BTreeSet<_>>();

    plan_repo_staged_mutation(
        repo_id,
        SearchPlaneService::repo_entity_table_name(repo_id).as_str(),
        SearchCorpusKind::RepoEntity,
        REPO_ENTITY_EXTRACTOR_VERSION,
        source_revision,
        previous_publication,
        previous_fingerprints,
        file_fingerprints,
        rows.to_vec(),
        changed_rows,
        changed_paths,
        deleted_paths,
    )
}

pub(crate) fn repo_entity_file_fingerprints(
    rows: &[RepoEntityRow],
    documents: &[RepoCodeDocument],
) -> BTreeMap<String, SearchFileFingerprint> {
    let documents_by_path = documents
        .iter()
        .map(|document| (document.path.as_str(), document))
        .collect::<BTreeMap<_, _>>();
    let mut row_hash_by_path = BTreeMap::<String, blake3::Hasher>::new();

    for row in rows {
        let hasher = row_hash_by_path
            .entry(row.path().to_string())
            .or_insert_with(blake3::Hasher::new);
        row.update_fingerprint(hasher);
    }

    row_hash_by_path
        .into_iter()
        .map(|(path, hasher)| {
            let row_hash = hasher.finalize().to_hex().to_string();
            let fingerprint = if let Some(document) = documents_by_path.get(path.as_str()) {
                let mut fingerprint = document.to_file_fingerprint(
                    REPO_ENTITY_EXTRACTOR_VERSION,
                    SearchCorpusKind::RepoEntity.schema_version(),
                );
                fingerprint.blake3 = Some(row_hash);
                fingerprint
            } else {
                SearchFileFingerprint {
                    relative_path: path.clone(),
                    partition_id: None,
                    size_bytes: 0,
                    modified_unix_ms: 0,
                    extractor_version: REPO_ENTITY_EXTRACTOR_VERSION,
                    schema_version: SearchCorpusKind::RepoEntity.schema_version(),
                    blake3: Some(row_hash),
                }
            };
            (path, fingerprint)
        })
        .collect()
}
