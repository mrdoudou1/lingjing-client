use super::{emit_asset_ready, emit_job_event, persist_asset, persist_job};
use crate::{domain::AudioRequest, AppState};
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

fn supported_format(value: &str, values: &serde_json::Value) -> bool {
    values
        .as_array()
        .map(|items| {
            items.iter().any(|item| {
                item.as_str()
                    .is_some_and(|candidate| candidate.eq_ignore_ascii_case(value))
            })
        })
        .unwrap_or(false)
}

fn normalized_language(value: &str) -> &str {
    match value {
        "中文（普通话）" | "中文" => "zh",
        "English" | "英语" => "en",
        other => other,
    }
}

fn normalized_voice(value: &str) -> &str {
    value.split(" · ").next().unwrap_or(value)
}

fn validate_tts_request(
    request: &AudioRequest,
    capabilities: &serde_json::Value,
) -> Result<(), String> {
    let tts = capabilities.get("tts").ok_or_else(|| {
        "CAPABILITY_UNSUPPORTED: model does not support speech synthesis".to_string()
    })?;
    if request.text.as_deref().unwrap_or("").trim().is_empty() {
        return Err("VALIDATION_FAILED: speech text is empty".into());
    }
    if !supported_format(
        &request.format,
        tts.get("formats").unwrap_or(&serde_json::Value::Null),
    ) {
        return Err("VALIDATION_FAILED: audio format is not supported by model".into());
    }
    if let Some(voice) = request.voice.as_deref() {
        if !supported_format(
            normalized_voice(voice),
            tts.get("voices").unwrap_or(&serde_json::Value::Null),
        ) {
            return Err("VALIDATION_FAILED: voice is not supported by model".into());
        }
    }
    if let Some(speed) = request.speed {
        if !(0.25..=4.0).contains(&speed) {
            return Err("VALIDATION_FAILED: speech speed must be between 0.25 and 4.0".into());
        }
    }
    Ok(())
}

fn validate_stt_request(
    request: &AudioRequest,
    capabilities: &serde_json::Value,
) -> Result<(), String> {
    let stt = capabilities.get("stt").ok_or_else(|| {
        "CAPABILITY_UNSUPPORTED: model does not support transcription".to_string()
    })?;
    let file_name = request.source_file_name.as_deref().unwrap_or("");
    if file_name.trim().is_empty()
        || !matches!(
            file_name
                .rsplit('.')
                .next()
                .map(str::to_ascii_lowercase)
                .as_deref(),
            Some("mp3" | "wav" | "m4a" | "mp4" | "mov" | "webm" | "ogg")
        )
    {
        return Err("VALIDATION_FAILED: unsupported audio or video file".into());
    }
    if request
        .source_file_base64
        .as_deref()
        .unwrap_or("")
        .is_empty()
    {
        return Err("VALIDATION_FAILED: audio file content is missing".into());
    }
    base64::Engine::decode(
        &base64::engine::general_purpose::STANDARD,
        request.source_file_base64.as_deref().unwrap_or(""),
    )
    .map_err(|error| format!("VALIDATION_FAILED: invalid audio payload: {error}"))?;
    if !supported_format(
        &request.format,
        stt.get("formats").unwrap_or(&serde_json::Value::Null),
    ) {
        return Err("VALIDATION_FAILED: transcription format is not supported by model".into());
    }
    if let Some(language) = request.language.as_deref() {
        let normalized = normalized_language(language);
        if !supported_format(
            normalized,
            stt.get("languages").unwrap_or(&serde_json::Value::Null),
        ) {
            return Err("VALIDATION_FAILED: language is not supported by model".into());
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn audio_tts(
    app: AppHandle,
    request: AudioRequest,
    state: State<'_, AppState>,
) -> Result<crate::domain::GenerationJob, String> {
    if request.kind != "tts" || request.text.as_deref().unwrap_or("").trim().is_empty() {
        return Err("请输入需要合成的文本".into());
    }
    let capabilities = model_capabilities(
        state.inner(),
        &request.gateway_profile_id,
        &request.model_id,
    )?;
    validate_tts_request(&request, &capabilities)?;
    let (job, profile, key) = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        let profile = registry
            .profile(&request.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?;
        if !profile.enabled {
            return Err("GATEWAY_DISABLED: gateway profile is disabled".into());
        }
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

#[cfg(test)]
mod tests {
    use super::{validate_stt_request, validate_tts_request};
    use crate::domain::AudioRequest;

    fn capabilities() -> serde_json::Value {
        serde_json::json!({
            "tts": { "voices": ["Aria"], "formats": ["MP3"] },
            "stt": { "languages": ["zh", "en"], "formats": ["TXT", "JSON"] }
        })
    }

    #[test]
    fn accepts_display_voice_label_and_supported_tts_values() {
        let request = AudioRequest {
            gateway_profile_id: "mock-default".into(),
            model_id: "mock-audio".into(),
            kind: "tts".into(),
            text: Some("hello".into()),
            source_file_name: None,
            source_file_base64: None,
            voice: Some("Aria · 温暖女声".into()),
            language: None,
            format: "mp3".into(),
            speed: Some(1.0),
        };
        assert!(validate_tts_request(&request, &capabilities()).is_ok());
    }

    #[test]
    fn rejects_invalid_stt_payload_or_file() {
        let request = AudioRequest {
            gateway_profile_id: "mock-default".into(),
            model_id: "mock-audio".into(),
            kind: "stt".into(),
            text: None,
            source_file_name: Some("clip.exe".into()),
            source_file_base64: Some("not-base64".into()),
            voice: None,
            language: Some("中文（普通话）".into()),
            format: "TXT".into(),
            speed: None,
        };
        assert!(validate_stt_request(&request, &capabilities()).is_err());
    }
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
    let capabilities = model_capabilities(
        state.inner(),
        &request.gateway_profile_id,
        &request.model_id,
    )?;
    validate_stt_request(&request, &capabilities)?;
    let (job, profile, key) = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        let profile = registry
            .profile(&request.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?;
        if !profile.enabled {
            return Err("GATEWAY_DISABLED: gateway profile is disabled".into());
        }
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
