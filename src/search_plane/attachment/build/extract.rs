use std::collections::HashSet;
use std::path::Path;

use crate::gateway::studio::search::project_scope::project_metadata_for_path;
use crate::gateway::studio::types::{AttachmentSearchHit, StudioNavigationTarget, UiProjectConfig};
use crate::link_graph::LinkGraphAttachmentKind;
use crate::link_graph::parser::{is_supported_note, parse_note};
use crate::search_plane::ProjectScannedFile;

pub(crate) fn build_attachment_hits_for_files(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    files: &[ProjectScannedFile],
) -> Vec<AttachmentSearchHit> {
    let mut hits = Vec::new();
    for file in files {
        hits.extend(build_attachment_hits_for_file(
            project_root,
            config_root,
            projects,
            file,
        ));
    }
    hits
}

pub(crate) fn build_attachment_hits_for_file(
    project_root: &Path,
    config_root: &Path,
    projects: &[UiProjectConfig],
    file: &ProjectScannedFile,
) -> Vec<AttachmentSearchHit> {
    if !is_supported_note(file.absolute_path.as_path()) {
        return Vec::new();
    }

    let Ok(content) = std::fs::read_to_string(file.absolute_path.as_path()) else {
        return Vec::new();
    };
    let Some(parsed) = parse_note(file.absolute_path.as_path(), project_root, &content) else {
        return Vec::new();
    };
    let metadata = project_metadata_for_path(
        project_root,
        config_root,
        projects,
        parsed.doc.path.as_str(),
    );
    attachment_hits_for_parsed_note(
        &parsed,
        metadata.project_name.as_deref(),
        metadata.root_label.as_deref(),
    )
}

fn attachment_hits_for_parsed_note(
    parsed: &crate::link_graph::parser::ParsedNote,
    project_name: Option<&str>,
    root_label: Option<&str>,
) -> Vec<AttachmentSearchHit> {
    let mut seen = HashSet::<String>::new();
    let mut hits = parsed
        .attachment_targets
        .iter()
        .filter(|attachment_path| seen.insert((*attachment_path).clone()))
        .map(|attachment_path| {
            let attachment_name = attachment_name(attachment_path);
            let attachment_ext = attachment_ext(attachment_path);
            AttachmentSearchHit {
                name: attachment_name.clone(),
                path: parsed.doc.path.clone(),
                source_id: parsed.doc.id.clone(),
                source_stem: parsed.doc.stem.clone(),
                source_title: parsed.doc.title.clone(),
                source_path: parsed.doc.path.clone(),
                attachment_id: format!("att://{}/{}", parsed.doc.id, attachment_path),
                attachment_path: attachment_path.clone(),
                attachment_name,
                attachment_ext: attachment_ext.clone(),
                kind: attachment_kind_label(LinkGraphAttachmentKind::from_extension(
                    attachment_ext.as_str(),
                ))
                .to_string(),
                navigation_target: StudioNavigationTarget {
                    path: parsed.doc.path.clone(),
                    category: "doc".to_string(),
                    project_name: project_name.map(ToString::to_string),
                    root_label: root_label.map(ToString::to_string),
                    line: None,
                    line_end: None,
                    column: None,
                },
                score: 0.0,
                vision_snippet: None,
            }
        })
        .collect::<Vec<_>>();
    hits.sort_by(|left, right| {
        left.attachment_path
            .cmp(&right.attachment_path)
            .then(left.source_path.cmp(&right.source_path))
    });
    hits
}

fn attachment_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map_or_else(|| path.to_string(), ToString::to_string)
}

fn attachment_ext(path: &str) -> String {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().trim_start_matches('.').to_ascii_lowercase())
        .unwrap_or_default()
}

pub(crate) fn attachment_kind_label(kind: LinkGraphAttachmentKind) -> &'static str {
    match kind {
        LinkGraphAttachmentKind::Image => "image",
        LinkGraphAttachmentKind::Pdf => "pdf",
        LinkGraphAttachmentKind::Gpg => "gpg",
        LinkGraphAttachmentKind::Document => "document",
        LinkGraphAttachmentKind::Archive => "archive",
        LinkGraphAttachmentKind::Audio => "audio",
        LinkGraphAttachmentKind::Video => "video",
        LinkGraphAttachmentKind::Other => "other",
    }
}
