pub(crate) fn infer_ecosystem(repo_id: &str) -> &'static str {
    let lower = repo_id.to_ascii_lowercase();
    if lower.contains("sciml") || lower.contains("diffeq") {
        "sciml"
    } else if lower.contains("modelica") || lower == "msl" {
        "msl"
    } else {
        "unknown"
    }
}
