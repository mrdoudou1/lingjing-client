use super::*;

impl GatewayHttpClient {
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
        let endpoint = crate::gateways::adapters::adapter_for(profile).endpoint("chat/completions");
        let url = format!("{}/{endpoint}", profile.base_url.trim_end_matches('/'));
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
}
