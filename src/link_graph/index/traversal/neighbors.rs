use super::super::{LinkGraphDirection, LinkGraphIndex, LinkGraphNeighbor};
use super::merge_direction;
use std::collections::{HashMap, HashSet, VecDeque};

impl LinkGraphIndex {
    /// Traverse neighbors for a note stem/id/path.
    #[must_use]
    pub fn neighbors(
        &self,
        stem_or_id: &str,
        direction: LinkGraphDirection,
        hops: usize,
        limit: usize,
    ) -> Vec<LinkGraphNeighbor> {
        self.neighbors_with_overlay(stem_or_id, direction, hops, limit)
            .0
    }

    /// Traverse neighbors for a note stem/id/path and include promoted overlay telemetry.
    #[must_use]
    pub fn neighbors_with_overlay(
        &self,
        stem_or_id: &str,
        direction: LinkGraphDirection,
        hops: usize,
        limit: usize,
    ) -> (
        Vec<LinkGraphNeighbor>,
        super::super::LinkGraphPromotedOverlayTelemetry,
    ) {
        let (overlay, telemetry) = self.promoted_overlay_telemetry();
        let rows = if let Some(overlay) = overlay {
            overlay.neighbors_core(stem_or_id, direction, hops, limit)
        } else {
            self.neighbors_core(stem_or_id, direction, hops, limit)
        };
        (rows, telemetry)
    }

    fn neighbors_core(
        &self,
        stem_or_id: &str,
        direction: LinkGraphDirection,
        hops: usize,
        limit: usize,
    ) -> Vec<LinkGraphNeighbor> {
        let Some(start_id) = self.resolve_doc_id(stem_or_id).map(str::to_string) else {
            return Vec::new();
        };

        let max_hops = hops.max(1);
        let max_items = limit.max(1);

        let mut queue: VecDeque<(String, usize, LinkGraphDirection)> = VecDeque::new();
        queue.push_back((start_id.clone(), 0, LinkGraphDirection::Both));
        let mut visited: HashSet<String> = HashSet::new();
        visited.insert(start_id.clone());

        let mut neighbors: HashMap<String, LinkGraphNeighbor> = HashMap::new();

        while let Some((current_id, depth, root_direction)) = queue.pop_front() {
            if depth >= max_hops {
                continue;
            }

            let mut next_nodes: Vec<(String, LinkGraphDirection)> = Vec::new();
            if matches!(
                direction,
                LinkGraphDirection::Both | LinkGraphDirection::Outgoing
            ) && let Some(targets) = self.outgoing.get(&current_id)
            {
                for target in targets {
                    let effective = if depth == 0 {
                        LinkGraphDirection::Outgoing
                    } else {
                        root_direction
                    };
                    next_nodes.push((target.clone(), effective));
                }
            }
            if matches!(
                direction,
                LinkGraphDirection::Both | LinkGraphDirection::Incoming
            ) && let Some(sources) = self.incoming.get(&current_id)
            {
                for source in sources {
                    let effective = if depth == 0 {
                        LinkGraphDirection::Incoming
                    } else {
                        root_direction
                    };
                    next_nodes.push((source.clone(), effective));
                }
            }

            for (next_id, next_direction) in next_nodes {
                if next_id == start_id {
                    continue;
                }
                let Some(doc) = self.docs_by_id.get(&next_id) else {
                    continue;
                };
                let distance = depth + 1;
                if let Some(existing) = neighbors.get_mut(&next_id) {
                    existing.distance = existing.distance.min(distance);
                    existing.direction = merge_direction(existing.direction, next_direction);
                } else {
                    neighbors.insert(
                        next_id.clone(),
                        LinkGraphNeighbor {
                            stem: doc.stem.clone(),
                            direction: next_direction,
                            distance,
                            title: doc.title.clone(),
                            path: doc.path.clone(),
                        },
                    );
                }
                if distance < max_hops && !visited.contains(&next_id) {
                    visited.insert(next_id.clone());
                    queue.push_back((next_id, distance, next_direction));
                }
            }
        }

        let mut out: Vec<LinkGraphNeighbor> = neighbors.into_values().collect();
        out.sort_by(|a, b| a.distance.cmp(&b.distance).then(a.path.cmp(&b.path)));
        out.truncate(max_items);
        out
    }
}
