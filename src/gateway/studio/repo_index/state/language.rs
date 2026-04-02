use std::path::Path;

pub(super) fn is_supported_code_path(path: &str) -> bool {
    has_code_extension(path, &["jl", "julia", "mo", "modelica"])
}

pub(super) fn is_excluded_code_path(path: &str) -> bool {
    [
        ".git/",
        ".cache/",
        ".devenv/",
        ".direnv/",
        "node_modules/",
        "target/",
        "dist/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

pub(super) fn infer_code_language(path: &str) -> Option<String> {
    if has_code_extension(path, &["jl", "julia"]) {
        return Some("julia".to_string());
    }
    if has_code_extension(path, &["mo", "modelica"]) {
        return Some("modelica".to_string());
    }
    None
}

fn has_code_extension(path: &str, extensions: &[&str]) -> bool {
    Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| {
            extensions
                .iter()
                .any(|candidate| ext.eq_ignore_ascii_case(candidate))
        })
}
