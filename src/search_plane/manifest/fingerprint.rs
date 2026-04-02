use serde::{Deserialize, Serialize};

/// Stable file-level fingerprint payload for incremental manifest updates.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchFileFingerprint {
    /// Repo-relative path for the source file.
    pub relative_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    /// Optional local partition identifier used to route incremental updates.
    pub partition_id: Option<String>,
    /// File size captured during manifest generation.
    pub size_bytes: u64,
    /// Modification time expressed as unix milliseconds.
    pub modified_unix_ms: u64,
    /// Extractor version that produced the manifest row.
    pub extractor_version: u32,
    /// Search-plane schema version associated with the row payload.
    pub schema_version: u32,
    /// Optional content hash used when metadata is insufficient.
    pub blake3: Option<String>,
}
