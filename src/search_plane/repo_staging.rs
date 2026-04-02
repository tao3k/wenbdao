use std::collections::{BTreeMap, BTreeSet};

use crate::search_plane::{SearchCorpusKind, SearchFileFingerprint, SearchRepoPublicationRecord};

#[derive(Debug, Clone)]
pub(crate) enum RepoStagedMutationAction<T> {
    Noop,
    RefreshPublication {
        table_name: String,
    },
    ReplaceAll {
        table_name: String,
        payload: T,
    },
    CloneAndMutate {
        base_table_name: String,
        target_table_name: String,
        replaced_paths: BTreeSet<String>,
        changed_payload: T,
    },
}

#[derive(Debug, Clone)]
pub(crate) struct RepoStagedMutationPlan<T> {
    pub(crate) file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    pub(crate) action: RepoStagedMutationAction<T>,
}

#[must_use]
pub(crate) fn plan_repo_staged_mutation<T>(
    repo_id: &str,
    table_name_prefix: &str,
    corpus: SearchCorpusKind,
    extractor_version: u32,
    source_revision: Option<&str>,
    previous_publication: Option<&SearchRepoPublicationRecord>,
    previous_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    file_fingerprints: BTreeMap<String, SearchFileFingerprint>,
    replace_payload: T,
    changed_payload: T,
    changed_paths: BTreeSet<String>,
    deleted_paths: BTreeSet<String>,
) -> RepoStagedMutationPlan<T> {
    let Some(previous_publication) = previous_publication else {
        return RepoStagedMutationPlan {
            file_fingerprints: file_fingerprints.clone(),
            action: RepoStagedMutationAction::ReplaceAll {
                table_name: versioned_repo_table_name(
                    table_name_prefix,
                    repo_id,
                    &file_fingerprints,
                    source_revision,
                    corpus,
                    extractor_version,
                ),
                payload: replace_payload,
            },
        };
    };

    if previous_fingerprints == file_fingerprints {
        return RepoStagedMutationPlan {
            file_fingerprints,
            action: if previous_publication.source_revision.as_deref() == source_revision {
                RepoStagedMutationAction::Noop
            } else {
                RepoStagedMutationAction::RefreshPublication {
                    table_name: previous_publication.table_name.clone(),
                }
            },
        };
    }

    let mut replaced_paths = changed_paths;
    replaced_paths.extend(deleted_paths);
    RepoStagedMutationPlan {
        file_fingerprints: file_fingerprints.clone(),
        action: RepoStagedMutationAction::CloneAndMutate {
            base_table_name: previous_publication.table_name.clone(),
            target_table_name: versioned_repo_table_name(
                table_name_prefix,
                repo_id,
                &file_fingerprints,
                source_revision,
                corpus,
                extractor_version,
            ),
            replaced_paths,
            changed_payload,
        },
    }
}

#[must_use]
pub(crate) fn versioned_repo_table_name(
    table_name_prefix: &str,
    repo_id: &str,
    file_fingerprints: &BTreeMap<String, SearchFileFingerprint>,
    source_revision: Option<&str>,
    corpus: SearchCorpusKind,
    extractor_version: u32,
) -> String {
    let mut payload = format!(
        "{repo_id}|{}|schema:{}|extractor:{}",
        source_revision
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase(),
        corpus.schema_version(),
        extractor_version,
    );
    for (path, fingerprint) in file_fingerprints {
        payload.push('|');
        payload.push_str(path.as_str());
        payload.push(':');
        payload.push_str(fingerprint.size_bytes.to_string().as_str());
        payload.push(':');
        payload.push_str(fingerprint.modified_unix_ms.to_string().as_str());
        payload.push(':');
        payload.push_str(fingerprint.blake3.as_deref().unwrap_or_default());
    }
    let token = blake3::hash(payload.as_bytes()).to_hex().to_string();
    format!("{table_name_prefix}_{}", &token[..16])
}
