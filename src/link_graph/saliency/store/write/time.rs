use std::time::Duration;

pub(super) fn unix_seconds_to_f64(seconds: i64) -> f64 {
    u64::try_from(seconds).map_or(0.0, |value| Duration::from_secs(value).as_secs_f64())
}
