use std::sync::Arc;

use xiuxian_vector::{
    LanceFloat64Array, LanceListArray, LanceListBuilder, LanceRecordBatch, LanceStringArray,
    LanceStringBuilder, LanceUInt32Array, VectorStoreError,
};

use crate::search_plane::repo_entity::schema::definitions::RepoEntityRow;
use crate::search_plane::repo_entity::schema::rows::repo_entity_schema;

const CHUNK_SIZE: usize = 1_000;

pub(crate) fn repo_entity_batches(
    rows: &[RepoEntityRow],
) -> Result<Vec<LanceRecordBatch>, VectorStoreError> {
    rows.chunks(CHUNK_SIZE)
        .map(batch_from_rows)
        .collect::<Result<Vec<_>, _>>()
}

fn batch_from_rows(rows: &[RepoEntityRow]) -> Result<LanceRecordBatch, VectorStoreError> {
    let schema = repo_entity_schema();
    let ids = rows.iter().map(|row| row.id.clone()).collect::<Vec<_>>();
    let entity_kind = rows
        .iter()
        .map(|row| row.entity_kind.clone())
        .collect::<Vec<_>>();
    let names = rows.iter().map(|row| row.name.clone()).collect::<Vec<_>>();
    let name_folded = rows
        .iter()
        .map(|row| row.name_folded.clone())
        .collect::<Vec<_>>();
    let qualified_name = rows
        .iter()
        .map(|row| row.qualified_name.clone())
        .collect::<Vec<_>>();
    let qualified_name_folded = rows
        .iter()
        .map(|row| row.qualified_name_folded.clone())
        .collect::<Vec<_>>();
    let paths = rows.iter().map(|row| row.path.clone()).collect::<Vec<_>>();
    let path_folded = rows
        .iter()
        .map(|row| row.path_folded.clone())
        .collect::<Vec<_>>();
    let languages = rows
        .iter()
        .map(|row| row.language.clone())
        .collect::<Vec<_>>();
    let symbol_kind = rows
        .iter()
        .map(|row| row.symbol_kind.clone())
        .collect::<Vec<_>>();
    let module_id = rows
        .iter()
        .map(|row| row.module_id.clone())
        .collect::<Vec<_>>();
    let signature = rows
        .iter()
        .map(|row| row.signature.clone())
        .collect::<Vec<_>>();
    let signature_folded = rows
        .iter()
        .map(|row| row.signature_folded.clone())
        .collect::<Vec<_>>();
    let summary = rows
        .iter()
        .map(|row| row.summary.clone())
        .collect::<Vec<_>>();
    let summary_folded = rows
        .iter()
        .map(|row| row.summary_folded.clone())
        .collect::<Vec<_>>();
    let related_symbols_folded = rows
        .iter()
        .map(|row| row.related_symbols_folded.clone())
        .collect::<Vec<_>>();
    let related_modules_folded = rows
        .iter()
        .map(|row| row.related_modules_folded.clone())
        .collect::<Vec<_>>();
    let line_start = rows.iter().map(|row| row.line_start).collect::<Vec<_>>();
    let line_end = rows.iter().map(|row| row.line_end).collect::<Vec<_>>();
    let audit_status = rows
        .iter()
        .map(|row| row.audit_status.clone())
        .collect::<Vec<_>>();
    let verification_state = rows
        .iter()
        .map(|row| row.verification_state.clone())
        .collect::<Vec<_>>();
    let attributes_json = rows
        .iter()
        .map(|row| row.attributes_json.clone())
        .collect::<Vec<_>>();
    let hierarchical_uri = rows
        .iter()
        .map(|row| row.hierarchical_uri.clone())
        .collect::<Vec<_>>();
    let hierarchy = build_utf8_list_array(
        rows.iter()
            .map(|row| row.hierarchy.as_slice())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let implicit_backlinks = build_utf8_list_array(
        rows.iter()
            .map(|row| row.implicit_backlinks.as_slice())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let implicit_backlink_items_json = rows
        .iter()
        .map(|row| row.implicit_backlink_items_json.clone())
        .collect::<Vec<_>>();
    let projection_page_ids = build_utf8_list_array(
        rows.iter()
            .map(|row| row.projection_page_ids.as_slice())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    let saliency_scores = rows
        .iter()
        .map(|row| row.saliency_score)
        .collect::<Vec<_>>();
    let search_text = rows
        .iter()
        .map(|row| row.search_text.clone())
        .collect::<Vec<_>>();
    let hit_json = rows
        .iter()
        .map(|row| row.hit_json.clone())
        .collect::<Vec<_>>();

    LanceRecordBatch::try_new(
        schema,
        vec![
            Arc::new(LanceStringArray::from(ids)),
            Arc::new(LanceStringArray::from(entity_kind)),
            Arc::new(LanceStringArray::from(names)),
            Arc::new(LanceStringArray::from(name_folded)),
            Arc::new(LanceStringArray::from(qualified_name)),
            Arc::new(LanceStringArray::from(qualified_name_folded)),
            Arc::new(LanceStringArray::from(paths)),
            Arc::new(LanceStringArray::from(path_folded)),
            Arc::new(LanceStringArray::from(languages)),
            Arc::new(LanceStringArray::from(symbol_kind)),
            Arc::new(LanceStringArray::from(module_id)),
            Arc::new(LanceStringArray::from(signature)),
            Arc::new(LanceStringArray::from(signature_folded)),
            Arc::new(LanceStringArray::from(summary)),
            Arc::new(LanceStringArray::from(summary_folded)),
            Arc::new(LanceStringArray::from(related_symbols_folded)),
            Arc::new(LanceStringArray::from(related_modules_folded)),
            Arc::new(LanceUInt32Array::from(line_start)),
            Arc::new(LanceUInt32Array::from(line_end)),
            Arc::new(LanceStringArray::from(audit_status)),
            Arc::new(LanceStringArray::from(verification_state)),
            Arc::new(LanceStringArray::from(attributes_json)),
            Arc::new(LanceStringArray::from(hierarchical_uri)),
            Arc::new(hierarchy),
            Arc::new(implicit_backlinks),
            Arc::new(LanceStringArray::from(implicit_backlink_items_json)),
            Arc::new(projection_page_ids),
            Arc::new(LanceFloat64Array::from(saliency_scores)),
            Arc::new(LanceStringArray::from(search_text)),
            Arc::new(LanceStringArray::from(hit_json)),
        ],
    )
    .map_err(VectorStoreError::Arrow)
}

fn build_utf8_list_array(rows: &[&[String]]) -> LanceListArray {
    let mut builder = LanceListBuilder::new(LanceStringBuilder::new());
    for row in rows {
        for value in *row {
            builder.values().append_value(value);
        }
        builder.append(true);
    }
    builder.finish()
}
