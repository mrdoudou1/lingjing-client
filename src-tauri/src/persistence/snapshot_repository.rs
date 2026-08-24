use super::SqliteStore;
use rusqlite::{params, Result as SqlResult};
use serde::{de::DeserializeOwned, Serialize};

impl SqliteStore {
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
