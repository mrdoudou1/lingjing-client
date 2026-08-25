use super::*;
use crate::domain::VideoRequest;
use crate::gateways::adapters::adapter_for;

impl GatewayHttpClient {
    pub async fn create_video(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &VideoRequest,
        image_data: Option<&str>,
    ) -> Result<RemoteVideoStatus, String> {
        if profile.base_url.starts_with("mock://") {
            return Err("MOCK_GATEWAY: video remains local mock".into());
        }
        let adapter = adapter_for(profile);
        let endpoint = adapter.video_endpoint(&request.model_id);
        let url = super::GatewayHttpClient::url(profile, endpoint);
        let (width, height) = match request.aspect_ratio.as_deref() {
            Some("9:16") => (720, 1280),
            Some("1:1") => (1024, 1024),
            _ => match request.resolution.as_deref() {
                Some("1080p") => (1920, 1080),
                _ => (1280, 720),
            },
        };
        let size = match (width, height) {
            (720, 1280) => "720x1280",
            (1024, 1024) => "1024x1024",
            (1920, 1080) => "1920x1080",
            _ => "1280x720",
        };
        let mut builder = self.client.post(url);
        if endpoint == "videos" {
            let seconds = request
                .duration_sec
                .or(request.extension_duration_sec)
                .unwrap_or(6);
            let boundary = format!("----LingjingVideo{}", uuid::Uuid::new_v4().simple());
            let mut body = Vec::new();
            let mut append_text = |name: &str, value: &str| {
                body.extend_from_slice(
                    format!(
                        "--{boundary}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
                    )
                    .as_bytes(),
                );
            };
            append_text("model", &request.model_id);
            append_text("prompt", &request.prompt);
            append_text("seconds", &seconds.to_string());
            append_text("size", size);
            if let Some(image) = image_data {
                if let Some((header, encoded)) = image.split_once(',') {
                    let bytes =
                        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
                            .map_err(|error| {
                                format!("VALIDATION_FAILED: invalid reference image: {error}")
                            })?;
                    let mime = header
                        .strip_prefix("data:")
                        .and_then(|value| value.strip_suffix(";base64"))
                        .unwrap_or("image/png");
                    body.extend_from_slice(
                        format!(
                            "--{boundary}\r\nContent-Disposition: form-data; name=\"input_reference\"; filename=\"reference.png\"\r\nContent-Type: {mime}\r\n\r\n"
                        )
                        .as_bytes(),
                    );
                    body.extend_from_slice(&bytes);
                    body.extend_from_slice(b"\r\n");
                }
            }
            body.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
            builder = builder
                .header(
                    "Content-Type",
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(body);
        } else {
            builder = builder.json(&serde_json::json!({
                "model": request.model_id,
                "prompt": request.prompt,
                "image": image_data,
                "duration": request.duration_sec.or(request.extension_duration_sec),
                "width": width,
                "height": height,
                "response_format": "url",
                "metadata": {
                    "operation": request.operation,
                    "size": size,
                    "aspect_ratio": request.aspect_ratio,
                    "resolution": request.resolution,
                    "first_frame_asset_id": request.first_frame_asset_id,
                    "reference_image_asset_ids": request.reference_image_asset_ids,
                    "reference_voice_ids": request.reference_voice_ids,
                    "source_video_asset_id": request.source_video_asset_id
                }
            }));
        }
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
        model_id: &str,
    ) -> Result<RemoteVideoStatus, String> {
        let endpoint = adapter_for(profile).video_endpoint(model_id);
        let url = super::GatewayHttpClient::url(profile, &format!("{endpoint}/{remote_id}"));
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
        model_id: &str,
    ) -> Result<(), String> {
        let endpoint = adapter_for(profile).video_endpoint(model_id);
        let url = super::GatewayHttpClient::url(profile, &format!("{endpoint}/{remote_id}"));
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

    pub async fn download_video_content(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        remote_id: &str,
        model_id: &str,
    ) -> Result<Vec<u8>, String> {
        let endpoint = adapter_for(profile).video_endpoint(model_id);
        let url =
            super::GatewayHttpClient::url(profile, &format!("{endpoint}/{remote_id}/content"));
        self.download_bytes(&url, api_key).await
    }

    pub(crate) fn parse_video_status(
        payload: serde_json::Value,
    ) -> Result<RemoteVideoStatus, String> {
        let remote_id = payload
            .get("id")
            .or_else(|| payload.get("job_id"))
            .or_else(|| payload.get("task_id"))
            .or_else(|| payload.pointer("/data/id"))
            .or_else(|| payload.pointer("/job/id"))
            .and_then(|value| value.as_str())
            .ok_or_else(|| "GATEWAY_INVALID_RESPONSE: video id missing".to_string())?
            .to_string();
        let status = payload
            .get("status")
            .or_else(|| payload.pointer("/data/status"))
            .or_else(|| payload.pointer("/job/status"))
            .and_then(|value| value.as_str())
            .unwrap_or("queued")
            .to_lowercase();
        let progress = payload
            .get("progress")
            .or_else(|| payload.pointer("/data/progress"))
            .or_else(|| payload.pointer("/job/progress"))
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
            .or_else(|| payload.pointer("/output/video_url"))
            .or_else(|| payload.pointer("/data/url"))
            .or_else(|| payload.pointer("/data/output/url"))
            .and_then(|value| value.as_str())
            .map(str::to_string);
        let error_message = payload
            .get("error")
            .or_else(|| payload.get("message"))
            .or_else(|| payload.pointer("/data/error"))
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
