//! Valkey-backed saliency write-path helpers.

mod coactivation;
mod edge_updates;
mod time;
mod touch;
mod types;
mod valkey;

pub use valkey::{valkey_saliency_del, valkey_saliency_touch, valkey_saliency_touch_with_valkey};

#[cfg(test)]
mod tests;
