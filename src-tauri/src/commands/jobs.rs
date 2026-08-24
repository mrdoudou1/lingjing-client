use super::{emit_job_event, persist_job};
use crate::AppState;
use tauri::{AppHandle, State};
use uuid::Uuid;

#[tauri::command]
pub fn job_get(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    if let Some(job) = jobs.get(id) {
        return Ok(Some(job));
    }
    drop(jobs);
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .get_snapshot("jobs", &id.to_string())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn job_cancel(
    app: AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let remote_context = {
        let jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        if let Some(job) = jobs.get(id) {
            job.remote_job_id
                .clone()
                .map(|remote_id| (remote_id, job.gateway_profile_id))
        } else {
            drop(jobs);
            let database = state
                .database
                .lock()
                .map_err(|_| "database lock poisoned")?;
            database
                .get_snapshot::<crate::domain::GenerationJob>("jobs", &id.to_string())
                .map_err(|error| error.to_string())?
                .and_then(|job| {
                    job.remote_job_id
                        .map(|remote_id| (remote_id, job.gateway_profile_id))
                })
        }
    };
    if let Some((remote_id, profile_id)) = remote_context {
        let profile = {
            let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
            registry.profile(&profile_id)
        };
        if let Some(profile) = profile {
            let key = state
                .secrets
                .lock()
                .map_err(|_| "secret lock poisoned")?
                .get(&profile.api_key_ref)?;
            let _ = crate::gateways::http::GatewayHttpClient::default()
                .cancel_video(&profile, key.as_deref(), &remote_id)
                .await;
        }
    }
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let result = jobs.cancel(id);
    drop(jobs);
    if let Some(ref job) = result {
        persist_job(state.inner(), job)?;
        emit_job_event(&app, "job://status", job);
        return Ok(result);
    }
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    let Some(mut job) = database
        .get_snapshot::<crate::domain::GenerationJob>("jobs", &id.to_string())
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    job.status = crate::domain::JobStatus::Canceled;
    database
        .save_snapshot("jobs", &job.id.to_string(), &job)
        .map_err(|error| error.to_string())?;
    emit_job_event(&app, "job://status", &job);
    Ok(Some(job))
}

#[tauri::command]
pub fn job_retry(
    app: AppHandle,
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let result = jobs.retry(id);
    drop(jobs);
    if let Some(ref job) = result {
        persist_job(state.inner(), job)?;
        emit_job_event(&app, "job://created", job);
        return Ok(result);
    }
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    let Some(mut job) = database
        .get_snapshot::<crate::domain::GenerationJob>("jobs", &id.to_string())
        .map_err(|error| error.to_string())?
    else {
        return Ok(None);
    };
    job.id = Uuid::new_v4();
    job.status = crate::domain::JobStatus::Queued;
    job.progress = 0.0;
    job.error_message = None;
    database
        .save_snapshot("jobs", &job.id.to_string(), &job)
        .map_err(|error| error.to_string())?;
    emit_job_event(&app, "job://created", &job);
    Ok(Some(job))
}

#[tauri::command]
pub fn job_update(
    app: AppHandle,
    id: String,
    status: String,
    progress: f32,
    error_message: Option<String>,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let status: crate::domain::JobStatus =
        serde_json::from_value(serde_json::Value::String(status))
            .map_err(|_| "VALIDATION_FAILED: invalid job status".to_string())?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let result = jobs.update(id, status, progress, error_message);
    drop(jobs);
    if let Some(ref job) = result {
        persist_job(state.inner(), job)?;
        let event = if matches!(job.status, crate::domain::JobStatus::Failed) {
            "job://failed"
        } else if matches!(job.status, crate::domain::JobStatus::Running) {
            "job://progress"
        } else {
            "job://status"
        };
        emit_job_event(&app, event, job);
    }
    Ok(result)
}

#[tauri::command]
pub fn job_list(
    kind: Option<String>,
    status: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<crate::domain::GenerationJob>, String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    let jobs: Vec<crate::domain::GenerationJob> = database
        .list_snapshots("jobs")
        .map_err(|error| error.to_string())?;
    Ok(jobs
        .into_iter()
        .filter(|job| {
            let kind_matches = kind
                .as_deref()
                .is_none_or(|value| format!("{:?}", job.kind).eq_ignore_ascii_case(value));
            let status_matches = status
                .as_deref()
                .is_none_or(|value| format!("{:?}", job.status).eq_ignore_ascii_case(value));
            kind_matches && status_matches
        })
        .collect())
}
