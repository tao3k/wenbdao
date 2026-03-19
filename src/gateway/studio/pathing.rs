use std::path::{Path, PathBuf};

use globset::{Glob, GlobMatcher};
use regex::Regex;

use super::types::UiProjectConfig;

const DIR_REGEX_PREFIXES: [&str; 2] = ["re:", "regex:"];
const DIR_GLOB_PREFIXES: [&str; 2] = ["glob:", "globset:"];

#[derive(Debug, Clone)]
pub(crate) enum ProjectFileFilter {
    IncludeRegex(Regex),
    IncludeGlob(GlobMatcher),
    ExcludeGlob(GlobMatcher),
}

/// Normalize a path-like value from configuration while preserving semantic markers.
pub(crate) fn normalize_path_like(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "." || trimmed == "~" {
        return Some(trimmed.to_string());
    }
    let normalized = trimmed
        .replace('\\', "/")
        .trim_end_matches('/')
        .trim_start_matches("./")
        .to_string();
    if normalized.is_empty() {
        None
    } else {
        Some(normalized)
    }
}

fn dir_regex_pattern(raw: &str) -> Option<&str> {
    let trimmed = raw.trim();
    for prefix in DIR_REGEX_PREFIXES {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            let pattern = rest.trim();
            if !pattern.is_empty() {
                return Some(pattern);
            }
            return None;
        }
    }
    None
}

fn dir_glob_pattern(raw: &str) -> Option<(bool, String)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (outer_included, body) = if let Some(rest) = trimmed.strip_prefix('!') {
        (false, rest.trim())
    } else {
        (true, trimmed)
    };
    if body.is_empty() {
        return None;
    }

    for prefix in DIR_GLOB_PREFIXES {
        if let Some(rest) = body.strip_prefix(prefix) {
            let mut pattern = rest.trim().replace('\\', "/");
            if pattern.is_empty() {
                return None;
            }
            let inner_excluded = pattern.starts_with('!');
            if inner_excluded {
                pattern = pattern.trim_start_matches('!').trim().to_string();
            }
            if pattern.is_empty() {
                return None;
            }
            return Some((outer_included && !inner_excluded, pattern));
        }
    }

    if contains_glob_magic(body) {
        return Some((outer_included, body.replace('\\', "/")));
    }

    None
}

fn contains_glob_magic(value: &str) -> bool {
    value.contains('*')
        || value.contains('?')
        || value.contains('[')
        || value.contains(']')
        || value.contains('{')
        || value.contains('}')
}

/// Normalize one project `dirs` entry.
///
/// - `re:<pattern>` / `regex:<pattern>` are canonicalized to `re:<pattern>`
/// - glob-style patterns are canonicalized to `glob:<pattern>` or
///   `glob:!<pattern>`
/// - path-like values are normalized by [`normalize_path_like`]
pub(crate) fn normalize_project_dir_entry(raw: &str) -> Option<String> {
    if let Some(pattern) = dir_regex_pattern(raw) {
        return Some(format!("re:{pattern}"));
    }
    if let Some((included, pattern)) = dir_glob_pattern(raw) {
        if included {
            return Some(format!("glob:{pattern}"));
        }
        return Some(format!("glob:!{pattern}"));
    }
    normalize_path_like(raw)
}

/// Normalize one project `dirs` entry as a concrete directory root.
///
/// Filter entries (`re:`, `glob:`, wildcard forms, negated wildcard forms) are
/// ignored by returning `None`.
pub(crate) fn normalize_project_dir_root(raw: &str) -> Option<String> {
    if dir_regex_pattern(raw).is_some() || dir_glob_pattern(raw).is_some() {
        return None;
    }
    normalize_path_like(raw)
}

/// Compile one project `dirs` entry as a file filter.
///
/// Non-filter entries or invalid patterns return `None`.
pub(crate) fn compile_project_dir_filter(raw: &str) -> Option<ProjectFileFilter> {
    if let Some(pattern) = dir_regex_pattern(raw) {
        return Regex::new(pattern)
            .ok()
            .map(ProjectFileFilter::IncludeRegex);
    }

    let (included, pattern) = dir_glob_pattern(raw)?;
    let matcher = Glob::new(pattern.as_str()).ok()?.compile_matcher();
    if included {
        Some(ProjectFileFilter::IncludeGlob(matcher))
    } else {
        Some(ProjectFileFilter::ExcludeGlob(matcher))
    }
}

