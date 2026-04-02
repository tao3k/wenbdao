/// Build a concise document description from the washed content and optional title.
pub(super) fn build_document_description(title: Option<&str>, washed_markdown: &str) -> String {
    let first_non_empty = washed_markdown
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("Ingested web document");
    let snippet = first_non_empty.chars().take(220).collect::<String>();
    match title {
        Some(title) if !title.trim().is_empty() => format!("{title}: {snippet}"),
        _ => snippet,
    }
}
