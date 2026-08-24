use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod chat_repository;
mod credential_store;
mod gateway_repository;
mod settings_repository;
mod snapshot_repository;
#[cfg(test)]
mod tests;

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
    pub fn set(&mut self, reference: String, secret: String) -> Result<(), String> {
        #[cfg(windows)]
        credential_store::set(&reference, &secret)?;
        self.values.insert(reference, secret);
        Ok(())
    }
    pub fn get(&self, reference: &str) -> Result<Option<String>, String> {
        if let Some(value) = self.values.get(reference) {
            return Ok(Some(value.clone()));
        }
        #[cfg(windows)]
        {
            credential_store::get(reference)
        }
        #[cfg(not(windows))]
        {
            Ok(None)
        }
    }
    pub fn remove(&mut self, reference: &str) -> Result<(), String> {
        #[cfg(windows)]
        credential_store::remove(reference)?;
        self.values.remove(reference);
        Ok(())
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
             CREATE TABLE IF NOT EXISTS model_snapshots (
               id TEXT PRIMARY KEY, gateway_profile_id TEXT NOT NULL, model_id TEXT NOT NULL,
               display_name TEXT, capabilities_json TEXT NOT NULL, raw_json TEXT, last_synced_at TEXT NOT NULL
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
}