/// Evaluate file filters against a project-relative path.
///
/// Rules:
/// - Exclude filters always deny on match.
/// - If any include filters exist, at least one must match.
/// - If no include filters exist, allow by default.
pub(crate) fn matches_project_file_filters(path: &str, filters: &[ProjectFileFilter]) -> bool {
    let mut has_include = false;
    let mut include_match = false;

    for filter in filters {
        match filter {
            ProjectFileFilter::ExcludeGlob(matcher) => {
                if matcher.is_match(path) {
                    return false;
                }
            }
            ProjectFileFilter::IncludeRegex(regex) => {
                has_include = true;
                if regex.is_match(path) {
                    include_match = true;
                }
            }
            ProjectFileFilter::IncludeGlob(matcher) => {
                has_include = true;
                if matcher.is_match(path) {
                    include_match = true;
                }
            }
        }
    }

    if has_include { include_match } else { true }
}

/// Check whether `hit_path` (absolute or project-relative) satisfies configured
/// project file filters.
///
/// Behavior:
/// - If a matched project has no filter entries, the path is allowed.
/// - If a matched project has filter entries, the path is evaluated by
///   [`matches_project_file_filters`].
/// - If no project matches the path, allow by default.
pub(crate) fn path_matches_project_file_filters(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    hit_path: &str,
) -> bool {
    let absolute_hit = if Path::new(hit_path).is_absolute() {
        PathBuf::from(hit_path)
    } else {
        project_root.join(hit_path)
    };

    let mut matched_project = false;

    for project in projects {
        let Some(project_root_path) = resolve_path_like(config_root, project.root.as_str()) else {
            continue;
        };
        if !path_within_scope(absolute_hit.as_path(), project_root_path.as_path()) {
            continue;
        }

        matched_project = true;
        let filters = project
            .dirs
            .iter()
            .filter_map(|entry| compile_project_dir_filter(entry.as_str()))
            .collect::<Vec<_>>();
        if filters.is_empty() {
            return true;
        }

        let Ok(relative) = absolute_hit.strip_prefix(project_root_path.as_path()) else {
            continue;
        };
        let normalized = relative.to_string_lossy().replace('\\', "/");
        if matches_project_file_filters(normalized.as_str(), filters.as_slice()) {
            return true;
        }
    }

    !matched_project
}

/// Resolve a normalized path-like config value against `base`.
///
/// Resolution order:
/// - `~` / `~/...` -> user home
/// - absolute path -> as-is
/// - `.` -> `base`
/// - relative path -> `base.join(path)`
pub(crate) fn resolve_path_like(base: &Path, raw: &str) -> Option<PathBuf> {
    let normalized = normalize_path_like(raw)?;
    Some(resolve_normalized_path_like(base, normalized.as_str()))
}

fn resolve_normalized_path_like(base: &Path, normalized: &str) -> PathBuf {
    if normalized == "." {
        return base.to_path_buf();
    }
    if let Some(home_expanded) = expand_tilde_path(normalized) {
        return home_expanded;
    }
    let candidate = Path::new(normalized);
    if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        base.join(candidate)
    }
}

fn expand_tilde_path(value: &str) -> Option<PathBuf> {
    if value == "~" {
        return dirs::home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(rest));
    }
    None
}

