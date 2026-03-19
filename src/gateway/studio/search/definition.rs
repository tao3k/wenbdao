use std::cmp::Ordering;
use std::path::Path;

use crate::gateway::studio::pathing;
use crate::gateway::studio::types::{AstSearchHit, UiProjectConfig};
use crate::link_graph::parser::code_observation::path_matches_scope;

use super::project_scope::{normalize_path, project_metadata_for_path};
use super::support::infer_crate_name;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum DefinitionMatchMode {
    ExactOnly,
    ExactThenFuzzy,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct DefinitionResolveOptions<'a> {
    pub(crate) source_paths: Option<&'a [String]>,
    pub(crate) scope_patterns: Option<&'a [String]>,
    pub(crate) languages: Option<&'a [String]>,
    pub(crate) match_mode: DefinitionMatchMode,
    pub(crate) include_markdown: bool,
}

impl Default for DefinitionResolveOptions<'_> {
    fn default() -> Self {
        Self {
            source_paths: None,
            scope_patterns: None,
            languages: None,
            match_mode: DefinitionMatchMode::ExactThenFuzzy,
            include_markdown: true,
        }
    }
}

pub(crate) fn enrich_ast_hit_project_metadata(
    hit: &mut AstSearchHit,
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
) {
    let metadata =
        project_metadata_for_path(project_root, config_root, projects, hit.path.as_str());
    hit.project_name = metadata.project_name;
    hit.root_label = metadata.root_label;
}

pub(crate) fn resolve_best_definition(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    index: &[AstSearchHit],
    query: &str,
    options: DefinitionResolveOptions<'_>,
) -> Option<AstSearchHit> {
    resolve_definition_candidates(project_root, config_root, projects, index, query, options)
        .into_iter()
        .next()
}

pub(crate) fn resolve_definition_candidates(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    index: &[AstSearchHit],
    query: &str,
    options: DefinitionResolveOptions<'_>,
) -> Vec<AstSearchHit> {
    let mut candidates = collect_definition_candidates(
        project_root,
        config_root,
        projects,
        index,
        query,
        options,
        true,
    );

    if candidates.is_empty() && matches!(options.match_mode, DefinitionMatchMode::ExactThenFuzzy) {
        candidates = collect_definition_candidates(
            project_root,
            config_root,
            projects,
            index,
            query,
            options,
            false,
        );
    }

    candidates = prefer_language_matched_candidates(candidates, options.languages);
    candidates = prefer_scope_matched_candidates(candidates, options.scope_patterns);

    candidates.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(Ordering::Equal)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line_start.cmp(&right.line_start))
    });
    candidates
}

fn collect_definition_candidates(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    index: &[AstSearchHit],
    query: &str,
    options: DefinitionResolveOptions<'_>,
    exact_only: bool,
) -> Vec<AstSearchHit> {
    index
        .iter()
        .filter(|hit| {
            pathing::path_matches_project_file_filters(
                project_root,
                config_root,
                projects,
                hit.path.as_str(),
            )
        })
        .filter(|hit| options.include_markdown || !hit.language.eq_ignore_ascii_case("markdown"))
        .filter(|hit| {
            if exact_only {
                hit.name.eq_ignore_ascii_case(query)
            } else {
                ast_hit_matches(hit, query)
            }
        })
        .map(|hit| {
            let mut hit = hit.clone();
            enrich_ast_hit_project_metadata(&mut hit, project_root, config_root, projects);
            hit.score = score_definition_hit(
                &hit,
                query,
                options.source_paths,
                options.scope_patterns,
                options.languages,
            );
            hit
        })
        .collect()
}

