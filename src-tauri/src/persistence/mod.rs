use rusqlite::{params, Connection, Result as SqlResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

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

#[derive(Default)]
pub struct SecretStore {
    values: HashMap<String, String>,
}
impl SecretStore {
    pub fn set(&mut self, reference: String, secret: String) {
        self.values.insert(reference, secret);
    }
    pub fn get(&self, reference: &str) -> Option<String> {
        self.values.get(reference).cloned()
    }
    pub fn remove(&mut self, reference: &str) {
        self.values.remove(reference);
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
    pub fn open(path: impl AsRef<Path>) -> SqlResult<Self> {
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        }
        let connection = Connection::open(path)?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn default_path() -> PathBuf {
        if let Ok(app_data) = std::env::var("APPDATA") {
            return PathBuf::from(app_data)
                .join("Lingjing")
                .join("db")
                .join("lingjing.sqlite");
        }
        if let Ok(home) = std::env::var("HOME") {
            return PathBuf::from(home)
                .join(".lingjing")
                .join("db")
                .join("lingjing.sqlite");
        }
        PathBuf::from("lingjing.sqlite")
    }

    pub fn open_default() -> SqlResult<Self> {
        Self::open(Self::default_path())
    }

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
             CREATE TABLE IF NOT EXISTS gateway_profiles (
               id TEXT PRIMARY KEY, name TEXT NOT NULL, base_url TEXT NOT NULL, protocol TEXT NOT NULL,
               api_key_ref TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1, is_default INTEGER NOT NULL DEFAULT 0,
               created_at TEXT NOT NULL, updated_at TEXT NOT NULL
             );
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
             );
             CREATE TABLE IF NOT EXISTS job_snapshots (
               id TEXT PRIMARY KEY, payload_json TEXT NOT NULL, updated_at TEXT NOT NULL
             );
             CREATE TABLE IF NOT EXISTS asset_snapshots (
               id TEXT PRIMARY KEY, payload_json TEXT NOT NULL, updated_at TEXT NOT NULL
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

    pub fn save_snapshot<T: Serialize>(&self, table: &str, id: &str, value: &T) -> SqlResult<()> {
        let payload = serde_json::to_string(value)
            .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
        let query = match table {
            "jobs" => "INSERT INTO job_snapshots(id,payload_json,updated_at) VALUES (?1,?2,datetime('now')) ON CONFLICT(id) DO UPDATE SET payload_json=excluded.payload_json,updated_at=excluded.updated_at",
            "assets" => "INSERT INTO asset_snapshots(id,payload_json,updated_at) VALUES (?1,?2,datetime('now')) ON CONFLICT(id) DO UPDATE SET payload_json=excluded.payload_json,updated_at=excluded.updated_at",
            _ => return Err(rusqlite::Error::InvalidParameterName(table.into())),
        };
        self.connection.execute(query, params![id, payload])?;
        Ok(())
    }

    pub fn get_snapshot<T: DeserializeOwned>(&self, table: &str, id: &str) -> SqlResult<Option<T>> {
        let query = match table {
            "jobs" => "SELECT payload_json FROM job_snapshots WHERE id=?1",
            "assets" => "SELECT payload_json FROM asset_snapshots WHERE id=?1",
            _ => return Err(rusqlite::Error::InvalidParameterName(table.into())),
        };
        let mut statement = self.connection.prepare(query)?;
        let mut rows = statement.query(params![id])?;
        match rows.next()? {
            Some(row) => {
                let raw: String = row.get(0)?;
                serde_json::from_str(&raw).map(Some).map_err(|error| {
                    rusqlite::Error::FromSqlConversionFailure(
                        0,
                        rusqlite::types::Type::Text,
                        Box::new(error),
                    )
                })
            }
            None => Ok(None),
        }
    }

    pub fn list_snapshots<T: DeserializeOwned>(&self, table: &str) -> SqlResult<Vec<T>> {
        let query = match table {
            "jobs" => "SELECT payload_json FROM job_snapshots ORDER BY updated_at DESC",
            "assets" => "SELECT payload_json FROM asset_snapshots ORDER BY updated_at DESC",
            _ => return Err(rusqlite::Error::InvalidParameterName(table.into())),
        };
        let mut statement = self.connection.prepare(query)?;
        let rows = statement.query_map([], |row| {
            let raw: String = row.get(0)?;
            serde_json::from_str(&raw).map_err(|error| {
                rusqlite::Error::FromSqlConversionFailure(
                    0,
                    rusqlite::types::Type::Text,
                    Box::new(error),
                )
            })
        })?;
        rows.collect()
    }

    pub fn delete_snapshot(&self, table: &str, id: &str) -> SqlResult<()> {
        let query = match table {
            "jobs" => "DELETE FROM job_snapshots WHERE id=?1",
            "assets" => "DELETE FROM asset_snapshots WHERE id=?1",
            _ => return Err(rusqlite::Error::InvalidParameterName(table.into())),
        };
        self.connection.execute(query, params![id])?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::SqliteStore;
    use serde::{Deserialize, Serialize};

    #[test]
    fn migration_creates_settings_store() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        store
            .set_setting("theme", "\"dark\"")
            .expect("setting should save");
        assert_eq!(store.get_setting("theme").unwrap(), Some("\"dark\"".into()));
    }

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct Snapshot {
        id: String,
        status: String,
    }

    #[test]
    fn snapshots_survive_repository_round_trip() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        let snapshot = Snapshot {
            id: "job-1".into(),
            status: "queued".into(),
        };
        store
            .save_snapshot("jobs", "job-1", &snapshot)
            .expect("snapshot should save");
        let restored: Snapshot = store
            .get_snapshot("jobs", "job-1")
            .unwrap()
            .expect("snapshot should restore");
        assert_eq!(restored, snapshot);
    }
}
