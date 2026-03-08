use crate::link_graph_refs::{extract_entity_refs, get_ref_stats};

use super::frontmatter::parse_frontmatter;
use super::relations::infer_relations;
use super::types::{EnhancedNote, EntityRefData, NoteInput, RefStatsData};

/// Enhance a single note with full secondary analysis.
#[must_use]
pub fn enhance_note(input: &NoteInput) -> EnhancedNote {
    let frontmatter = parse_frontmatter(&input.content);
    let entity_refs_raw = extract_entity_refs(&input.content);
    let stats_raw = get_ref_stats(&input.content);

    let entity_refs: Vec<EntityRefData> = entity_refs_raw
        .iter()
        .map(|r| EntityRefData {
            name: r.name.clone(),
            entity_type: r.entity_type.clone(),
            original: r.original.clone(),
        })
        .collect();

    let ref_stats = RefStatsData {
        total_refs: stats_raw.total_refs,
        unique_entities: stats_raw.unique_entities,
        by_type: stats_raw.by_type.clone(),
    };

    let relations = infer_relations(&input.path, &input.title, &frontmatter, &entity_refs_raw);

    EnhancedNote {
        path: input.path.clone(),
        title: input.title.clone(),
        frontmatter,
        entity_refs,
        ref_stats,
        inferred_relations: relations,
    }
}

/// Batch enhance multiple notes (parallelized with Rayon).
pub fn enhance_notes_batch(inputs: &[NoteInput]) -> Vec<EnhancedNote> {
    use rayon::prelude::*;
    inputs.par_iter().map(enhance_note).collect()
}
