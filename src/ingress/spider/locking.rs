use std::hash::{Hash, Hasher};

const DEFAULT_INGEST_LOCK_SEGMENTS: usize = 64;

pub(super) fn default_ingest_lock_segments() -> usize {
    DEFAULT_INGEST_LOCK_SEGMENTS
}

pub(super) fn lock_slot_for_hash(content_hash: &str, lock_segments: usize) -> usize {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    content_hash.hash(&mut hasher);
    let segment_count = lock_segments.max(1);
    let segment_count_u64 = u64::try_from(segment_count).unwrap_or(u64::MAX);
    let slot_u64 = hasher.finish() % segment_count_u64;
    usize::try_from(slot_u64).unwrap_or_default()
}