fn prefer_language_matched_candidates(
    candidates: Vec<AstSearchHit>,
    languages: Option<&[String]>,
) -> Vec<AstSearchHit> {
    let Some(languages) = languages.filter(|languages| !languages.is_empty()) else {
        return candidates;
    };

    let matched = candidates
        .iter()
        .filter(|hit| {
            languages
                .iter()
                .any(|language| hit.language.eq_ignore_ascii_case(language.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    if matched.is_empty() {
        candidates
    } else {
        matched
    }
}

fn prefer_scope_matched_candidates(
    candidates: Vec<AstSearchHit>,
    scope_patterns: Option<&[String]>,
) -> Vec<AstSearchHit> {
    let Some(scope_patterns) = scope_patterns.filter(|patterns| !patterns.is_empty()) else {
        return candidates;
    };

    let scoped = candidates
        .iter()
        .filter(|hit| {
            scope_patterns
                .iter()
                .any(|scope| path_matches_scope(hit.path.as_str(), scope.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();
    if scoped.is_empty() {
        candidates
    } else {
        scoped
    }
}

pub(crate) fn ast_hit_matches(hit: &AstSearchHit, query: &str) -> bool {
    let query_lc = query.to_ascii_lowercase();
    hit.name.to_ascii_lowercase().contains(query_lc.as_str())
        || hit
            .signature
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
        || hit.path.to_ascii_lowercase().contains(query_lc.as_str())
        || hit
            .language
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
        || hit
            .crate_name
            .to_ascii_lowercase()
            .contains(query_lc.as_str())
        || hit
            .node_kind
            .as_ref()
            .is_some_and(|value| value.to_ascii_lowercase().contains(query_lc.as_str()))
        || hit
            .owner_title
            .as_ref()
            .is_some_and(|value| value.to_ascii_lowercase().contains(query_lc.as_str()))
}

pub(crate) fn score_ast_hit(hit: &AstSearchHit, query: &str) -> f64 {
    let query_lc = query.to_ascii_lowercase();
    let name_lc = hit.name.to_ascii_lowercase();
    let signature_lc = hit.signature.to_ascii_lowercase();
    let path_lc = hit.path.to_ascii_lowercase();
    let owner_title_lc = hit
        .owner_title
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let node_kind_lc = hit
        .node_kind
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name_lc == query_lc {
        1.0
    } else if name_lc.starts_with(query_lc.as_str()) {
        0.95
    } else if name_lc.contains(query_lc.as_str()) {
        0.88
    } else if owner_title_lc.contains(query_lc.as_str()) {
        0.84
    } else if signature_lc.contains(query_lc.as_str()) {
        0.8
    } else if node_kind_lc.contains(query_lc.as_str()) {
        0.76
    } else if path_lc.contains(query_lc.as_str()) {
        0.72
    } else {
        0.5
    }
}

fn score_definition_hit(
    hit: &AstSearchHit,
    query: &str,
    source_paths: Option<&[String]>,
    scope_patterns: Option<&[String]>,
    languages: Option<&[String]>,
) -> f64 {
    let mut score = score_ast_hit(hit, query);

    if let Some(source_paths) = source_paths {
        let hit_parent = Path::new(hit.path.as_str()).parent().map(normalize_path);
        let source_bonus = source_paths
            .iter()
            .map(|source_path| {
                let normalized_source_path = source_path.replace('\\', "/");
                let source_path = Path::new(normalized_source_path.as_str());
                let source_crate = infer_crate_name(source_path);
                let mut bonus = 0.0;

                if hit.path == normalized_source_path {
                    bonus += 0.15;
                }

                if hit.crate_name.eq_ignore_ascii_case(source_crate.as_str()) {
                    bonus += 0.1;
                }

                let source_parent = source_path.parent().map(normalize_path);
                if source_parent.is_some() && source_parent == hit_parent {
                    bonus += 0.05;
                }

                bonus
            })
            .fold(0.0, f64::max);
        score += source_bonus;
    }

    if let Some(scope_patterns) = scope_patterns
        && scope_patterns
            .iter()
            .any(|scope| path_matches_scope(hit.path.as_str(), scope.as_str()))
    {
        score += 0.2;
    }

    if let Some(languages) = languages
        && languages
            .iter()
            .any(|language| hit.language.eq_ignore_ascii_case(language.as_str()))
    {
        score += 0.2;
    }

    score
}
