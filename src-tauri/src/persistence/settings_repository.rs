use super::SqliteStore;
use rusqlite::{params, Result as SqlResult};
use std::collections::HashMap;

impl SqliteStore {
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
