use super::{emit_asset_ready, emit_job_event, persist_asset, persist_job};
use crate::{domain::ImageRequest, AppState};
use tauri::{AppHandle, State};

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

fn validate_image_request(
    request: &ImageRequest,
    capabilities: &serde_json::Value,
) -> Result<(), String> {
    if request.prompt.trim().is_empty() {
        return Err("VALIDATION_FAILED: image prompt is empty".into());
    }
    let image = capabilities.get("image").ok_or_else(|| {
        "CAPABILITY_UNSUPPORTED: model does not support image generation".to_string()
    })?;
    let count = image.get("count").cloned().unwrap_or_default();
    let min = count
        .get("min")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);
    let max = count
        .get("max")
        .and_then(|value| value.as_u64())
        .unwrap_or(1);
    if (request.count as u64) < min || (request.count as u64) > max {
        return Err("VALIDATION_FAILED: image count is outside model limits".into());
    }
    if let Some(aspect_ratio) = request.aspect_ratio.as_deref() {
        if let Some(values) = image.get("aspectRatios").and_then(|value| value.as_array()) {
            if !values
                .iter()
                .any(|value| value.as_str() == Some(aspect_ratio))
            {
                return Err("VALIDATION_FAILED: aspect ratio is not supported by model".into());
            }
        }
    }
    if let Some(resolution) = request.resolution.as_deref() {
        if let Some(values) = image.get("resolutions").and_then(|value| value.as_array()) {
            if !values
                .iter()
                .any(|value| value.as_str() == Some(resolution))
            {
                return Err("VALIDATION_FAILED: resolution is not supported by model".into());
            }
        }
    }
    if let Some(quality) = request.quality.as_deref() {
        if let Some(values) = image.get("qualities").and_then(|value| value.as_array()) {
            if !values.iter().any(|value| value.as_str() == Some(quality)) {
                return Err("VALIDATION_FAILED: quality is not supported by model".into());
            }
        }
    }
    if !request.reference_asset_ids.is_empty()
        && image.get("supportsEdit").and_then(|value| value.as_bool()) != Some(true)
    {
        return Err("CAPABILITY_UNSUPPORTED: model does not support image references".into());
    }
    Ok(())
}

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
    let capabilities = model_capabilities(
        state.inner(),
        &request.gateway_profile_id,
        &request.model_id,
    )?;
    validate_image_request(&request, &capabilities)?;
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

#[cfg(test)]
mod tests {
    use super::validate_image_request;
    use crate::domain::ImageRequest;

    fn request() -> ImageRequest {
        ImageRequest {
            gateway_profile_id: "mock-default".into(),
            model_id: "gpt-image-1".into(),
            prompt: "a paper crane".into(),
            count: 2,
            aspect_ratio: Some("1:1".into()),
            resolution: Some("1k".into()),
            quality: Some("standard".into()),
            reference_asset_ids: Vec::new(),
        }
    }

    fn capabilities() -> serde_json::Value {
        serde_json::json!({
            "image": {
                "count": { "min": 1, "max": 4 },
                "aspectRatios": ["1:1"],
                "resolutions": ["1k"],
                "qualities": ["standard"],
                "supportsEdit": true
            }
        })
    }

    #[test]
    fn validates_supported_image_request() {
        assert!(validate_image_request(&request(), &capabilities()).is_ok());
    }

    #[test]
    fn rejects_invalid_count_and_reference_capability() {
        let mut invalid = request();
        invalid.count = 5;
        assert!(validate_image_request(&invalid, &capabilities()).is_err());
        let mut unsupported = request();
        unsupported.reference_asset_ids = vec!["ref".into()];
        let mut caps = capabilities();
        caps["image"]["supportsEdit"] = serde_json::Value::Bool(false);
        assert!(validate_image_request(&unsupported, &caps).is_err());
    }
}
