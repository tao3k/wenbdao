use serde::Deserialize;
use thiserror::Error;
use xiuxian_skills::{InternalSkillManifest, ToolAnnotations};

/// Canonical URI prefix for internal skill manifests.
pub const INTERNAL_SKILL_URI_PREFIX: &str = "wendao://skills-internal";

/// Supported internal workflow types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InternalSkillWorkflowType {
    /// Qianji engine flow definition.
    Qianji,
    /// Unknown or unsupported workflow type.
    Unknown(String),
}

impl InternalSkillWorkflowType {
    /// Parse workflow type from raw string.
    #[must_use]
    pub fn from_raw(raw: Option<&str>) -> Self {
        let normalized = raw.unwrap_or("qianji").trim().to_ascii_lowercase();
        match normalized.as_str() {
            "qianji" | "flow" | "workflow" => Self::Qianji,
            other => Self::Unknown(other.to_string()),
        }
    }
}

/// Authority report for internal skill manifest discovery.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InternalSkillAuthorityReport {
    /// Manifests explicitly authorized by `SKILL.md` links.
    pub authorized_manifests: Vec<String>,
    /// `SKILL.md` links pointing to missing physical manifests.
    pub ghost_links: Vec<String>,
    /// Physical manifests not granted by any `SKILL.md`.
    pub unauthorized_manifests: Vec<String>,
}

/// Authority resolution output.
#[derive(Debug, Clone)]
pub struct InternalSkillAuthorityOutcome {
    /// Detailed classification report.
    pub report: InternalSkillAuthorityReport,
    /// Successfully loaded authorized manifest objects.
    pub authorized: Vec<InternalSkillManifest>,
}

/// Error type for internal skill manifest operations.
#[derive(Debug, Error)]
pub enum InternalSkillManifestError {
    /// File read failure.
    #[error("failed to read internal skill manifest {path}: {source}")]
    Io {
        /// Source file path.
        path: String,
        /// Underlying I/O error.
        source: std::io::Error,
    },
    /// TOML parsing or validation failure.
    #[error("failed to parse internal skill manifest {path}: {reason}")]
    Toml {
        /// Source file path.
        path: String,
        /// Human-readable reason for failure.
        reason: String,
    },
    /// Required manifest field is missing.
    #[error("internal skill manifest missing required field `{field}` at {path}")]
    MissingField {
        /// Source file path.
        path: String,
        /// Name of the missing field.
        field: String,
    },
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct ToolAnnotationsOverride {
    #[serde(default)]
    read_only: Option<bool>,
    #[serde(default)]
    destructive: Option<bool>,
    #[serde(default)]
    idempotent: Option<bool>,
    #[serde(default)]
    open_world: Option<bool>,
}

impl ToolAnnotationsOverride {
    pub(super) fn apply_defaults(self) -> ToolAnnotations {
        let mut annotations = ToolAnnotations {
            read_only: false,
            destructive: true,
            ..ToolAnnotations::default()
        };
        annotations.set_idempotent(false);
        annotations.set_open_world(true);
        if let Some(value) = self.read_only {
            annotations.read_only = value;
        }
        if let Some(value) = self.destructive {
            annotations.destructive = value;
        }
        if let Some(value) = self.idempotent {
            annotations.set_idempotent(value);
        }
        if let Some(value) = self.open_world {
            annotations.set_open_world(value);
        }
        annotations
    }
}

#[derive(Debug, Deserialize, Default)]
pub(super) struct InternalSkillManifestToml {
    #[serde(default)]
    pub(super) manifest_id: Option<String>,
    #[serde(default)]
    pub(super) id: Option<String>,
    #[serde(default)]
    pub(super) name: Option<String>,
    #[serde(default)]
    pub(super) tool_name: Option<String>,
    #[serde(default)]
    pub(super) internal_id: Option<String>,
    #[serde(default)]
    pub(super) description: Option<String>,
    #[serde(default)]
    pub(super) tool_contract: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) contract: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) workflow_type: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) workflow: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) qianhuan: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) qianhuan_background: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) background: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) flow_definition: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) flow: Option<serde_json::Value>,
    #[serde(default)]
    pub(super) annotations: Option<ToolAnnotationsOverride>,
    #[serde(default)]
    pub(super) tool_annotations: Option<ToolAnnotationsOverride>,
}
