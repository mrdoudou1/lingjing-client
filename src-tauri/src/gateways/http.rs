use crate::domain::{AudioRequest, GatewayProfile, ImageRequest, VideoRequest};
use futures_util::{future::Either, FutureExt, StreamExt};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Instant;
use tokio::sync::watch;

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Option<Vec<ModelItem>>,
}
#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImageResult {
    pub url: Option<String>,
    pub b64_json: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TranscriptResult {
    pub text: String,
    pub raw: String,
}

#[derive(Debug, Clone)]
pub struct RemoteVideoStatus {
    pub remote_id: String,
    pub status: String,
    pub progress: f32,
    pub result_url: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ImageResponse {
    data: Vec<ImageResult>,
}

pub struct GatewayHttpClient {
    client: Client,
}
impl Default for GatewayHttpClient {
    fn default() -> Self {
        Self {
            client: Client::new(),
        }
    }
}

impl GatewayHttpClient {
    fn models_url(profile: &GatewayProfile) -> String {
        format!("{}/models", profile.base_url.trim_end_matches('/'))
    }
    fn request(&self, profile: &GatewayProfile, api_key: Option<&str>) -> reqwest::RequestBuilder {
        let request = self
            .client
            .get(Self::models_url(profile))
            .header("Accept", "application/json");
        match api_key.filter(|key| !key.is_empty()) {
            Some(key) => request.bearer_auth(key),
            None => request,
        }
    }
    pub async fn test_connection(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        if profile.base_url.starts_with("mock://") {
            return Ok(serde_json::json!({ "ok": true, "latencyMs": 42, "protocol": "mock" }));
        }
        let started = Instant::now();
        let response = self
            .request(profile, api_key)
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        let status = response.status();
        let latency = started.elapsed().as_millis();
        if status.is_success() {
            Ok(serde_json::json!({ "ok": true, "latencyMs": latency, "status": status.as_u16() }))
        } else {
            Err(Self::status_error(status))
        }
    }
    pub async fn list_models(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
    ) -> Result<Vec<String>, String> {
        if profile.base_url.starts_with("mock://") {
            return Ok(vec![
                "gpt-4.1".into(),
                "grok-imagine-image-2.0".into(),
                "grok-imagine-video".into(),
                "mock-audio".into(),
            ]);
        }
        let response = self
            .request(profile, api_key)
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !status.is_success() {
            return Err(Self::status_error(status));
        }
        let payload: ModelsResponse = response
            .json()
            .await
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
        Ok(payload
            .data
            .unwrap_or_default()
            .into_iter()
            .map(|item| item.id)
            .collect())
    }
    fn status_error(status: StatusCode) -> String {
        if status.is_server_error() {
            format!("GATEWAY_5XX: HTTP {}", status.as_u16())
        } else if status == StatusCode::TOO_MANY_REQUESTS {
            "RATE_LIMITED: gateway rate limit".into()
        } else {
            format!("GATEWAY_HTTP: HTTP {}", status.as_u16())
        }
    }

