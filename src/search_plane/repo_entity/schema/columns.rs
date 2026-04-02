#[cfg(test)]
use crate::search_plane::repo_entity::schema::definitions::COLUMN_SEARCH_TEXT;
use crate::search_plane::repo_entity::schema::definitions::{
    COLUMN_ENTITY_KIND, COLUMN_HIT_JSON, COLUMN_ID, COLUMN_LANGUAGE, COLUMN_NAME,
    COLUMN_NAME_FOLDED, COLUMN_PATH, COLUMN_PATH_FOLDED, COLUMN_QUALIFIED_NAME_FOLDED,
    COLUMN_RELATED_MODULES_FOLDED, COLUMN_RELATED_SYMBOLS_FOLDED, COLUMN_SALIENCY_SCORE,
    COLUMN_SIGNATURE_FOLDED, COLUMN_SUMMARY_FOLDED, COLUMN_SYMBOL_KIND,
};

pub(crate) const fn projected_columns() -> [&'static str; 14] {
    [
        COLUMN_ID,
        COLUMN_ENTITY_KIND,
        COLUMN_NAME,
        COLUMN_NAME_FOLDED,
        COLUMN_QUALIFIED_NAME_FOLDED,
        COLUMN_PATH,
        COLUMN_PATH_FOLDED,
        COLUMN_LANGUAGE,
        COLUMN_SYMBOL_KIND,
        COLUMN_SIGNATURE_FOLDED,
        COLUMN_SUMMARY_FOLDED,
        COLUMN_RELATED_SYMBOLS_FOLDED,
        COLUMN_RELATED_MODULES_FOLDED,
        COLUMN_SALIENCY_SCORE,
    ]
}

pub(crate) const fn id_column() -> &'static str {
    COLUMN_ID
}

pub(crate) const fn hit_json_column() -> &'static str {
    COLUMN_HIT_JSON
}

#[cfg(test)]
pub(crate) const fn search_text_column() -> &'static str {
    COLUMN_SEARCH_TEXT
}

#[cfg(test)]
pub(crate) const fn language_column() -> &'static str {
    COLUMN_LANGUAGE
}

pub(crate) const fn path_column() -> &'static str {
    COLUMN_PATH
}

#[cfg(test)]
pub(crate) const fn entity_kind_column() -> &'static str {
    COLUMN_ENTITY_KIND
}

#[cfg(test)]
pub(crate) const fn symbol_kind_column() -> &'static str {
    COLUMN_SYMBOL_KIND
}
