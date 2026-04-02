/// Compute Blake3 hash of content (one-time verification).
pub(super) fn compute_blake3_hash(content: &str) -> String {
    blake3::hash(content.as_bytes()).to_hex().to_string()
}
