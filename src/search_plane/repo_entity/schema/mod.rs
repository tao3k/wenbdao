mod batches;
mod columns;
mod definitions;
mod helpers;
mod rows;

pub(crate) use batches::repo_entity_batches;
#[cfg(test)]
pub(crate) use columns::{
    entity_kind_column, language_column, search_text_column, symbol_kind_column,
};
pub(crate) use columns::{hit_json_column, id_column, path_column, projected_columns};
pub(crate) use definitions::{
    COLUMN_ATTRIBUTES_JSON, COLUMN_AUDIT_STATUS, COLUMN_HIERARCHICAL_URI, COLUMN_HIERARCHY,
    COLUMN_IMPLICIT_BACKLINK_ITEMS_JSON, COLUMN_IMPLICIT_BACKLINKS, COLUMN_LINE_END,
    COLUMN_LINE_START, COLUMN_MODULE_ID, COLUMN_NAME, COLUMN_PATH, COLUMN_PROJECTION_PAGE_IDS,
    COLUMN_QUALIFIED_NAME, COLUMN_SALIENCY_SCORE, COLUMN_SIGNATURE, COLUMN_SUMMARY,
    COLUMN_SYMBOL_KIND, COLUMN_VERIFICATION_STATE, RepoEntityRow,
};
pub(crate) use rows::rows_from_analysis;

#[cfg(test)]
mod tests;
