use serde::Serialize;

pub(super) fn assert_studio_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../../../snapshots/gateway/studio",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

#[allow(dead_code)]
pub(super) fn round_f64(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

#[allow(dead_code)]
pub(super) fn round_f32(value: f32) -> f32 {
    ((value as f64) * 10_000.0).round() as f32 / 10_000.0_f32
}
