use super::*;

impl GatewayHttpClient {
    pub async fn generate_images(
        &self,
        profile: &GatewayProfile,
        api_key: Option<&str>,
        request: &ImageRequest,
    ) -> Result<Vec<ImageResult>, String> {
        if profile.base_url.starts_with("mock://") {
            return Ok(Vec::new());
        }
        let url = super::GatewayHttpClient::url(profile, "images/generations");
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
        let url = super::GatewayHttpClient::url(profile, "audio/speech");
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
        let url = super::GatewayHttpClient::url(profile, "audio/transcriptions");
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
}
