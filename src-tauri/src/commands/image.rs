use super::{emit_asset_ready, emit_job_event, persist_asset, persist_job};
use crate::{domain::ImageRequest, AppState};
use tauri::{AppHandle, State};

#[tauri::command]
pub async fn image_create_job(
    app: AppHandle,
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
    emit_job_event(&app, "job://created", &inserted);
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
                emit_job_event(&app, "job://failed", &failed);
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
        let asset = crate::domain::Asset {
            id: asset_id.clone(),
            job_id: Some(inserted.id),
            kind: "image".into(),
            mime_type: "image/png".into(),
            local_path: path.to_string_lossy().into_owned(),
            thumbnail_path: None,
            size_bytes: bytes.len() as u64,
            favorite: false,
            created_at: chrono::Utc::now(),
        };
        persist_asset(state.inner(), &asset)?;
        emit_asset_ready(&app, &asset);
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
    emit_job_event(&app, "job://status", &completed);
    Ok(completed)
}