fn path_within_scope(path: &Path, scope: &Path) -> bool {
    path == scope || path.strip_prefix(scope).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_path_like_preserves_tilde_roots() {
        assert_eq!(normalize_path_like("~"), Some("~".to_string()));
        assert_eq!(
            normalize_path_like("~/ghq/github.com"),
            Some("~/ghq/github.com".to_string())
        );
    }

    #[test]
    fn normalize_path_like_cleans_relative_and_windows_values() {
        assert_eq!(normalize_path_like("./docs/"), Some("docs".to_string()));
        assert_eq!(
            normalize_path_like(".\\data\\knowledge\\"),
            Some("data/knowledge".to_string())
        );
    }

    #[test]
    fn normalize_project_dir_entry_preserves_regex_rules() {
        assert_eq!(
            normalize_project_dir_entry(r"re:^docs/[^/]+\.md$"),
            Some(r"re:^docs/[^/]+\.md$".to_string())
        );
        assert_eq!(
            normalize_project_dir_entry(r"regex:^docs/.+\.md$"),
            Some(r"re:^docs/.+\.md$".to_string())
        );
    }

    #[test]
    fn normalize_project_dir_entry_canonicalizes_glob_rules() {
        assert_eq!(
            normalize_project_dir_entry("**/*.md"),
            Some("glob:**/*.md".to_string())
        );
        assert_eq!(
            normalize_project_dir_entry("!**/private/**"),
            Some("glob:!**/private/**".to_string())
        );
        assert_eq!(
            normalize_project_dir_entry("glob:**/*.markdown"),
            Some("glob:**/*.markdown".to_string())
        );
    }

    #[test]
    fn normalize_project_dir_root_ignores_filter_entries() {
        assert_eq!(normalize_project_dir_root(r"re:^docs/.+\.md$"), None);
        assert_eq!(normalize_project_dir_root("**/*.md"), None);
        assert_eq!(normalize_project_dir_root("!**/private/**"), None);
        assert_eq!(normalize_project_dir_root("docs"), Some("docs".to_string()));
    }

    #[test]
    fn compile_project_dir_filter_handles_re_and_glob_prefixes() {
        let Some(ProjectFileFilter::IncludeRegex(regex)) =
            compile_project_dir_filter(r"re:^docs/.+\.md$")
        else {
            panic!("expected regex filter to compile");
        };
        assert!(regex.is_match("docs/guide.md"));
        assert!(!regex.is_match("internal_skills/writer/skill.md"));

        let Some(ProjectFileFilter::IncludeGlob(include_glob)) =
            compile_project_dir_filter("**/*.md")
        else {
            panic!("expected include glob filter to compile");
        };
        assert!(include_glob.is_match("docs/guide.md"));

        let Some(ProjectFileFilter::ExcludeGlob(exclude_glob)) =
            compile_project_dir_filter("!**/private/**")
        else {
            panic!("expected exclude glob filter to compile");
        };
        assert!(exclude_glob.is_match("docs/private/secret.md"));
        let Some(ProjectFileFilter::ExcludeGlob(exclude_glob_with_prefix)) =
            compile_project_dir_filter("glob:!**/private/**")
        else {
            panic!("expected explicit exclude glob filter to compile");
        };
        assert!(exclude_glob_with_prefix.is_match("docs/private/secret.md"));
    }

    #[test]
    fn matches_project_file_filters_applies_include_and_exclude() {
        let filters = vec![
            compile_project_dir_filter("**/*.md")
                .unwrap_or_else(|| panic!("expected include filter")),
            compile_project_dir_filter("!**/private/**")
                .unwrap_or_else(|| panic!("expected exclude filter")),
        ];
        assert!(matches_project_file_filters(
            "docs/guide.md",
            filters.as_slice()
        ));
        assert!(!matches_project_file_filters(
            "docs/private/secret.md",
            filters.as_slice()
        ));
        assert!(!matches_project_file_filters(
            "docs/guide.rs",
            filters.as_slice()
        ));
    }

    #[test]
    fn resolve_path_like_maps_relative_and_dot_to_base() {
        let base = Path::new("/tmp/kernel");
        assert_eq!(resolve_path_like(base, ".").as_deref(), Some(base));
        assert_eq!(resolve_path_like(base, "docs"), Some(base.join("docs")));
    }

    #[test]
    fn resolve_path_like_maps_tilde_to_home_dir() {
        let Some(home) = dirs::home_dir() else {
            return;
        };
        let base = Path::new("/tmp/kernel");
        assert_eq!(resolve_path_like(base, "~"), Some(home.clone()));
        assert_eq!(
            resolve_path_like(base, "~/ghq/github.com"),
            Some(home.join("ghq/github.com"))
        );
    }
}
