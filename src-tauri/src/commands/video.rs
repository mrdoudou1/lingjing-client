use super::{emit_asset_ready, emit_job_event, persist_asset, persist_job};
use crate::{domain::VideoRequest, AppState};
use tauri::{AppHandle, State};
use uuid::Uuid;

fn validate_video_request(
    request: &VideoRequest,
    capabilities: &serde_json::Value,
) -> Result<(), String> {
    if request.prompt.trim().is_empty() {
        return Err("VALIDATION_FAILED: video prompt is empty".into());
    }
    let video = capabilities
        .get("video")
        .ok_or_else(|| "CAPABILITY_UNSUPPORTED: model does not support video".to_string())?;
    let operation_supported = video
        .get("operations")
        .and_then(|value| value.as_array())
        .map(|values| {
            values
                .iter()
                .any(|value| value.as_str() == Some(request.operation.as_str()))
        })
        .unwrap_or(false);
    if !operation_supported {
        return Err("CAPABILITY_UNSUPPORTED: video operation is not supported".into());
    }
    if request.first_frame_asset_id.is_some() && !request.reference_image_asset_ids.is_empty() {
        return Err(
            "VALIDATION_FAILED: first frame and reference images are mutually exclusive".into(),
        );
    }
    if matches!(request.operation.as_str(), "edit" | "extend")
        && request
            .source_video_asset_id
            .as_deref()
            .unwrap_or("")
            .trim()
            .is_empty()
    {
        return Err("VALIDATION_FAILED: edit or extend requires a source video".into());
    }
    let max_images = video
        .get("maxReferenceImages")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    if request.reference_image_asset_ids.len() > max_images {
        return Err("VALIDATION_FAILED: reference image count exceeds model limit".into());
    }
    let max_voices = video
        .get("maxReferenceVoices")
        .and_then(|value| value.as_u64())
        .unwrap_or(0) as usize;
    if request.reference_voice_ids.len() > max_voices {
        return Err("VALIDATION_FAILED: reference voice count exceeds model limit".into());
    }
    if let Some(duration) = request.duration_sec {
        if let Some(values) = video.get("durations").and_then(|value| value.as_array()) {
            if !values
                .iter()
                .any(|value| value.as_u64() == Some(duration as u64))
            {
                return Err("VALIDATION_FAILED: duration is not supported by model".into());
            }
        }
    }
    if let Some(aspect_ratio) = request.aspect_ratio.as_deref() {
        if let Some(values) = video.get("aspectRatios").and_then(|value| value.as_array()) {
            if !values
                .iter()
                .any(|value| value.as_str() == Some(aspect_ratio))
            {
                return Err("VALIDATION_FAILED: aspect ratio is not supported by model".into());
            }
        }
    }
    if let Some(resolution) = request.resolution.as_deref() {
        if let Some(values) = video.get("resolutions").and_then(|value| value.as_array()) {
            if !values
                .iter()
                .any(|value| value.as_str() == Some(resolution))
            {
                return Err("VALIDATION_FAILED: resolution is not supported by model".into());
            }
        }
    }
    Ok(())
}

fn model_capabilities(
    state: &AppState,
    profile_id: &str,
    model_id: &str,
) -> Result<serde_json::Value, String> {
    let snapshot = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .list_model_snapshots(profile_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|snapshot| snapshot.model_id == model_id);
    Ok(snapshot
        .map(|snapshot| snapshot.capabilities_json)
        .unwrap_or_else(|| crate::gateways::GatewayRegistry::capabilities_for_model(model_id)))
}

#[tauri::command]
pub async fn video_create_job(
    app: AppHandle,
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
    let capabilities = model_capabilities(
        state.inner(),
        &request.gateway_profile_id,
        &request.model_id,
    )?;
    validate_video_request(&request, &capabilities)?;
    let inserted = {
        let mut jobs = state.jobs.lock().map_err(|_| "job lock poisoned")?;
        jobs.insert(job)
    };
    persist_job(state.inner(), &inserted)?;
    emit_job_event(&app, "job://created", &inserted);
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
    emit_job_event(&app, "job://status", &updated);
    Ok(updated)
}

#[tauri::command]
pub async fn video_poll_job(
    app: AppHandle,
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
        let asset = crate::domain::Asset {
            id: asset_id,
            job_id: Some(job.id),
            kind: "video".into(),
            mime_type: "video/mp4".into(),
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
    let updated = jobs
        .update(job.id, status, remote.progress, remote.error_message)
        .ok_or_else(|| "job disappeared".to_string())?;
    drop(jobs);
    persist_job(state.inner(), &updated)?;
    emit_job_event(
        &app,
        if matches!(updated.status, crate::domain::JobStatus::Failed) {
            "job://failed"
        } else {
            "job://status"
        },
        &updated,
    );
    Ok(Some(updated))
}

#[cfg(test)]
mod tests {
    use super::validate_video_request;
    use crate::domain::VideoRequest;

    fn request() -> VideoRequest {
        VideoRequest {
            gateway_profile_id: "mock-default".into(),
            model_id: "mock-video".into(),
            operation: "generate".into(),
            prompt: "a quiet lake".into(),
            source_video_asset_id: None,
            first_frame_asset_id: None,
            reference_image_asset_ids: Vec::new(),
            reference_voice_ids: Vec::new(),
            duration_sec: Some(6),
            extension_duration_sec: None,
            aspect_ratio: Some("16:9".into()),
            resolution: Some("720p".into()),
        }
    }

    fn capabilities() -> serde_json::Value {
        serde_json::json!({
            "video": {
                "operations": ["generate", "edit", "extend"],
                "durations": [6, 12],
                "aspectRatios": ["16:9"],
                "resolutions": ["720p"],
                "maxReferenceImages": 1,
                "maxReferenceVoices": 1
            }
        })
    }

    #[test]
    fn validates_supported_video_request() {
        assert!(validate_video_request(&request(), &capabilities()).is_ok());
    }

    #[test]
    fn rejects_mutually_exclusive_inputs_and_invalid_capabilities() {
        let mut invalid = request();
        invalid.first_frame_asset_id = Some("frame".into());
        invalid.reference_image_asset_ids = vec!["ref".into()];
        assert!(validate_video_request(&invalid, &capabilities()).is_err());
        let mut unsupported = request();
        unsupported.duration_sec = Some(24);
        assert!(validate_video_request(&unsupported, &capabilities()).is_err());
    }
}
