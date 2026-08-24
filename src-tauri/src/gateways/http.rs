use crate::domain::GatewayProfile;
use futures_util::StreamExt;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use std::time::Instant;

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    data: Option<Vec<ModelItem>>,
}
#[derive(Debug, Deserialize)]
struct ModelItem {
    id: String,
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
                on_delta(String::from_utf8_lossy(chunk).to_string())?;
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
        while let Some(chunk) = stream.next().await {
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
                    on_delta(delta.to_string())?;
                }
            }
        }
        Ok(())
    }
}
