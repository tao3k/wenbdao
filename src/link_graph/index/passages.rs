use super::{IndexedSection, LinkGraphIndex};
use crate::link_graph::models::LinkGraphPassage;

impl LinkGraphIndex {
    #[allow(dead_code)]
    pub(super) fn rebuild_all_passages(&mut self) {
        self.passages_by_id.clear();
        let doc_ids: Vec<String> = self.sections_by_doc.keys().cloned().collect();
        for doc_id in doc_ids {
            self.rebuild_passages_for_doc(&doc_id);
        }
    }

    pub(in crate::link_graph::index) fn rebuild_passages_for_doc(&mut self, doc_id: &str) {
        self.remove_passages_for_doc(doc_id);
        let Some(sections) = self.sections_by_doc.get(doc_id).cloned() else {
            return;
        };
        for (section_idx, section) in sections.iter().enumerate() {
            let passage_id = Self::passage_id_for_section(doc_id, section, section_idx);
            self.passages_by_id.insert(
                passage_id.clone(),
                LinkGraphPassage {
                    id: passage_id,
                    parent_doc_id: doc_id.to_string(),
                    content: section.section_text.clone(),
                    content_lower: section.section_text_lower.clone(),
                    entities: section.entities.clone(),
                },
            );
        }
    }

    pub(in crate::link_graph::index) fn remove_passages_for_doc(&mut self, doc_id: &str) {
        self.passages_by_id
            .retain(|_, passage| passage.parent_doc_id != doc_id);
    }

    fn passage_id_for_section(
        doc_id: &str,
        section: &IndexedSection,
        section_idx: usize,
    ) -> String {
        let heading_slug: String = section
            .heading_path_lower
            .chars()
            .map(|ch| match ch {
                'a'..='z' | '0'..='9' => ch,
                _ => '-',
            })
            .collect();
        let heading_slug = heading_slug.trim_matches('-');
        if heading_slug.is_empty() {
            return format!("{doc_id}#passage-{section_idx}");
        }
        format!("{doc_id}#passage-{section_idx}-{heading_slug}")
    }
}
