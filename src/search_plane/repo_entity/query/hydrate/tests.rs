use crate::analyzers::RepoSymbolKind;
use crate::search_plane::repo_entity::query::hydrate::{
    id_filter_expression, non_empty_vec, parse_attributes_map, parse_symbol_kind,
    typed_repo_entity_columns,
};
use crate::search_plane::repo_entity::schema::{COLUMN_PATH, COLUMN_SALIENCY_SCORE, id_column};

#[test]
fn id_filter_expression_escapes_quotes() {
    let expression = id_filter_expression(&["repo'one".to_string(), "repo-two".to_string()]);
    assert_eq!(
        expression,
        format!("{} IN ('repo''one','repo-two')", id_column())
    );
}

#[test]
fn typed_repo_entity_columns_include_core_fields() {
    let columns = typed_repo_entity_columns();
    assert!(columns.contains(&id_column().to_string()));
    assert!(columns.contains(&COLUMN_PATH.to_string()));
    assert!(columns.contains(&COLUMN_SALIENCY_SCORE.to_string()));
}

#[test]
fn parse_helpers_cover_empty_and_kind_conversion() {
    assert_eq!(non_empty_vec(Vec::new()), None);
    assert_eq!(
        parse_symbol_kind("module_export"),
        RepoSymbolKind::ModuleExport
    );
    assert_eq!(parse_symbol_kind("unknown"), RepoSymbolKind::Other);
    let attributes = parse_attributes_map(Some("{\"arity\":\"1\"}"))
        .unwrap_or_else(|error| panic!("attributes: {error}"));
    assert_eq!(attributes.get("arity").map(String::as_str), Some("1"));
}
