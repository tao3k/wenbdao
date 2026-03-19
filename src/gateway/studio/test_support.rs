use serde::Serialize;

pub(crate) fn assert_studio_json_snapshot(name: &str, value: impl Serialize) {
    insta::with_settings!({
        snapshot_path => "../../../tests/snapshots/gateway/studio",
        prepend_module_to_snapshot => false,
        sort_maps => true,
    }, {
        insta::assert_json_snapshot!(name, value);
    });
}

pub(crate) fn round_f64(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
}

pub(crate) fn round_f32(value: f32) -> f32 {
    (value * 10_000.0_f32).round() / 10_000.0_f32
}
