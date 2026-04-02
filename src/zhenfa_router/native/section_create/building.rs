use std::fmt::Write;

use crate::zhenfa_router::native::section_create::types::BuildSectionOptions;

/// Build new sections with optional ID generation.
#[must_use]
pub(crate) fn build_new_sections_content_with_options(
    remaining_path: &[String],
    start_level: usize,
    content: &str,
    options: &BuildSectionOptions,
) -> String {
    let mut result = String::new();
    let mut current_level = start_level;

    for (i, heading) in remaining_path.iter().enumerate() {
        let level = current_level.clamp(1, 6);
        let heading_marker = "#".repeat(level);

        if i > 0 {
            result.push('\n');
        }
        let _ = write!(result, "{heading_marker} {heading}");

        // Add :ID: property drawer if requested
        if options.generate_id {
            let id = generate_section_id(options.id_prefix.as_deref());
            let _ = write!(result, "\n:ID: {id}");
        }

        result.push_str("\n\n");
        current_level += 1;
    }

    result.push_str(content);
    result.push('\n');

    result
}

/// Generate a section ID: either prefixed or plain UUID.
#[must_use]
pub(crate) fn generate_section_id(prefix: Option<&str>) -> String {
    let uuid = uuid::Uuid::new_v4();
    let uuid_str = uuid.simple().to_string();

    match prefix {
        Some(p) => format!("{}-{}", p, &uuid_str[..8]),
        None => uuid_str[..12].to_string(),
    }
}

/// Compute Blake3 hash truncated to 16 hex characters.
#[must_use]
pub(crate) fn compute_content_hash(content: &str) -> String {
    use blake3::Hasher;
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..16].to_string()
}
