use redis::Connection;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use xiuxian_wendao::link_graph::{LinkGraphSortField, LinkGraphSortOrder, LinkGraphSortTerm};

pub(crate) fn write_file(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    Ok(())
}

pub(crate) fn sort_term(field: LinkGraphSortField, order: LinkGraphSortOrder) -> LinkGraphSortTerm {
    LinkGraphSortTerm { field, order }
}

fn valkey_connection() -> Result<Connection, Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1:6379/0")?;
    let conn = client.get_connection()?;
    Ok(conn)
}

pub(crate) fn clear_cache_keys(prefix: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    if !keys.is_empty() {
        redis::cmd("DEL").arg(keys).query::<()>(&mut conn)?;
    }
    Ok(())
}

pub(crate) fn count_cache_keys(prefix: &str) -> Result<usize, Box<dyn std::error::Error>> {
    let mut conn = valkey_connection()?;
    let pattern = format!("{prefix}:*");
    let keys: Vec<String> = redis::cmd("KEYS").arg(&pattern).query(&mut conn)?;
    Ok(keys.len())
}

pub(crate) fn unique_cache_prefix() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_nanos())
        .unwrap_or(0);
    format!("omni:test:link_graph:{nanos}")
}
