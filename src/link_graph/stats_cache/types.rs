use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LinkGraphStatsCacheStats {
    pub(super) total_notes: i64,
    pub(super) orphans: i64,
    pub(super) links_in_graph: i64,
    pub(super) nodes_in_graph: i64,
}

impl LinkGraphStatsCacheStats {
    pub(super) fn normalize(self) -> Self {
        Self {
            total_notes: self.total_notes.max(0),
            orphans: self.orphans.max(0),
            links_in_graph: self.links_in_graph.max(0),
            nodes_in_graph: self.nodes_in_graph.max(0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct LinkGraphStatsCachePayload {
    pub(super) schema: String,
    pub(super) source_key: String,
    pub(super) updated_at_unix: f64,
    pub(super) stats: LinkGraphStatsCacheStats,
}
