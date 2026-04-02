use include_dir::{Dir, File};
use std::path::Path;

use crate::WendaoResourceUri;

pub(crate) fn is_markdown_file(path: &str) -> bool {
    matches!(
        path.rsplit('.').next().map(str::to_ascii_lowercase),
        Some(ext) if ext == "md" || ext == "markdown"
    )
}

pub(crate) fn normalize_registry_key(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}

pub(crate) fn is_wendao_uri(target: &str) -> bool {
    WendaoResourceUri::parse(target).is_ok()
}

pub(crate) fn collect_embedded_markdown_files<'a>(dir: &'a Dir<'a>, out: &mut Vec<&'a File<'a>>) {
    for file in dir.files() {
        let path = file.path().to_string_lossy().replace('\\', "/");
        if is_markdown_file(path.as_str()) {
            out.push(file);
        }
    }
    for child in dir.dirs() {
        collect_embedded_markdown_files(child, out);
    }
}

pub(crate) fn semantic_skill_name_from_descriptor(path: &str, markdown: &str) -> Option<String> {
    if !Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md") || name == "skill.md")
    {
        return None;
    }
    crate::enhancer::parse_frontmatter(markdown)
        .name
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
}
