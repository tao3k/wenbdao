use blake3::Hasher;

/// Compute Blake3 hash for block content (truncated to 16 hex chars).
pub(crate) fn compute_block_hash(content: &str) -> String {
    let mut hasher = Hasher::new();
    hasher.update(content.as_bytes());
    let hash = hasher.finalize();
    hash.to_hex()[..16].to_string()
}
