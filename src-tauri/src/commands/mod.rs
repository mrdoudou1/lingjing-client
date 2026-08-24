use crate::{
    domain::{ChatSendInput, ImageRequest, VideoRequest},
    AppState,
};
use tauri::State;
use uuid::Uuid;

#[tauri::command]
pub fn gateway_list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    serde_json::to_value(registry.profiles()).map_err(|error| error.to_string())
}

#[tauri::command]
pub fn gateway_test_connection(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    Ok(registry.test(&profile_id))
}

#[tauri::command]
pub fn gateway_refresh_models(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    Ok(registry.models(&profile_id))
}

#[tauri::command]
pub fn chat_send(input: ChatSendInput) -> Result<serde_json::Value, String> {
    Ok(
        serde_json::json!({"sessionId": input.session_id, "accepted": true, "modelId": input.model_id}),
    )
}

#[tauri::command]
pub fn video_create_job(
    request: VideoRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    let job = registry.create_video_job(&request);
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    Ok(jobs.insert(job))
}

#[tauri::command]
pub fn image_create_job(
    request: ImageRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    if request.prompt.trim().is_empty() {
        return Err("请输入图片描述".into());
    }
    if request.count == 0 || request.count > 4 {
        return Err("图片数量必须在 1 到 4 张之间".into());
    }
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    let job = registry.create_image_job(&request);
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    Ok(jobs.insert(job))
}

#[tauri::command]
pub fn job_get(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    Ok(jobs.get(id))
}

#[tauri::command]
pub fn job_cancel(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    Ok(jobs.cancel(id))
}

#[tauri::command]
pub fn job_retry(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    Ok(jobs.retry(id))
}

#[tauri::command]
pub fn asset_list(state: State<'_, AppState>) -> Result<Vec<crate::domain::Asset>, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    Ok(assets.list())
}

#[tauri::command]
pub fn settings_get(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    Ok(settings.get())
}

#[tauri::command]
pub fn settings_update(
    update: crate::persistence::SettingsUpdate,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut settings = state
        .settings
        .lock()
        .map_err(|_| "settings lock poisoned")?;
    settings.update(update.values);
    Ok(())
}
