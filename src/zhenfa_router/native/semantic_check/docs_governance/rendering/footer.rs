/// Render a normalized index `:FOOTER:` block with explicit standards and sync values.
#[must_use]
pub fn render_index_footer_with_values(standards: &str, last_sync: &str) -> String {
    format!(":FOOTER:\n:STANDARDS: {standards}\n:LAST_SYNC: {last_sync}\n:END:\n")
}
