use super::{emit_asset_ready, emit_job_event, persist_asset, persist_job};
use crate::{domain::AudioRequest, AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn audio_tts(
    app: AppHandle,
    request: AudioRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    if request.kind != "tts" || request.text.as_deref().unwrap_or("").trim().is_empty() {
        return Err("请输入需要合成的文本".into());
    }
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
        (registry.create_audio_job(&request), profile, key)
    };
    let inserted = {
        let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        jobs.insert(job)
    };
    persist_job(state.inner(), &inserted)?;
    emit_job_event(&app, "job://created", &inserted);
    if profile.base_url.starts_with("mock://") {
        return Ok(inserted);
    }
    let bytes = match crate::gateways::http::GatewayHttpClient::default()
        .synthesize_speech(&profile, key.as_deref(), &request)
        .await
    {
        Ok(bytes) => bytes,
        Err(error) => {
            let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
            if let Some(failed) = jobs.update(
                inserted.id,
                crate::domain::JobStatus::Failed,
                0.0,
                Some(error.clone()),
            ) {
                drop(jobs);
                persist_job(state.inner(), &failed)?;
                emit_job_event(&app, "job://failed", &failed);
            }
            return Err(error);
        }
    };
    let extension = request.format.to_lowercase();
    let asset_id = format!("asset_{}", inserted.id);
    let path = state
        .media
        .lock()
        .map_err(|_| "media lock poisoned")?
        .save_bytes(&asset_id, &extension, &bytes)?;
    let asset = crate::domain::Asset {
        id: asset_id,
        job_id: Some(inserted.id),
        kind: "audio".into(),
        mime_type: if extension == "wav" {
            "audio/wav".into()
        } else {
            "audio/mpeg".into()
        },
        local_path: path.to_string_lossy().into_owned(),
        thumbnail_path: None,
        size_bytes: bytes.len() as u64,
        favorite: false,
        created_at: chrono::Utc::now(),
    };
    persist_asset(state.inner(), &asset)?;
    emit_asset_ready(&app, &asset);
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let completed = jobs
        .update(
            inserted.id,
            crate::domain::JobStatus::Succeeded,
            100.0,
            None,
        )
        .ok_or_else(|| "job disappeared".to_string())?;
    drop(jobs);
    persist_job(state.inner(), &completed)?;
    emit_job_event(&app, "job://status", &completed);
    Ok(completed)
}

#[tauri::command]
pub async fn audio_stt(
    app: AppHandle,
    request: AudioRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    if request.kind != "stt"
        || request
            .source_file_name
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("请先选择音频或视频文件".into());
    }
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
        (registry.create_audio_job(&request), profile, key)
    };
    let inserted = {
        let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        jobs.insert(job)
    };
    persist_job(state.inner(), &inserted)?;
    emit_job_event(&app, "job://created", &inserted);
    if profile.base_url.starts_with("mock://") {
        return Ok(inserted);
    }
    let transcript = match crate::gateways::http::GatewayHttpClient::default()
        .transcribe_audio(&profile, key.as_deref(), &request)
        .await
    {
        Ok(result) => result,
        Err(error) => {
            let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
            if let Some(failed) = jobs.update(
                inserted.id,
                crate::domain::JobStatus::Failed,
                0.0,
                Some(error.clone()),
            ) {
                drop(jobs);
                persist_job(state.inner(), &failed)?;
                emit_job_event(&app, "job://failed", &failed);
            }
            return Err(error);
        }
    };
    let extension = request.format.to_lowercase();
    let content = if extension == "json" && !transcript.raw.is_empty() {
        transcript.raw.into_bytes()
    } else {
        transcript.text.into_bytes()
    };
    let asset_id = format!("asset_{}", inserted.id);
    let path = state
        .media
        .lock()
        .map_err(|_| "media lock poisoned")?
        .save_bytes(&asset_id, &extension, &content)?;
    let asset = crate::domain::Asset {
        id: asset_id,
        job_id: Some(inserted.id),
        kind: "audio".into(),
        mime_type: if extension == "json" {
            "application/json".into()
        } else {
            "text/plain".into()
        },
        local_path: path.to_string_lossy().into_owned(),
        thumbnail_path: None,
        size_bytes: content.len() as u64,
        favorite: false,
        created_at: chrono::Utc::now(),
    };
    persist_asset(state.inner(), &asset)?;
    emit_asset_ready(&app, &asset);
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let completed = jobs
        .update(
            inserted.id,
            crate::domain::JobStatus::Succeeded,
            100.0,
            None,
        )
        .ok_or_else(|| "job disappeared".to_string())?;
    drop(jobs);
    persist_job(state.inner(), &completed)?;
    emit_job_event(&app, "job://status", &completed);
    Ok(completed)
}
