use crate::analyzers::{DocsPlannerRankReasonCode, DocsPlannerWorksetQuotaHint};

#[test]
fn workset_quota_hint_roundtrip() {
    let value = DocsPlannerWorksetQuotaHint {
        target_floor_count: 2,
        target_ceiling_count: 4,
        within_target_band: true,
    };
    let encoded = serde_json::to_string(&value).expect("serialize quota hint");
    let decoded: DocsPlannerWorksetQuotaHint =
        serde_json::from_str(&encoded).expect("deserialize quota hint");
    assert_eq!(decoded, value);
}

#[test]
fn rank_reason_code_serializes_as_snake_case() {
    let encoded = serde_json::to_string(&DocsPlannerRankReasonCode::ReferencePageBonus)
        .expect("serialize rank reason");
    assert_eq!(encoded, "\"reference_page_bonus\"");
}
