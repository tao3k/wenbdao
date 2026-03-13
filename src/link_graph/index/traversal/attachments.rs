use crate::link_graph::{LinkGraphAttachmentHit, LinkGraphAttachmentKind, LinkGraphIndex};
use std::collections::HashSet;

impl LinkGraphIndex {
    /// Search extracted local attachments by query, extension, and kind filters.
    #[must_use]
    pub fn search_attachments(
        &self,
        query: &str,
        limit: usize,
        extensions: &[String],
        kinds: &[LinkGraphAttachmentKind],
        case_sensitive: bool,
    ) -> Vec<LinkGraphAttachmentHit> {
        let bounded_limit = limit.max(1);
        let query = query.trim();
        let normalized_query = if case_sensitive {
            query.to_string()
        } else {
            query.to_lowercase()
        };
        let query_tokens: Vec<String> = normalized_query
            .split_whitespace()
            .map(ToString::to_string)
            .collect();
        let ext_filters: HashSet<String> = extensions
            .iter()
            .map(|value| value.trim().trim_start_matches('.').to_lowercase())
            .filter(|value| !value.is_empty())
            .collect();
        let kind_filters: HashSet<LinkGraphAttachmentKind> = kinds.iter().copied().collect();

        let mut out: Vec<LinkGraphAttachmentHit> = Vec::new();
        for rows in self.attachments_by_doc.values() {
            for row in rows {
                if !ext_filters.is_empty() && !ext_filters.contains(&row.attachment_ext) {
                    continue;
                }
                if !kind_filters.is_empty() && !kind_filters.contains(&row.kind) {
                    continue;
                }

                let mut fields = vec![
                    row.attachment_path.clone(),
                    row.attachment_name.clone(),
                    row.source_path.clone(),
                    row.source_title.clone(),
                    row.source_stem.clone(),
                ];
                if !case_sensitive {
                    for value in &mut fields {
                        *value = value.to_lowercase();
                    }
                }

                let score = if normalized_query.is_empty() {
                    1.0
                } else {
                    let query_hit = fields
                        .iter()
                        .any(|value| value.contains(normalized_query.as_str()));
                    let token_hit_count = query_tokens
                        .iter()
                        .filter(|token| fields.iter().any(|value| value.contains(token.as_str())))
                        .count();
                    if !query_hit && token_hit_count == 0 {
                        continue;
                    }
                    let exact_name = if fields[1] == normalized_query {
                        1.0
                    } else {
                        0.0
                    };
                    let path_hit = if fields[0].contains(normalized_query.as_str()) {
                        1.0
                    } else {
                        0.0
                    };
                    let token_ratio = if query_tokens.is_empty() {
                        0.0
                    } else {
                        usize_to_f64_saturating(token_hit_count)
                            / usize_to_f64_saturating(query_tokens.len())
                    };
                    (exact_name * 0.5 + path_hit * 0.3 + token_ratio * 0.2).clamp(0.0, 1.0)
                };

                out.push(LinkGraphAttachmentHit {
                    source_stem: row.source_stem.clone(),
                    source_title: row.source_title.clone(),
                    source_path: row.source_path.clone(),
                    attachment_path: row.attachment_path.clone(),
                    attachment_name: row.attachment_name.clone(),
                    attachment_ext: row.attachment_ext.clone(),
                    kind: row.kind,
                    score,
                    vision_snippet: row.vision_annotation.as_ref().map(|v| {
                        let desc = &v.description;
                        if desc.len() > 100 {
                            format!("{}...", &desc[..100])
                        } else {
                            desc.clone()
                        }
                    }),
                });
            }
        }

        out.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then(left.attachment_path.cmp(&right.attachment_path))
                .then(left.source_path.cmp(&right.source_path))
        });
        out.truncate(bounded_limit);
        out
    }
}

fn usize_to_f64_saturating(value: usize) -> f64 {
    u32::try_from(value).map_or(f64::from(u32::MAX), f64::from)
}
