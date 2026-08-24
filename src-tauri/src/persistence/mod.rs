use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Default)]
pub struct SettingsStore {
    values: HashMap<String, serde_json::Value>,
}
impl SettingsStore {
    pub fn get(&self) -> HashMap<String, serde_json::Value> {
        self.values.clone()
    }
    pub fn update(&mut self, values: HashMap<String, serde_json::Value>) {
        self.values.extend(values);
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettingsUpdate {
    pub values: HashMap<String, serde_json::Value>,
}

pub struct SqliteStore {
    connection: Connection,
}

impl SqliteStore {
    pub fn in_memory() -> SqlResult<Self> {
        let connection = Connection::open_in_memory()?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> SqlResult<()> {
        self.connection.execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE IF NOT EXISTS schema_migrations (version INTEGER PRIMARY KEY);
             CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value_json TEXT NOT NULL);
             CREATE TABLE IF NOT EXISTS generation_jobs (
               id TEXT PRIMARY KEY, gateway_profile_id TEXT NOT NULL, kind TEXT NOT NULL,
               operation TEXT, model_id TEXT, status TEXT NOT NULL, progress REAL NOT NULL DEFAULT 0,
               request_json TEXT NOT NULL, response_json TEXT, error_message TEXT,
               created_at TEXT NOT NULL, finished_at TEXT
             );
             CREATE TABLE IF NOT EXISTS assets (
               id TEXT PRIMARY KEY, job_id TEXT, kind TEXT NOT NULL, mime_type TEXT NOT NULL,
               local_path TEXT NOT NULL, thumbnail_path TEXT, size_bytes INTEGER NOT NULL,
               created_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS chat_sessions (
               id TEXT PRIMARY KEY, title TEXT NOT NULL, model_id TEXT NOT NULL,
               gateway_profile_id TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS chat_messages (
               id TEXT PRIMARY KEY, session_id TEXT NOT NULL, role TEXT NOT NULL,
               content TEXT NOT NULL, status TEXT NOT NULL, created_at TEXT NOT NULL
             );",
        )?;
        self.connection.execute(
            "INSERT OR IGNORE INTO schema_migrations(version) VALUES (?1)",
            params![1],
        )?;
        Ok(())
    }

    pub fn set_setting(&self, key: &str, value_json: &str) -> SqlResult<()> {
        self.connection.execute("INSERT INTO settings(key,value_json) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json", params![key, value_json])?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> SqlResult<Option<String>> {
        let mut statement = self
            .connection
            .prepare("SELECT value_json FROM settings WHERE key=?1")?;
        let mut rows = statement.query(params![key])?;
        rows.next()?.map(|row| row.get(0)).transpose()
    }

    pub fn get_settings(&self) -> SqlResult<HashMap<String, serde_json::Value>> {
        let mut statement = self
            .connection
            .prepare("SELECT key, value_json FROM settings")?;
        let rows = statement.query_map([], |row| {
            let key: String = row.get(0)?;
            let raw: String = row.get(1)?;
            let value = serde_json::from_str(&raw).unwrap_or(serde_json::Value::Null);
            Ok((key, value))
        })?;
        let mut values = HashMap::new();
        for row in rows {
            let (key, value) = row?;
            values.insert(key, value);
        }
        Ok(values)
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteStore;

    #[test]
    fn migration_creates_settings_store() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        store
            .set_setting("theme", "\"dark\"")
            .expect("setting should save");
        assert_eq!(store.get_setting("theme").unwrap(), Some("\"dark\"".into()));
    }
}
