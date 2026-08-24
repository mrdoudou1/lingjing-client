use super::{persist_asset, persist_job};
use crate::{domain::VideoRequest, AppState};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub async fn video_create_job(
    request: VideoRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    let (job, profile, key) = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        let profile = registry
            .profile(&request.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?;
        let key = state
            .secrets
            .lock()
            .map_err(|_| "secret lock poisoned")?
            .get(&profile.api_key_ref)?;
        (registry.create_video_job(&request), profile, key)
    };
    let inserted = {
        let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        jobs.insert(job)
    };
    persist_job(state.inner(), &inserted)?;
    if profile.base_url.starts_with("mock://") {
        return Ok(inserted);
    }
    let remote = crate::gateways::http::GatewayHttpClient::default()
        .create_video(&profile, key.as_deref(), &request)
        .await?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let updated = jobs
        .set_remote_job_id(inserted.id, remote.remote_id)
        .ok_or_else(|| "job disappeared".to_string())?;
    drop(jobs);
    persist_job(state.inner(), &updated)?;
    Ok(updated)
}

#[tauri::command]
pub async fn video_poll_job(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let job = {
        let jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        if let Some(job) = jobs.get(id) {
            Some(job)
        } else {
            drop(jobs);
            let database = state
                .database
                .lock()
                .map_err(|_| "database lock poisoned")?;
            let snapshot = database
                .get_snapshot::<crate::domain::GenerationJob>("jobs", &id.to_string())
                .map_err(|error| error.to_string())?;
            if let Some(ref snapshot) = snapshot {
                let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
                jobs.insert(snapshot.clone());
            }
            snapshot
        }
    };
    let Some(job) = job else { return Ok(None) };
    let Some(remote_id) = job.remote_job_id.clone() else {
        return Ok(Some(job));
    };
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&job.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    let remote = crate::gateways::http::GatewayHttpClient::default()
        .get_video_status(&profile, key.as_deref(), &remote_id)
        .await?;
    let status = match remote.status.as_str() {
        "succeeded" | "completed" | "success" => crate::domain::JobStatus::Succeeded,
        "failed" | "error" => crate::domain::JobStatus::Failed,
        "canceled" | "cancelled" => crate::domain::JobStatus::Canceled,
        _ => crate::domain::JobStatus::Running,
    };
    if matches!(status, crate::domain::JobStatus::Succeeded) {
        let url = remote
            .result_url
            .ok_or_else(|| "GATEWAY_INVALID_RESPONSE: video result URL missing".to_string())?;
        let bytes = crate::gateways::http::GatewayHttpClient::default()
            .download_bytes(&url, key.as_deref())
            .await?;
        let asset_id = format!("asset_{}", job.id);
        let path = state
            .media
            .lock()
            .map_err(|_| "media lock poisoned")?
            .save_bytes(&asset_id, "mp4", &bytes)?;
        persist_asset(
            state.inner(),
            &crate::domain::Asset {
                id: asset_id,
                job_id: Some(job.id),
                kind: "video".into(),
                mime_type: "video/mp4".into(),
                local_path: path.to_string_lossy().into_owned(),
                thumbnail_path: None,
                size_bytes: bytes.len() as u64,
                favorite: false,
                created_at: chrono::Utc::now(),
            },
        )?;
    }
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let updated = jobs
        .update(job.id, status, remote.progress, remote.error_message)
        .ok_or_else(|| "job disappeared".to_string())?;
    drop(jobs);
    persist_job(state.inner(), &updated)?;
    Ok(Some(updated))
}