    pub async fn chat_stream<F>(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        model_id: &str,
        content: &str,
        mut stop: watch::Receiver<bool>,
        mut on_delta: F,
    ) -> Result<(), String>
    where
        F: FnMut(String) -> Result<(), String>,
    {
        if profile.base_url.starts_with("mock://") {
            let reply = format!(
                "已收到你的请求：**{}**\n\n这是 Rust Mock Gateway 的桌面流式响应。",
                content.trim()
            );
            for chunk in reply.as_bytes().chunks(3) {
                if *stop.borrow() {
                    return Err("CANCELED: chat stream stopped".into());
                }
                on_delta(String::from_utf8_lossy(chunk).to_string())?;
                tokio::task::yield_now().await;
            }
            return Ok(());
        }
        let url = format!(
            "{}/chat/completions",
            profile.base_url.trim_end_matches('/')
        );
        let mut request = self.client.post(url).header("Accept", "text/event-stream").json(&serde_json::json!({"model": model_id, "stream": true, "messages": [{"role": "user", "content": content}]}));
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            request = request.bearer_auth(key);
        }
        let response = request
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !status.is_success() {
            return Err(Self::status_error(status));
        }
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        loop {
            let (next, stop_notified) = {
                let stream_next = stream.next().fuse();
                let stop_changed = stop.changed().fuse();
                futures_util::pin_mut!(stream_next, stop_changed);
                match futures_util::future::select(stream_next, stop_changed).await {
                    Either::Left((chunk, _)) => (chunk, false),
                    Either::Right((changed, _)) => (None, changed.is_ok()),
                }
            };
            if stop_notified && *stop.borrow() {
                return Err("CANCELED: chat stream stopped".into());
            }
            let Some(chunk) = next else { break };
            let chunk = chunk.map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
            buffer.push_str(&String::from_utf8_lossy(&chunk));
            while let Some(index) = buffer.find("\n") {
                let line = buffer.drain(..=index).collect::<String>();
                let data = line.trim().strip_prefix("data:").map(str::trim);
                let Some(data) = data else { continue };
                if data == "[DONE]" {
                    return Ok(());
                }
                let payload: serde_json::Value = serde_json::from_str(data)
                    .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
                if let Some(delta) = payload
                    .pointer("/choices/0/delta/content")
                    .and_then(|value| value.as_str())
                {
                    if *stop.borrow() {
                        return Err("CANCELED: chat stream stopped".into());
                    }
                    on_delta(delta.to_string())?;
                }
            }
        }
        Ok(())
    }

    pub async fn generate_images(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &ImageRequest,
    ) -> Result<Vec<ImageResult>, String> {
        if profile.base_url.starts_with("mock://") {
            return Ok(Vec::new());
        }
        let url = format!(
            "{}/images/generations",
            profile.base_url.trim_end_matches('/')
        );
        let size = match request.aspect_ratio.as_deref() {
            Some("16:9") => "1792x1024",
            Some("9:16") => "1024x1792",
            _ if request.resolution.as_deref() == Some("2k") => "2048x2048",
            _ => "1024x1024",
        };
        let mut builder = self.client.post(url).json(&serde_json::json!({
            "model": request.model_id,
            "prompt": request.prompt,
            "n": request.count,
            "size": size,
            "quality": request.quality,
            "response_format": "b64_json"
        }));
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        let status = response.status();
        if status == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !status.is_success() {
            return Err(Self::status_error(status));
        }
        let payload: ImageResponse = response
            .json()
            .await
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
        Ok(payload.data)
    }

