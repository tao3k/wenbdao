//! Unit tests for `property_drawer_edges` module.

use super::*;

#[test]
fn test_parse_id_references_single() {
    let refs = parse_id_references("#arch-v1");
    assert_eq!(refs, vec!["arch-v1"]);
}

#[test]
fn test_parse_id_references_comma_separated() {
    let refs = parse_id_references("#id1, #id2, #id3");
    assert_eq!(refs, vec!["id1", "id2", "id3"]);
}

#[test]
fn test_parse_id_references_space_separated() {
    let refs = parse_id_references("#id1 #id2 #id3");
    assert_eq!(refs, vec!["id1", "id2", "id3"]);
}

#[test]
fn test_parse_id_references_mixed() {
    let refs = parse_id_references("#id1, #id2 #id3,\n#id4");
    assert_eq!(refs, vec!["id1", "id2", "id3", "id4"]);
}

#[test]
fn test_parse_id_references_empty() {
    let refs = parse_id_references("");
    assert!(refs.is_empty());
}

#[test]
fn test_parse_id_references_no_hash() {
    let refs = parse_id_references("id1, id2");
    assert!(refs.is_empty());
}

#[test]
fn test_extract_property_drawer_edges_related() {
    let mut attrs = HashMap::new();
    attrs.insert("RELATED".to_string(), "#arch-v1, #impl-v2".to_string());

    let edges = extract_property_drawer_edges("doc.md#intro", &attrs);

    assert_eq!(edges.len(), 2);
    assert_eq!(edges[0].from, "doc.md#intro");
    assert_eq!(edges[0].to, "arch-v1");
    assert_eq!(edges[0].edge_type, LinkGraphEdgeType::PropertyDrawer);
    assert_eq!(edges[0].attribute_key, "RELATED");
    assert_eq!(edges[1].to, "impl-v2");
}

#[test]
fn test_extract_property_drawer_edges_multiple_attrs() {
    let mut attrs = HashMap::new();
    attrs.insert("RELATED".to_string(), "#id1".to_string());
    attrs.insert("DEPENDS_ON".to_string(), "#id2".to_string());

    let edges = extract_property_drawer_edges("doc.md#section", &attrs);

    assert_eq!(edges.len(), 2);
    assert!(edges.iter().any(|e| e.attribute_key == "RELATED"));
    assert!(edges.iter().any(|e| e.attribute_key == "DEPENDS_ON"));
}
