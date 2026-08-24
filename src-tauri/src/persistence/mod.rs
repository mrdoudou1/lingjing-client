use crate::domain::{ChatSession, GatewayProfile, ModelSnapshot};
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

mod credential_store;

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

    pub fn set_setting(&self, key: &str, value_json: &str) -> SqlResult<()> {
        self.connection.execute("INSERT INTO settings(key,value_json) VALUES (?1,?2) ON CONFLICT(key) DO UPDATE SET value_json=excluded.value_json", params![key, value_json])?;
        Ok(())
    }

    pub fn list_gateway_profiles(&self) -> SqlResult<Vec<GatewayProfile>> {
        let mut statement = self.connection.prepare(
            "SELECT id,name,base_url,protocol,api_key_ref,enabled,is_default,created_at,updated_at
             FROM gateway_profiles ORDER BY is_default DESC, updated_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(GatewayProfile {
                id: row.get(0)?,
                name: row.get(1)?,
                base_url: row.get(2)?,
                protocol: row.get(3)?,
                api_key_ref: row.get(4)?,
                enabled: row.get::<_, i64>(5)? != 0,
                is_default: row.get::<_, i64>(6)? != 0,
                created_at: Some(row.get(7)?),
                updated_at: Some(row.get(8)?),
            })
        })?;
        rows.collect()
    }

    pub fn save_model_snapshots(
        &self,
        profile_id: &str,
        models: &[(String, serde_json::Value)],
    ) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        for (model_id, capabilities_json) in models {
            let snapshot = ModelSnapshot {
                id: format!("{profile_id}:{model_id}"),
                gateway_profile_id: profile_id.to_string(),
                model_id: model_id.clone(),
                display_name: None,
                capabilities_json: capabilities_json.clone(),
                raw_json: None,
                last_synced_at: now.clone(),
            };
            let capabilities = serde_json::to_string(&snapshot.capabilities_json)
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            let raw = snapshot
                .raw_json
                .as_ref()
                .map(serde_json::to_string)
                .transpose()
                .map_err(|error| rusqlite::Error::ToSqlConversionFailure(Box::new(error)))?;
            self.connection.execute(
                "INSERT INTO model_snapshots(id,gateway_profile_id,model_id,display_name,capabilities_json,raw_json,last_synced_at)
                 VALUES (?1,?2,?3,?4,?5,?6,?7)
                 ON CONFLICT(id) DO UPDATE SET display_name=excluded.display_name,
                 capabilities_json=excluded.capabilities_json,raw_json=excluded.raw_json,last_synced_at=excluded.last_synced_at",
                params![snapshot.id, snapshot.gateway_profile_id, snapshot.model_id, snapshot.display_name, capabilities, raw, snapshot.last_synced_at],
            )?;
        }
        Ok(())
    }

    pub fn list_model_snapshots(&self, profile_id: &str) -> SqlResult<Vec<ModelSnapshot>> {
        let mut statement = self.connection.prepare(
            "SELECT id,gateway_profile_id,model_id,display_name,capabilities_json,raw_json,last_synced_at
             FROM model_snapshots WHERE gateway_profile_id=?1 ORDER BY model_id ASC",
        )?;
        let rows = statement.query_map(params![profile_id], |row| {
            let capabilities: String = row.get(4)?;
            let raw: Option<String> = row.get(5)?;
            Ok(ModelSnapshot {
                id: row.get(0)?,
                gateway_profile_id: row.get(1)?,
                model_id: row.get(2)?,
                display_name: row.get(3)?,
                capabilities_json: serde_json::from_str(&capabilities).unwrap_or_default(),
                raw_json: raw.and_then(|value| serde_json::from_str(&value).ok()),
                last_synced_at: row.get(6)?,
            })
        })?;
        rows.collect()
    }

    pub fn upsert_gateway_profile(&self, profile: &GatewayProfile) -> SqlResult<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let created_at = profile.created_at.as_deref().unwrap_or(&now);
        let updated_at = profile.updated_at.as_deref().unwrap_or(&now);
        if profile.is_default {
            self.connection.execute(
                "UPDATE gateway_profiles SET is_default=0 WHERE id<>?1",
                params![profile.id],
            )?;
        }
        self.connection.execute(
            "INSERT INTO gateway_profiles
             (id,name,base_url,protocol,api_key_ref,enabled,is_default,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
             ON CONFLICT(id) DO UPDATE SET
               name=excluded.name, base_url=excluded.base_url, protocol=excluded.protocol,
               api_key_ref=excluded.api_key_ref, enabled=excluded.enabled,
               is_default=excluded.is_default, updated_at=excluded.updated_at",
            params![
                profile.id,
                profile.name,
                profile.base_url,
                profile.protocol,
                profile.api_key_ref,
                profile.enabled as i64,
                profile.is_default as i64,
                created_at,
                updated_at,
            ],
        )?;
        Ok(())
    }

    pub fn delete_gateway_profile(&self, id: &str) -> SqlResult<()> {
        self.connection
            .execute("DELETE FROM gateway_profiles WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn list_chat_sessions(&self) -> SqlResult<Vec<ChatSession>> {
        let mut statement = self.connection.prepare(
            "SELECT id,title,model_id,gateway_profile_id,created_at,updated_at
             FROM chat_sessions ORDER BY updated_at DESC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ChatSession {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    model_id: row.get(2)?,
                    gateway_profile_id: row.get(3)?,
                    messages: Vec::new(),
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                },
            ))
        })?;
        let mut sessions = Vec::new();
        for row in rows {
            let (id, mut session) = row?;
            let mut messages = self.connection.prepare(
                "SELECT id,role,content,status,created_at FROM chat_messages WHERE session_id=?1 ORDER BY created_at ASC",
            )?;
            let rows = messages.query_map(params![id], |message| {
                Ok(crate::domain::ChatMessage {
                    id: message.get(0)?,
                    role: message.get(1)?,
                    content: message.get(2)?,
                    status: message.get(3)?,
                    created_at: message.get(4)?,
                })
            })?;
            session.messages = rows.collect::<SqlResult<Vec<_>>>()?;
            sessions.push(session);
        }
        Ok(sessions)
    }

    pub fn save_chat_session(&self, session: &ChatSession) -> SqlResult<()> {
        self.connection.execute(
            "INSERT INTO chat_sessions(id,title,model_id,gateway_profile_id,created_at,updated_at)
             VALUES (?1,?2,?3,?4,?5,?6)
             ON CONFLICT(id) DO UPDATE SET title=excluded.title,model_id=excluded.model_id,
             gateway_profile_id=excluded.gateway_profile_id,updated_at=excluded.updated_at",
            params![
                session.id,
                session.title,
                session.model_id,
                session.gateway_profile_id,
                session.created_at,
                session.updated_at,
            ],
        )?;
        self.connection.execute(
            "DELETE FROM chat_messages WHERE session_id=?1",
            params![session.id],
        )?;
        for message in &session.messages {
            self.connection.execute(
                "INSERT INTO chat_messages(id,session_id,role,content,status,created_at)
                 VALUES (?1,?2,?3,?4,?5,?6)",
                params![
                    message.id,
                    session.id,
                    message.role,
                    message.content,
                    message.status,
                    message.created_at,
                ],
            )?;
        }
        Ok(())
    }

    pub fn delete_chat_session(&self, id: &str) -> SqlResult<()> {
        self.connection
            .execute("DELETE FROM chat_messages WHERE session_id=?1", params![id])?;
        self.connection
            .execute("DELETE FROM chat_sessions WHERE id=?1", params![id])?;
        Ok(())
    }

    pub fn set_default_gateway_profile(&self, id: &str) -> SqlResult<()> {
        self.connection
            .execute("UPDATE gateway_profiles SET is_default=0", [])?;
        self.connection.execute(
            "UPDATE gateway_profiles SET is_default=1, updated_at=datetime('now') WHERE id=?1",
            params![id],
        )?;
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
    use crate::domain::{ChatSession, GatewayProfile};
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

    #[test]
    fn gateway_profiles_round_trip_and_keep_one_default() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        let first = GatewayProfile {
            id: "first".into(),
            name: "First".into(),
            base_url: "mock://first".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:first".into(),
            enabled: true,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        let second = GatewayProfile {
            id: "second".into(),
            name: "Second".into(),
            base_url: "mock://second".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:second".into(),
            enabled: true,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        store.upsert_gateway_profile(&first).unwrap();
        store.upsert_gateway_profile(&second).unwrap();
        let profiles = store.list_gateway_profiles().unwrap();
        assert_eq!(profiles.len(), 2);
        assert_eq!(
            profiles.iter().filter(|profile| profile.is_default).count(),
            1
        );
        assert_eq!(profiles[0].id, "second");
    }

    #[test]
    fn chat_sessions_round_trip_with_messages() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        let session = ChatSession {
            id: "session-1".into(),
            title: "Test".into(),
            model_id: "gpt-4.1".into(),
            gateway_profile_id: "mock-default".into(),
            messages: vec![crate::domain::ChatMessage {
                id: "message-1".into(),
                role: "user".into(),
                content: "hello".into(),
                status: "completed".into(),
                created_at: "2026-08-24T00:00:00Z".into(),
            }],
            created_at: "2026-08-24T00:00:00Z".into(),
            updated_at: "2026-08-24T00:00:00Z".into(),
        };
        store.save_chat_session(&session).unwrap();
        let restored = store.list_chat_sessions().unwrap();
        assert_eq!(restored.len(), 1);
        assert_eq!(restored[0].messages[0].content, "hello");
        store.delete_chat_session("session-1").unwrap();
        assert!(store.list_chat_sessions().unwrap().is_empty());
    }

    #[test]
    fn model_snapshots_refresh_by_gateway() {
        let store = SqliteStore::in_memory().expect("sqlite should initialize");
        store
            .save_model_snapshots(
                "gateway-1",
                &[
                    (
                        "gpt-4.1".into(),
                        serde_json::json!({ "chat": { "streaming": true } }),
                    ),
                    (
                        "flux-pro".into(),
                        serde_json::json!({ "image": { "supportsEdit": true } }),
                    ),
                ],
            )
            .unwrap();
        let snapshots = store.list_model_snapshots("gateway-1").unwrap();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0].gateway_profile_id, "gateway-1");
        let flux = snapshots
            .iter()
            .find(|snapshot| snapshot.model_id == "flux-pro")
            .unwrap();
        assert_eq!(
            flux.capabilities_json,
            serde_json::json!({ "image": { "supportsEdit": true } })
        );
    }
}
