use crate::{
    domain::{AudioRequest, ChatSendInput, GatewayProfile, ImageRequest, VideoRequest},
    AppState,
};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::watch;
use uuid::Uuid;

fn persist_asset(state: &AppState, asset: &crate::domain::Asset) -> Result<(), String> {
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

fn persist_job(state: &AppState, job: &crate::domain::GenerationJob) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .save_snapshot("jobs", &job.id.to_string(), job)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn gateway_list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    serde_json::to_value(registry.profiles()).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn gateway_test_connection(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    crate::gateways::http::GatewayHttpClient::default()
        .test_connection(&profile, key.as_deref())
        .await
}

#[tauri::command]
pub async fn gateway_refresh_models(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    crate::gateways::http::GatewayHttpClient::default()
        .list_models(&profile, key.as_deref())
        .await
}

#[tauri::command]
pub fn gateway_create_profile(
    profile: GatewayProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .upsert_gateway_profile(&profile)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.create(profile);
    Ok(())
}

#[tauri::command]
pub fn gateway_update_profile(
    profile: GatewayProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .upsert_gateway_profile(&profile)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.update(profile);
    Ok(())
}

#[tauri::command]
pub fn gateway_delete_profile(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let api_key_ref = state
        .gateways
        .lock()
        .map_err(|_| "gateway lock poisoned")?
        .profile(&id)
        .map(|profile| profile.api_key_ref);
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .delete_gateway_profile(&id)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.delete(&id);
    if let Some(reference) = api_key_ref {
        state
            .secrets
            .lock()
            .map_err(|_| "secret lock poisoned")?
            .remove(&reference)?;
    }
    Ok(())
}

#[tauri::command]
pub fn gateway_set_default(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .set_default_gateway_profile(&id)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.set_default(&id);
    Ok(())
}

#[tauri::command]
pub fn gateway_set_api_key(
    profile_id: String,
    secret: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if secret.trim().is_empty() {
        return Err("VALIDATION_FAILED: API key is empty".into());
    }
    let reference = format!("system-keychain:{profile_id}");
    state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .set(reference.clone(), secret)?;
    Ok(reference)
}

#[tauri::command]
pub fn chat_send(input: ChatSendInput) -> Result<serde_json::Value, String> {
    if input.content.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    let reply = format!(
        "已收到你的请求：**{}**\n\n这是 Rust Mock Gateway 的桌面流式响应。",
        input.content.trim()
    );
    Ok(
        serde_json::json!({"sessionId": input.session_id, "accepted": true, "modelId": input.model_id, "reply": reply}),
    )
}

#[tauri::command]
pub fn chat_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::domain::ChatSession>, String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .list_chat_sessions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_save_session(
    session: crate::domain::ChatSession,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .save_chat_session(&session)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_delete_session(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .delete_chat_session(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    input: ChatSendInput,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if input.content.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&input.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    let correlation_id = Uuid::new_v4().to_string();
    let session_id = input.session_id.clone();
    let (stop_tx, stop_rx) = watch::channel(false);
    {
        let mut stops = state
            .chat_stops
            .lock()
            .map_err(|_| "chat stop lock poisoned")?;
        if let Some(previous) = stops.insert(session_id.clone(), stop_tx) {
            let _ = previous.send(true);
        }
    }
    let event_app = app.clone();
    let result = crate::gateways::http::GatewayHttpClient::default()
        .chat_stream(
            &profile,
            key.as_deref(),
            &input.model_id,
            &input.content,
            stop_rx,
            |delta| {
                event_app
                    .emit(
                        "chat://delta",
                        serde_json::json!({ "sessionId": session_id, "delta": delta, "correlationId": correlation_id }),
                    )
                    .map_err(|error| error.to_string())
            },
        )
        .await;
    if let Ok(mut stops) = state.chat_stops.lock() {
        stops.remove(&input.session_id);
    }
    match result {
        Ok(()) => {
            app.emit(
                "chat://completed",
                serde_json::json!({ "sessionId": input.session_id, "correlationId": correlation_id }),
            )
            .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({ "accepted": true, "correlationId": correlation_id }))
        }
        Err(error) => {
            let code = error
                .split(':')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("GATEWAY_ERROR");
            let _ = app.emit(
                "chat://error",
                serde_json::json!({ "sessionId": input.session_id, "correlationId": correlation_id, "code": code, "message": error }),
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub fn chat_stop(session_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let stops = state
        .chat_stops
        .lock()
        .map_err(|_| "chat stop lock poisoned")?;
    Ok(stops
        .get(&session_id)
        .map(|sender| sender.send(true).is_ok())
        .unwrap_or(false))
}

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

#[tauri::command]
pub async fn image_create_job(
    request: ImageRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    if request.prompt.trim().is_empty() {
        return Err("请输入图片描述".into());
    }
    if request.count == 0 || request.count > 4 {
        return Err("图片数量必须在 1 到 4 张之间".into());
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
        (registry.create_image_job(&request), profile, key)
    };
    let inserted = {
        let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        jobs.insert(job)
    };
    persist_job(state.inner(), &inserted)?;
    if profile.base_url.starts_with("mock://") {
        return Ok(inserted);
    }
    let results = match crate::gateways::http::GatewayHttpClient::default()
        .generate_images(&profile, key.as_deref(), &request)
        .await
    {
        Ok(results) => results,
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
            }
            return Err(error);
        }
    };
    for (index, result) in results.into_iter().enumerate() {
        let bytes = if let Some(encoded) = result.b64_json {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?
        } else if let Some(url) = result.url {
            crate::gateways::http::GatewayHttpClient::default()
                .download_bytes(&url, key.as_deref())
                .await?
        } else {
            return Err("GATEWAY_INVALID_RESPONSE: image result has no data".into());
        };
        let asset_id = format!("asset_{}_{}", inserted.id, index + 1);
        let path = state
            .media
            .lock()
            .map_err(|_| "media lock poisoned")?
            .save_bytes(&asset_id, "png", &bytes)?;
        persist_asset(
            state.inner(),
            &crate::domain::Asset {
                id: asset_id.clone(),
                job_id: Some(inserted.id),
                kind: "image".into(),
                mime_type: "image/png".into(),
                local_path: path.to_string_lossy().into_owned(),
                thumbnail_path: None,
                size_bytes: bytes.len() as u64,
                favorite: false,
                created_at: chrono::Utc::now(),
            },
        )?;
    }
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
    Ok(completed)
}

#[tauri::command]
pub async fn audio_tts(
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
    persist_asset(
        state.inner(),
        &crate::domain::Asset {
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
        },
    )?;
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
    Ok(completed)
}

#[tauri::command]
pub async fn audio_stt(
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
    persist_asset(
        state.inner(),
        &crate::domain::Asset {
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
        },
    )?;
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
    Ok(completed)
}

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
    Ok(Some(job))
}

#[tauri::command]
pub fn job_retry(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::GenerationJob>, String> {
    let id = Uuid::parse_str(&id).map_err(|error| error.to_string())?;
    let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
    let result = jobs.retry(id);
    drop(jobs);
    if let Some(ref job) = result {
        persist_job(state.inner(), job)?;
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
    Ok(Some(job))
}

#[tauri::command]
pub fn job_update(
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

#[tauri::command]
pub fn asset_list(state: State<'_, AppState>) -> Result<Vec<crate::domain::Asset>, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    Ok(assets.list())
}

#[tauri::command]
pub fn asset_register(
    asset: crate::domain::Asset,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .save_snapshot("assets", &asset.id, &asset)
            .map_err(|error| error.to_string())?;
    }
    let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    assets.upsert(asset);
    Ok(())
}

#[tauri::command]
pub fn asset_toggle_favorite(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::Asset>, String> {
    let asset = {
        let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
        assets.toggle_favorite(&id)
    };
    if let Some(ref asset) = asset {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .save_snapshot("assets", &asset.id, asset)
            .map_err(|error| error.to_string())?;
    }
    Ok(asset)
}

#[tauri::command]
pub fn asset_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .delete_snapshot("assets", &id)
            .map_err(|error| error.to_string())?;
    }
    let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    assets.delete(&id);
    Ok(())
}

#[tauri::command]
pub fn asset_open_location(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    let path = assets
        .list()
        .into_iter()
        .find(|asset| asset.id == id)
        .map(|asset| asset.local_path);
    let Some(path) = path else { return Ok(None) };
    if path.starts_with("mock://") {
        return Err("ASSET_NOT_LOCAL: asset has no local file".into());
    }
    let file = std::path::Path::new(&path);
    if !file.exists() {
        return Err("ASSET_NOT_FOUND: local file does not exist".into());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(file)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(file)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = file.parent().unwrap_or(file);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    Ok(Some(path))
}

#[tauri::command]
pub fn asset_export(id: String, state: State<'_, AppState>) -> Result<String, String> {
    let asset = {
        let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
        assets.list().into_iter().find(|asset| asset.id == id)
    }
    .ok_or_else(|| "ASSET_NOT_FOUND: asset metadata missing".to_string())?;
    if asset.local_path.starts_with("mock://") {
        return Err("ASSET_NOT_LOCAL: asset has no local file".into());
    }
    let source = std::path::Path::new(&asset.local_path);
    let media = state.media.lock().map_err(|_| "media lock poisoned")?;
    if !media.is_under_root(source) {
        return Err("ASSET_EXPORT_BLOCKED: path is outside the asset store".into());
    }
    if !source.exists() {
        return Err("ASSET_NOT_FOUND: local file does not exist".into());
    }
    let downloads = std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Downloads");
    std::fs::create_dir_all(&downloads).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let destination = downloads.join(format!("{id}.{extension}"));
    std::fs::copy(source, &destination).map_err(|error| format!("DOWNLOAD_FAILED: {error}"))?;
    Ok(destination.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn storage_usage(state: State<'_, AppState>) -> Result<u64, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    Ok(assets.usage())
}

#[tauri::command]
pub fn settings_get(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database.get_settings().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn settings_update(
    update: crate::persistence::SettingsUpdate,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    for (key, value) in &update.values {
        let raw = serde_json::to_string(value).map_err(|error| error.to_string())?;
        database
            .set_setting(key, &raw)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
