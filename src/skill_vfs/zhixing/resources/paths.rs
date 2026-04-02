use include_dir::Dir;

/// Embedded skill document path relative to the `resources/` root.
pub const ZHIXING_SKILL_DOC_PATH: &str = "zhixing/skills/agenda-management/SKILL.md";
pub(crate) const ZHIXING_EMBEDDED_CRATE_ID: &str = "xiuxian-zhixing";

static EMBEDDED_ZHIXING_RESOURCES: Dir<'_> =
    include_dir::include_dir!("$CARGO_MANIFEST_DIR/resources");

#[must_use]
pub(crate) fn embedded_resource_dir() -> &'static Dir<'static> {
    &EMBEDDED_ZHIXING_RESOURCES
}

#[must_use]
pub(crate) fn normalize_embedded_resource_path(path: &str) -> String {
    path.trim().trim_start_matches("./").replace('\\', "/")
}
