use super::*;
use crate::zhenfa_router::native::section_create::types::SiblingInfo;

#[test]
fn format_sibling_context_renders_both_sides() {
    let info = section_create::InsertionInfo {
        insertion_byte: 12,
        start_level: 2,
        remaining_path: vec!["New".to_string()],
        prev_sibling: Some(SiblingInfo {
            title: "Prev".to_string(),
            preview: "previous preview".to_string(),
        }),
        next_sibling: Some(SiblingInfo {
            title: "Next".to_string(),
            preview: String::new(),
        }),
    };

    let rendered = format_sibling_context(&info);
    assert!(rendered.contains("prev_sibling"));
    assert!(rendered.contains("Prev"));
    assert!(rendered.contains("previous preview"));
    assert!(rendered.contains("next_sibling"));
    assert!(rendered.contains("Next"));
    assert!(rendered.contains("..."));
}
