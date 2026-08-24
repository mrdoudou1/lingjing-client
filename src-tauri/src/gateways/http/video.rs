use super::*;
use crate::domain::VideoRequest;

impl GatewayHttpClient {
    pub async fn create_video(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &VideoRequest,
    ) -> Result<RemoteVideoStatus, String> {
        if profile.base_url.starts_with("mock://") {
            return Err("MOCK_GATEWAY: video remains local mock".into());
        }
        let endpoint = crate::gateways::adapters::adapter_for(profile).endpoint("videos");
        let url = format!("{}/{endpoint}", profile.base_url.trim_end_matches('/'));
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
        let endpoint = crate::gateways::adapters::adapter_for(profile)
            .endpoint(&format!("videos/{remote_id}"));
        let url = format!("{}/{endpoint}", profile.base_url.trim_end_matches('/'));
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
        let endpoint = crate::gateways::adapters::adapter_for(profile)
            .endpoint(&format!("videos/{remote_id}"));
        let url = format!("{}/{endpoint}", profile.base_url.trim_end_matches('/'));
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

    pub(crate) fn parse_video_status(
        payload: serde_json::Value,
    ) -> Result<RemoteVideoStatus, String> {
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
