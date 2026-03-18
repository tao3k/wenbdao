use std::collections::HashSet;
use std::path::Path;

use regex::RegexBuilder;
use walkdir::{DirEntry, WalkDir};
use xiuxian_ast::{Lang, extract_items, get_skeleton_patterns};

use crate::dependency_indexer::extract_symbols;
use crate::gateway::studio::types::{AstSearchHit, ReferenceSearchHit, UiProjectConfig};
use crate::unified_symbol::UnifiedSymbolIndex;

use super::project_scope::{
    configured_project_scan_roots, index_path_for_entry, project_metadata_for_path,
};
use super::{first_signature_line, infer_crate_name, score_reference_hit, source_language_label};

pub(super) fn build_ast_index(
    project_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<Vec<AstSearchHit>, String> {
    let mut hits = Vec::new();
    let mut seen = HashSet::new();

    for root in configured_project_scan_roots(project_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            let Some(lang) = ast_search_lang(normalized_path_ref) else {
                continue;
            };

            let content = std::fs::read_to_string(entry.path())
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;
            let crate_name = infer_crate_name(normalized_path_ref);

            for pattern in get_skeleton_patterns(lang) {
                for result in extract_items(content.as_str(), pattern, lang, Some(vec!["NAME"])) {
                    let name =
                        result.captures.get("NAME").cloned().unwrap_or_else(|| {
                            first_signature_line(result.text.as_str()).to_string()
                        });
                    let signature = first_signature_line(result.text.as_str()).to_string();
                    if signature.is_empty() {
                        continue;
                    }
                    let dedupe_key = format!(
                        "{normalized_path}:{}:{}:{name}",
                        result.line_start, result.line_end
                    );
                    if !seen.insert(dedupe_key) {
                        continue;
                    }

                    hits.push(AstSearchHit {
                        name,
                        signature,
                        path: normalized_path.clone(),
                        language: lang.as_str().to_string(),
                        crate_name: crate_name.clone(),
                        project_name: None,
                        root_label: None,
                        line_start: result.line_start,
                        line_end: result.line_end,
                        score: 0.0,
                    });
                }
            }
        }
    }

    Ok(hits)
}

pub(super) fn build_symbol_index(
    project_root: &Path,
    projects: &[UiProjectConfig],
) -> Result<UnifiedSymbolIndex, String> {
    let mut index = UnifiedSymbolIndex::new();

    for root in configured_project_scan_roots(project_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let Some(language) = source_language_label(entry.path()) else {
                continue;
            };
            let normalized_path = index_path_for_entry(project_root, entry.path());
            let crate_name = infer_crate_name(Path::new(normalized_path.as_str()));
            let symbols = extract_symbols(entry.path(), language)
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            for symbol in symbols {
                let location = format!("{normalized_path}:{}", symbol.line);
                index.add_project_symbol(
                    symbol.name.as_str(),
                    super::symbol_kind_label(&symbol.kind),
                    location.as_str(),
                    crate_name.as_str(),
                );
            }
        }
    }

    Ok(index)
}

pub(super) fn build_reference_hits(
    project_root: &Path,
    projects: &[UiProjectConfig],
    ast_hits: &[AstSearchHit],
    query: &str,
    limit: usize,
) -> Result<Vec<ReferenceSearchHit>, String> {
    let regex = RegexBuilder::new(format!(r"\b{}\b", regex::escape(query)).as_str())
        .case_insensitive(true)
        .build()
        .map_err(|error| error.to_string())?;
    let definition_locations = ast_hits
        .iter()
        .filter(|hit| hit.name.eq_ignore_ascii_case(query))
        .map(|hit| (hit.path.clone(), hit.line_start))
        .collect::<HashSet<_>>();

    let mut hits = Vec::new();

    for root in configured_project_scan_roots(project_root, projects) {
        for entry in WalkDir::new(root.as_path())
            .into_iter()
            .filter_entry(|entry| !should_skip_entry(entry))
        {
            let entry = entry.map_err(|error| error.to_string())?;
            if !entry.file_type().is_file() {
                continue;
            }

            let normalized_path = index_path_for_entry(project_root, entry.path());
            let normalized_path_ref = Path::new(normalized_path.as_str());
            let Some(lang) = ast_search_lang(normalized_path_ref) else {
                continue;
            };
            let crate_name = infer_crate_name(normalized_path_ref);
            let content = std::fs::read_to_string(entry.path())
                .map_err(|error| format!("{}: {error}", entry.path().display()))?;

            for (line_idx, line_text) in content.lines().enumerate() {
                let line_number = line_idx + 1;
                if definition_locations.contains(&(normalized_path.clone(), line_number)) {
                    continue;
                }

                let Some(mat) = regex.find(line_text) else {
                    continue;
                };
                let metadata =
                    project_metadata_for_path(project_root, projects, normalized_path.as_str());

                hits.push(ReferenceSearchHit {
                    name: query.to_string(),
                    path: normalized_path.clone(),
                    language: lang.as_str().to_string(),
                    crate_name: crate_name.clone(),
                    project_name: metadata.project_name,
                    root_label: metadata.root_label,
                    line: line_number,
                    column: line_text[..mat.start()].chars().count() + 1,
                    line_text: line_text.trim().to_string(),
                    score: score_reference_hit(line_text, query),
                });
            }
        }
    }

    hits.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.path.cmp(&right.path))
            .then_with(|| left.line.cmp(&right.line))
            .then_with(|| left.column.cmp(&right.column))
    });
    hits.truncate(limit);

    Ok(hits)
}

fn should_skip_entry(entry: &DirEntry) -> bool {
    if !entry.file_type().is_dir() {
        return false;
    }

    matches!(
        entry.file_name().to_string_lossy().as_ref(),
        ".git"
            | ".cache"
            | ".devenv"
            | ".direnv"
            | ".run"
            | "target"
            | "node_modules"
            | "dist"
            | "coverage"
            | "__pycache__"
    )
}

fn ast_search_lang(path: &Path) -> Option<Lang> {
    match Lang::from_path(path)? {
        Lang::Python
        | Lang::Rust
        | Lang::JavaScript
        | Lang::TypeScript
        | Lang::Bash
        | Lang::Go
        | Lang::Java
        | Lang::C
        | Lang::Cpp
        | Lang::CSharp
        | Lang::Ruby
        | Lang::Swift
        | Lang::Kotlin
        | Lang::Lua
        | Lang::Php => Lang::from_path(path),
        _ => None,
    }
}
