//! Internal skill manifest loading and authority resolution helpers.

mod authority;
mod load;
mod types;

#[cfg(test)]
mod tests;

pub use authority::resolve_internal_skill_authority;
pub use load::load_internal_skill_manifest_from_path;
pub use types::{
    INTERNAL_SKILL_URI_PREFIX, InternalSkillAuthorityOutcome, InternalSkillAuthorityReport,
    InternalSkillManifestError, InternalSkillWorkflowType,
};
pub use xiuxian_skills::InternalSkillManifest;
