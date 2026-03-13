use chrono::Utc;

use crate::types::KnowledgeEntry;

use super::KnowledgeStorage;

impl KnowledgeStorage {
    /// Initialize the storage (validate Valkey connectivity).
    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client()?;
        let mut conn = client.get_connection()?;
        let _pong: String = redis::cmd("PING").query(&mut conn)?;
        Ok(())
    }

    pub(super) fn entries_key(&self) -> String {
        format!("{}:entries", self.table_name)
    }

    pub(super) fn redis_client(&self) -> Result<redis::Client, String> {
        // Correct implementation should use the url from context or env
        let url = std::env::var("XIUXIAN_WENDAO_KNOWLEDGE_VALKEY_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1/".to_string());
        redis::Client::open(url).map_err(|e| e.to_string())
    }

    /// Upsert a knowledge entry.
    pub async fn upsert(&self, entry: &KnowledgeEntry) -> Result<(), Box<dyn std::error::Error>> {
        self.init().await?;
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
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
    pub async fn count(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
        let mut conn = client.get_connection()?;
        let total: i64 = redis::cmd("HLEN")
            .arg(self.entries_key())
            .query(&mut conn)?;
        Ok(total)
    }

    /// Delete an entry by ID.
    pub async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
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
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
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
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
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
    pub async fn clear(&self) -> Result<(), Box<dyn std::error::Error>> {
        let client = self.redis_client().map_err(|e| {
            Box::new(std::io::Error::new(std::io::ErrorKind::Other, e))
                as Box<dyn std::error::Error>
        })?;
        let mut conn = client.get_connection()?;
        let _: i64 = redis::cmd("DEL").arg(self.entries_key()).query(&mut conn)?;
        Ok(())
    }
}
