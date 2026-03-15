use redis::Connection;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const TEST_VALKEY_URL: &str = "redis://127.0.0.1:6379/0";
static PREFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn unique_prefix() -> String {
    let seq = PREFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    format!("xiuxian_wendao:test:suggested_link:{pid}:{nanos}:{seq}")
}

pub(super) fn valkey_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let client = redis::Client::open(TEST_VALKEY_URL)?;
    Ok(client.get_connection()?)
}

pub(super) fn clear_prefix(prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(keys).query::<()>(&mut conn)?;
    }
    Ok(())
}
