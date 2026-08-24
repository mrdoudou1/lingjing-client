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