    pub async fn download_bytes(
        &self,
        url: &str,
        api_key: Option<&str>,
    ) -> Result<Vec<u8>, String> {
        let mut builder = self.client.get(url);
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("DOWNLOAD_FAILED: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "DOWNLOAD_FAILED: HTTP {}",
                response.status().as_u16()
            ));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| format!("DOWNLOAD_FAILED: {error}"))
    }

    pub async fn synthesize_speech(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &AudioRequest,
    ) -> Result<Vec<u8>, String> {
        if profile.base_url.starts_with("mock://") {
            return Ok(format!(
                "Mock speech: {}",
                request.text.as_deref().unwrap_or_default()
            )
            .into_bytes());
        }
        let url = format!("{}/audio/speech", profile.base_url.trim_end_matches('/'));
        let mut builder = self.client.post(url).json(&serde_json::json!({
            "model": request.model_id,
            "input": request.text,
            "voice": request.voice,
            "response_format": request.format.to_lowercase(),
            "speed": request.speed
        }));
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !response.status().is_success() {
            return Err(Self::status_error(response.status()));
        }
        response
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))
    }

    pub async fn transcribe_audio(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &AudioRequest,
    ) -> Result<TranscriptResult, String> {
        let encoded = request
            .source_file_base64
            .as_deref()
            .ok_or_else(|| "VALIDATION_FAILED: audio file content is missing".to_string())?;
        let bytes = base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|error| format!("VALIDATION_FAILED: invalid audio payload: {error}"))?;
        if profile.base_url.starts_with("mock://") {
            return Ok(TranscriptResult {
                text: format!(
                    "Mock transcript for {}",
                    request.source_file_name.as_deref().unwrap_or("audio")
                ),
                raw: String::new(),
            });
        }
        let url = format!(
            "{}/audio/transcriptions",
            profile.base_url.trim_end_matches('/')
        );
        let file_name = request
            .source_file_name
            .as_deref()
            .unwrap_or("audio.bin")
            .to_string();
        let boundary = format!("----LingjingBoundary{}", uuid::Uuid::new_v4().simple());
        let mut body = Vec::new();
        let mut append_text = |name: &str, value: &str| {
            body.extend_from_slice(format!("--{boundary}\\r\\nContent-Disposition: form-data; name=\\\"{name}\\\"\\r\\n\\r\\n{value}\\r\\n").as_bytes());
        };
        append_text("model", &request.model_id);
        append_text("response_format", &request.format.to_lowercase());
        if let Some(language) = request.language.as_deref() {
            append_text("language", language);
        }
        body.extend_from_slice(format!("--{boundary}\\r\\nContent-Disposition: form-data; name=\\\"file\\\"; filename=\\\"{file_name}\\\"\\r\\nContent-Type: application/octet-stream\\r\\n\\r\\n").as_bytes());
        body.extend_from_slice(&bytes);
        body.extend_from_slice(format!("\\r\\n--{boundary}--\\r\\n").as_bytes());
        let mut builder = self
            .client
            .post(url)
            .header(
                "Content-Type",
                format!("multipart/form-data; boundary={boundary}"),
            )
            .body(body);
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !response.status().is_success() {
            return Err(Self::status_error(response.status()));
        }
        let raw = response
            .text()
            .await
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
        let text = serde_json::from_str::<serde_json::Value>(&raw)
            .ok()
            .and_then(|value| {
                value
                    .get("text")
                    .and_then(|value| value.as_str())
                    .map(str::to_string)
            })
            .unwrap_or_else(|| raw.clone());
        Ok(TranscriptResult { text, raw })
    }

    pub async fn create_video(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &VideoRequest,
    ) -> Result<RemoteVideoStatus, String> {
        if profile.base_url.starts_with("mock://") {
            return Err("MOCK_GATEWAY: video remains local mock".into());
        }
        let url = format!("{}/videos", profile.base_url.trim_end_matches('/'));
        let mut builder = self.client.post(url).json(&serde_json::json!({
            "model": request.model_id,
            "prompt": request.prompt,
            "operation": request.operation,
            "duration": request.duration_sec,
            "extension_duration": request.extension_duration_sec,
            "aspect_ratio": request.aspect_ratio,
            "resolution": request.resolution,
            "first_frame_asset_id": request.first_frame_asset_id,
            "reference_image_asset_ids": request.reference_image_asset_ids,
            "reference_voice_ids": request.reference_voice_ids,
            "source_video_asset_id": request.source_video_asset_id
        }));
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err("AUTH_INVALID: gateway rejected API key".into());
        }
        if !response.status().is_success() {
            return Err(Self::status_error(response.status()));
        }
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
        Self::parse_video_status(payload)
    }

    pub async fn get_video_status(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        remote_id: &str,
    ) -> Result<RemoteVideoStatus, String> {
        let url = format!(
            "{}/videos/{remote_id}",
            profile.base_url.trim_end_matches('/')
        );
        let mut builder = self.client.get(url);
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        if !response.status().is_success() {
            return Err(Self::status_error(response.status()));
        }
        let payload: serde_json::Value = response
            .json()
            .await
            .map_err(|error| format!("GATEWAY_INVALID_RESPONSE: {error}"))?;
        Self::parse_video_status(payload)
    }

    pub async fn cancel_video(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        remote_id: &str,
    ) -> Result<(), String> {
        let url = format!(
            "{}/videos/{remote_id}",
            profile.base_url.trim_end_matches('/')
        );
        let mut builder = self.client.delete(url);
        if let Some(key) = api_key.filter(|key| !key.is_empty()) {
            builder = builder.bearer_auth(key);
        }
        let response = builder
            .send()
            .await
            .map_err(|error| format!("NETWORK_OFFLINE: {error}"))?;
        if !response.status().is_success() && response.status() != StatusCode::NOT_FOUND {
            return Err(Self::status_error(response.status()));
        }
        Ok(())
    }

    fn parse_video_status(payload: serde_json::Value) -> Result<RemoteVideoStatus, String> {
        let remote_id = payload
            .get("id")
            .or_else(|| payload.get("job_id"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| "GATEWAY_INVALID_RESPONSE: video id missing".to_string())?
            .to_string();
        let status = payload
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("queued")
            .to_lowercase();
        let progress = payload
            .get("progress")
            .and_then(|value| value.as_f64())
            .unwrap_or_else(|| {
                if status == "succeeded" || status == "completed" {
                    100.0
                } else {
                    0.0
                }
            }) as f32;
        let result_url = payload
            .get("url")
            .or_else(|| payload.get("video_url"))
            .or_else(|| payload.pointer("/output/url"))
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let error_message = payload
            .get("error")
            .or_else(|| payload.get("message"))
            .and_then(|value| value.as_str())
            .map(str::to_string);
        Ok(RemoteVideoStatus {
            remote_id,
            status,
            progress,
            result_url,
            error_message,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mock_chat_stream_honors_stop_signal() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let profile = GatewayProfile {
            id: "mock-default".into(),
            name: "Mock Gateway".into(),
            base_url: "mock://local".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:mock-default".into(),
            enabled: true,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        let (_stop_tx, stop_rx) = watch::channel(true);
        let mut deltas = 0;
        let result = runtime.block_on(GatewayHttpClient::default().chat_stream(
            &profile,
            None,
            "gpt-4.1",
            "hello",
            stop_rx,
            |_| {
                deltas += 1;
                Ok(())
            },
        ));
        assert_eq!(result, Err("CANCELED: chat stream stopped".into()));
        assert_eq!(deltas, 0);
    }

    #[test]
    fn mock_audio_adapters_return_deterministic_results() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime should build");
        let profile = GatewayProfile {
            id: "mock-default".into(),
            name: "Mock Gateway".into(),
            base_url: "mock://local".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:mock-default".into(),
            enabled: true,
            is_default: true,
            created_at: None,
            updated_at: None,
        };
        let tts = AudioRequest {
            gateway_profile_id: "mock-default".into(),
            model_id: "mock-audio".into(),
            kind: "tts".into(),
            text: Some("hello".into()),
            source_file_name: None,
            source_file_base64: None,
            voice: Some("Aria".into()),
            language: None,
            format: "MP3".into(),
            speed: None,
        };
        let stt = AudioRequest {
            kind: "stt".into(),
            source_file_name: Some("clip.wav".into()),
            source_file_base64: Some("aGk=".into()),
            ..tts.clone()
        };
        runtime.block_on(async {
            let speech = GatewayHttpClient::default()
                .synthesize_speech(&profile, None, &tts)
                .await
                .unwrap();
            assert!(String::from_utf8(speech).unwrap().contains("hello"));
            let transcript = GatewayHttpClient::default()
                .transcribe_audio(&profile, None, &stt)
                .await
                .unwrap();
            assert!(transcript.text.contains("clip.wav"));
        });
    }

    #[test]
    fn video_status_parser_accepts_common_gateway_fields() {
        let status = GatewayHttpClient::parse_video_status(serde_json::json!({
            "job_id": "remote-1",
            "status": "completed",
            "progress": 100,
            "output": { "url": "https://example.test/video.mp4" }
        }))
        .expect("status should parse");
        assert_eq!(status.remote_id, "remote-1");
        assert_eq!(status.status, "completed");
        assert_eq!(status.progress, 100.0);
        assert_eq!(
            status.result_url.as_deref(),
            Some("https://example.test/video.mp4")
        );
    }
}
