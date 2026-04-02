use chrono::Utc;

use crate::types::KnowledgeEntry;
use crate::valkey_common::{first_non_empty_env, open_client};

use super::KnowledgeStorage;

const KNOWLEDGE_VALKEY_URL_ENV: &str = "XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL";
const DEFAULT_KNOWLEDGE_VALKEY_URL: &str = "redis://127.0.0.1/";

impl KnowledgeStorage {
    /// Initialize the storage (validate Valkey connectivity).
    ///
    /// # Errors
    ///
    /// Returns an error when the Valkey client cannot be created, the
    /// connection cannot be established, or the connectivity/key checks fail.
    pub fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::redis_client()?;
        let mut conn = client.get_connection()?;
        let _pong: String = redis::cmd("PING").query(&mut conn)?;
        let _exists: i64 = redis::cmd("EXISTS")
            .arg(self.entries_key())
            .query(&mut conn)?;
        Ok(())
    }

    pub(super) fn entries_key(&self) -> String {
        format!("{}:entries", self.table_name())
    }

    fn resolve_knowledge_valkey_url() -> String {
        resolve_knowledge_valkey_url_with_fallback(first_non_empty_env(&[
            KNOWLEDGE_VALKEY_URL_ENV,
            "VALKEY_URL",
            "REDIS_URL",
        ]))
    }

    pub(super) fn redis_client() -> Result<redis::Client, String> {
        let url = Self::resolve_knowledge_valkey_url();
        open_client(url.as_str()).map_err(|e| e.to_string())
    }

    /// Upsert a knowledge entry.
    ///
    /// # Errors
    ///
    /// Returns an error when storage initialization fails, the Valkey
    /// connection fails, or JSON serialization/deserialization fails.
    pub fn upsert(&self, entry: &KnowledgeEntry) -> Result<(), Box<dyn std::error::Error>> {
        self.init()?;
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let entries_key = self.entries_key();
        let existing_raw: Option<String> = redis::cmd("HGET")
            .arg(&entries_key)
            .arg(&entry.id)
            .query(&mut conn)?;
        let existing = existing_raw
            .as_deref()
            .map(serde_json::from_str::<KnowledgeEntry>)
            .transpose()?;

        let now = Utc::now();
        let (created_at, version) = if let Some(found) = existing {
            (found.created_at, found.version + 1)
        } else {
            (now, entry.version.max(1))
        };

        let mut to_store = entry.clone();
        to_store.created_at = created_at;
        to_store.updated_at = now;
        to_store.version = version;
        let payload = serde_json::to_string(&to_store)?;

        let _: i64 = redis::cmd("HSET")
            .arg(entries_key)
            .arg(&to_store.id)
            .arg(payload)
            .query(&mut conn)?;
        Ok(())
    }

    /// Count total entries.
    ///
    /// # Errors
    ///
    /// Returns an error when the Valkey client or connection cannot be
    /// created, or when the `HLEN` command fails.
    pub fn count(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let total: i64 = redis::cmd("HLEN")
            .arg(self.entries_key())
            .query(&mut conn)?;
        Ok(total)
    }

    /// Delete an entry by ID.
    ///
    /// # Errors
    ///
    /// Returns an error when the Valkey client or connection cannot be
    /// created, or when the `HDEL` command fails.
    pub fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let _: i64 = redis::cmd("HDEL")
            .arg(self.entries_key())
            .arg(id)
            .query(&mut conn)?;
        Ok(())
    }

    /// Retrieve one knowledge entry by ID.
    ///
    /// # Errors
    /// Returns an error if Valkey connection or deserialization fails.
    pub fn get_entry(
        &self,
        id: &str,
    ) -> Result<Option<KnowledgeEntry>, Box<dyn std::error::Error>> {
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let entries_key = self.entries_key();
        let raw: Option<String> = redis::cmd("HGET")
            .arg(&entries_key)
            .arg(id)
            .query(&mut conn)?;

        match raw {
            Some(s) => Ok(Some(serde_json::from_str::<KnowledgeEntry>(&s)?)),
            None => Ok(None),
        }
    }

    /// Load all knowledge entries from the table.
    ///
    /// # Errors
    /// Returns an error if Valkey connection or deserialization fails.
    pub fn load_all_entries(&self) -> Result<Vec<KnowledgeEntry>, Box<dyn std::error::Error>> {
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let entries_key = self.entries_key();
        let raws: std::collections::HashMap<String, String> =
            redis::cmd("HGETALL").arg(&entries_key).query(&mut conn)?;

        let mut out = Vec::new();
        for s in raws.values() {
            out.push(serde_json::from_str::<KnowledgeEntry>(s)?);
        }
        Ok(out)
    }

    /// Clear all entries.
    ///
    /// # Errors
    ///
    /// Returns an error when the Valkey client or connection cannot be
    /// created, or when the `DEL` command fails.
    pub fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = Self::redis_client()
            .map_err(|e| Box::new(std::io::Error::other(e)) as Box<dyn std::error::Error>)?;
        let mut conn = client.get_connection()?;
        let _: i64 = redis::cmd("DEL").arg(self.entries_key()).query(&mut conn)?;
        Ok(())
    }
}

fn resolve_knowledge_valkey_url_with_fallback(candidate: Option<String>) -> String {
    candidate.unwrap_or_else(|| DEFAULT_KNOWLEDGE_VALKEY_URL.to_string())
}

impl KnowledgeStorage {
    #[cfg(test)]
    fn redis_client_from_url(valkey_url: &str) -> Result<redis::Client, String> {
        open_client(valkey_url).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_KNOWLEDGE_VALKEY_URL, KnowledgeStorage, resolve_knowledge_valkey_url_with_fallback,
    };

    #[test]
    fn resolve_knowledge_valkey_url_uses_localhost_fallback() {
        assert_eq!(
            resolve_knowledge_valkey_url_with_fallback(None),
            DEFAULT_KNOWLEDGE_VALKEY_URL.to_string()
        );
    }

    #[test]
    fn resolve_knowledge_valkey_url_preserves_trimmed_env_choice() {
        assert_eq!(
            resolve_knowledge_valkey_url_with_fallback(Some("redis://127.0.0.1/1".to_string())),
            "redis://127.0.0.1/1".to_string()
        );
    }

    #[test]
    fn redis_client_opens_trimmed_valid_url() {
        let client = KnowledgeStorage::redis_client_from_url(" redis://127.0.0.1/ ");
        assert!(client.is_ok());
    }
}
