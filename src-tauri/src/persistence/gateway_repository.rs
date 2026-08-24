use super::SqliteStore;
use crate::domain::{GatewayProfile, ModelSnapshot};
use rusqlite::{params, Result as SqlResult};

impl SqliteStore {
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
    pub fn set_default_gateway_profile(&self, id: &str) -> SqlResult<()> {
        self.connection
            .execute("UPDATE gateway_profiles SET is_default=0", [])?;
        self.connection.execute(
            "UPDATE gateway_profiles SET is_default=1, updated_at=datetime('now') WHERE id=?1",
            params![id],
        )?;
        Ok(())
    }
}
