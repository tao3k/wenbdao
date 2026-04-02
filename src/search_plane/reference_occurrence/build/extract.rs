use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use xiuxian_ast::Lang;

use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::search::source_index::build_ast_hits_for_file;
use crate::gateway::studio::search::support::infer_crate_name;
use crate::gateway::studio::types::{ReferenceSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::search_plane::ProjectScannedFile;

static REFERENCE_TOKEN_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[A-Za-z_][A-Za-z0-9_]*").unwrap_or_else(|error| {
        panic!("reference token regex must compile: {error}");
    })
});

pub(crate) fn build_reference_occurrences_for_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    files: &[ProjectScannedFile],
) -> Vec<ReferenceSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        hits.extend(build_reference_occurrences_for_file(
            project_root,
            config_root,
            projects,
            file,
        ));
    }
    hits
}

pub(crate) fn build_reference_occurrences_for_file(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    file: &ProjectScannedFile,
) -> Vec<ReferenceSearchHit> {
    let normalized_path_ref = Path::new(file.normalized_path.as_str());
    let Some(language) = reference_scan_lang(normalized_path_ref) else {
        return Vec::new();
    };
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        file.normalized_path.as_str(),
    );
    let crate_name = infer_crate_name(normalized_path_ref);
    let definition_locations = build_ast_hits_for_file(
        project_root,
        file.scan_root.as_path(),
        file.absolute_path.as_path(),
    )
    .into_iter()
    .map(|hit| (hit.name.to_ascii_lowercase(), hit.path, hit.line_start))
    .collect::<HashSet<_>>();

    let Ok(content) = std::fs::read_to_string(file.absolute_path.as_path()) else {
        return Vec::new();
    };
    let mut hits = Vec::new();
    for (line_idx, line_text) in content.lines().enumerate() {
        let line_number = line_idx + 1;
        let mut seen_tokens = HashSet::new();
        for mat in REFERENCE_TOKEN_PATTERN.find_iter(line_text) {
            let token = mat.as_str();
            let token_folded = token.to_ascii_lowercase();
            if !seen_tokens.insert(token_folded.clone()) {
                continue;
            }
            if definition_locations.contains(&(
                token_folded,
                file.normalized_path.clone(),
                line_number,
            )) {
                continue;
            }

            let column = line_text[..mat.start()].chars().count() + 1;
            hits.push(ReferenceSearchHit {
                name: token.to_string(),
                path: file.normalized_path.clone(),
                language: language.to_string(),
                crate_name: crate_name.clone(),
                project_name: metadata.project_name.clone(),
                root_label: metadata.root_label.clone(),
                navigation_target: reference_navigation_target(
                    file.normalized_path.as_str(),
                    crate_name.as_str(),
                    metadata.project_name.as_deref(),
                    metadata.root_label.as_deref(),
                    line_number,
                    column,
                ),
                line: line_number,
                column,
                line_text: line_text.trim().to_string(),
                score: 0.0,
            });
        }
    }
    hits
}

fn reference_scan_lang(path: &Path) -> Option<&'static str> {
    match Lang::from_path(path)? {
        Lang::Python => Some("python"),
        Lang::Rust => Some("rust"),
        Lang::JavaScript => Some("javascript"),
        Lang::TypeScript => Some("typescript"),
        Lang::Bash => Some("bash"),
        Lang::Go => Some("go"),
        Lang::Java => Some("java"),
        Lang::C => Some("c"),
        Lang::Cpp => Some("cpp"),
        Lang::CSharp => Some("csharp"),
        Lang::Ruby => Some("ruby"),
        Lang::Swift => Some("swift"),
        Lang::Kotlin => Some("kotlin"),
        Lang::Lua => Some("lua"),
        Lang::Php => Some("php"),
        _ => None,
    }
}

fn reference_navigation_target(
    path: &str,
    crate_name: &str,
    project_name: Option<&str>,
    root_label: Option<&str>,
    line: usize,
    column: usize,
) -> StudioNavigationTarget {
    StudioNavigationTarget {
        path: path.to_string(),
        category: "doc".to_string(),
        project_name: project_name
            .map(ToString::to_string)
            .or_else(|| Some(crate_name.to_string())),
        root_label: root_label.map(ToString::to_string),
        line: Some(line),
        line_end: Some(line),
        column: Some(column),
    }
}
