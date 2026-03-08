use super::filters::should_skip_entry;
use crate::link_graph::parser::is_supported_note;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct LinkGraphFingerprint {
    pub note_count: usize,
    pub latest_modified_ts: Option<i64>,
    pub total_size_bytes: u64,
}

fn system_time_to_unix(ts: SystemTime) -> Option<i64> {
    let seconds = ts.duration_since(UNIX_EPOCH).ok()?.as_secs();
    i64::try_from(seconds).ok()
}

fn update_fingerprint(path: &Path, fingerprint: &mut LinkGraphFingerprint) {
    let Ok(meta) = std::fs::metadata(path) else {
        return;
    };
    fingerprint.note_count = fingerprint.note_count.saturating_add(1);
    fingerprint.total_size_bytes = fingerprint.total_size_bytes.saturating_add(meta.len());
    let modified = meta.modified().ok().and_then(system_time_to_unix);
    if let Some(ts) = modified {
        fingerprint.latest_modified_ts =
            Some(fingerprint.latest_modified_ts.map_or(ts, |v| v.max(ts)));
    }
}

pub(super) fn scan_note_fingerprint(
    root: &Path,
    include_dirs: &HashSet<String>,
    excluded_dirs: &HashSet<String>,
) -> LinkGraphFingerprint {
    let mut fingerprint = LinkGraphFingerprint::default();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !should_skip_entry(
                entry.path(),
                entry.file_type().is_dir(),
                root,
                include_dirs,
                excluded_dirs,
            )
        })
        .filter_map(Result::ok)
    {
        let path = entry.path();
        if !entry.file_type().is_file() || !is_supported_note(path) {
            continue;
        }
        update_fingerprint(path, &mut fingerprint);
    }
    fingerprint
}
