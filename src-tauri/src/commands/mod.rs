mod assets;
mod audio;
mod chat;
mod gateway;
mod image;
mod jobs;
mod settings;
mod video;

pub use assets::*;
pub use audio::*;
pub use chat::*;
pub use gateway::*;
pub use image::*;
pub use jobs::*;
pub use settings::*;
pub use video::*;

use crate::AppState;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

pub(crate) fn emit_job_event(app: &AppHandle, event: &str, job: &crate::domain::GenerationJob) {
    let status = serde_json::to_value(&job.status).unwrap_or_default();
    let payload = serde_json::json!({
        "jobId": job.id.to_string(),
        "status": status,
        "progress": job.progress,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "correlationId": Uuid::new_v4().to_string(),
        "job": job,
    });
    let _ = app.emit(event, payload);
}

pub(crate) fn emit_asset_ready(app: &AppHandle, asset: &crate::domain::Asset) {
    let _ = app.emit(
        "job://asset-ready",
        serde_json::json!({
            "jobId": asset.job_id.map(|id| id.to_string()),
            "assetId": asset.id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "correlationId": Uuid::new_v4().to_string(),
            "asset": asset,
        }),
    );
}

pub(crate) fn persist_asset(state: &AppState, asset: &crate::domain::Asset) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .save_snapshot("assets", &asset.id, asset)
        .map_err(|error| error.to_string())?;
    drop(database);
    let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    assets.upsert(asset.clone());
    Ok(())
}

pub(crate) fn persist_job(
    state: &AppState,
    job: &crate::domain::GenerationJob,
) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .save_snapshot("jobs", &job.id.to_string(), job)
        .map_err(|error| error.to_string())
}
