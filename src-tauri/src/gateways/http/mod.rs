use crate::domain::{AudioRequest, GatewayProfile, ImageRequest};
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
    pub(crate) fn url(profile: &GatewayProfile, endpoint: &str) -> String {
        let base = profile.base_url.trim_end_matches('/');
        let endpoint = crate::gateways::adapters::adapter_for(profile).endpoint(endpoint);
        let endpoint = if base.ends_with("/v1") && endpoint.starts_with("v1/") {
            endpoint.trim_start_matches("v1/")
        } else {
            endpoint.as_str()
        };
        format!("{base}/{endpoint}")
    }
    fn models_url(profile: &GatewayProfile) -> String {
        Self::url(profile, "models")
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
}

mod chat;
mod media;
#[cfg(test)]
mod tests;
mod video;
