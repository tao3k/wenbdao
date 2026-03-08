use serde::{Deserialize, Serialize};

/// Attachment kind classification inferred from file extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LinkGraphAttachmentKind {
    /// Image attachments (`png`, `jpg`, `svg`, ...).
    Image,
    /// PDF attachments.
    Pdf,
    /// GPG/PGP/key material attachments.
    Gpg,
    /// Generic document attachments (`docx`, `txt`, `epub`, ...).
    Document,
    /// Compressed/archive attachments.
    Archive,
    /// Audio attachments.
    Audio,
    /// Video attachments.
    Video,
    /// Unclassified attachment type.
    Other,
}

impl LinkGraphAttachmentKind {
    /// Parse kind alias from CLI/runtime input.
    #[must_use]
    pub fn from_alias(raw: &str) -> Self {
        match raw.trim().to_lowercase().as_str() {
            "image" | "img" => Self::Image,
            "pdf" => Self::Pdf,
            "gpg" | "pgp" | "key" => Self::Gpg,
            "doc" | "document" => Self::Document,
            "archive" | "compressed" => Self::Archive,
            "audio" | "sound" => Self::Audio,
            "video" => Self::Video,
            _ => Self::Other,
        }
    }

    /// Infer attachment kind from extension (without leading dot).
    #[must_use]
    pub fn from_extension(ext: &str) -> Self {
        let normalized = ext.trim().trim_start_matches('.').to_lowercase();
        match normalized.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "svg" | "bmp" | "ico" | "avif" | "tif"
            | "tiff" | "heic" | "heif" => Self::Image,
            "pdf" => Self::Pdf,
            "gpg" | "pgp" | "asc" | "sig" | "key" => Self::Gpg,
            "zip" | "tar" | "gz" | "bz2" | "xz" | "7z" | "rar" | "zst" => Self::Archive,
            "mp3" | "wav" | "flac" | "aac" | "ogg" | "m4a" | "opus" => Self::Audio,
            "mp4" | "mkv" | "mov" | "avi" | "webm" | "m4v" => Self::Video,
            "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "txt" | "rtf" | "odt" | "ods"
            | "odp" | "epub" => Self::Document,
            _ => Self::Other,
        }
    }
}

/// One normalized attachment reference extracted from a markdown note.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphAttachment {
    /// Source document id (relative path without extension).
    pub source_id: String,
    /// Source note stem.
    pub source_stem: String,
    /// Source note path with extension.
    pub source_path: String,
    /// Source note title.
    pub source_title: String,
    /// Normalized attachment path/target.
    pub attachment_path: String,
    /// Attachment basename.
    pub attachment_name: String,
    /// Lowercased extension without leading dot.
    pub attachment_ext: String,
    /// Attachment kind inferred from extension.
    pub kind: LinkGraphAttachmentKind,
}

/// Attachment search hit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkGraphAttachmentHit {
    /// Source note stem.
    pub source_stem: String,
    /// Source note title.
    pub source_title: String,
    /// Source note path with extension.
    pub source_path: String,
    /// Normalized attachment path/target.
    pub attachment_path: String,
    /// Attachment basename.
    pub attachment_name: String,
    /// Lowercased extension without leading dot.
    pub attachment_ext: String,
    /// Attachment kind inferred from extension.
    pub kind: LinkGraphAttachmentKind,
    /// Search relevance score (0-1).
    pub score: f64,
}
