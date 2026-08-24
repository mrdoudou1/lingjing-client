use crate::{domain::ChatSendInput, AppState};
use tauri::{AppHandle, Emitter, State};
use tokio::sync::watch;
use uuid::Uuid;

#[tauri::command]
pub fn chat_send(input: ChatSendInput) -> Result<serde_json::Value, String> {
    if input.content.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    let reply = format!(
        "已收到你的请求：**{}**\n\n这是 Rust Mock Gateway 的桌面流式响应。",
        input.content.trim()
    );
    Ok(
        serde_json::json!({"sessionId": input.session_id, "accepted": true, "modelId": input.model_id, "reply": reply}),
    )
}

#[tauri::command]
pub fn chat_list_sessions(
    state: State<'_, AppState>,
) -> Result<Vec<crate::domain::ChatSession>, String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .list_chat_sessions()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_save_session(
    session: crate::domain::ChatSession,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .save_chat_session(&session)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub fn chat_delete_session(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database
        .delete_chat_session(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn chat_stream(
    app: AppHandle,
    input: ChatSendInput,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    if input.content.trim().is_empty() {
        return Err("消息不能为空".into());
    }
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&input.gateway_profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    if !profile.enabled {
        return Err("GATEWAY_DISABLED: gateway profile is disabled".into());
    }
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    let correlation_id = Uuid::new_v4().to_string();
    let session_id = input.session_id.clone();
    let (stop_tx, stop_rx) = watch::channel(false);
    {
        let mut stops = state
            .chat_stops
            .lock()
            .map_err(|_| "chat stop lock poisoned")?;
        if let Some(previous) = stops.insert(session_id.clone(), stop_tx) {
            let _ = previous.send(true);
        }
    }
    let event_app = app.clone();
    let result = crate::gateways::http::GatewayHttpClient::default()
        .chat_stream(
            &profile,
            key.as_deref(),
            &input,
            stop_rx,
            |delta| {
                event_app
                    .emit(
                        "chat://delta",
                        serde_json::json!({ "sessionId": session_id, "delta": delta, "correlationId": correlation_id }),
                    )
                    .map_err(|error| error.to_string())
            },
        )
        .await;
    if let Ok(mut stops) = state.chat_stops.lock() {
        stops.remove(&input.session_id);
    }
    match result {
        Ok(()) => {
            app.emit(
                "chat://completed",
                serde_json::json!({ "sessionId": input.session_id, "correlationId": correlation_id }),
            )
            .map_err(|error| error.to_string())?;
            Ok(serde_json::json!({ "accepted": true, "correlationId": correlation_id }))
        }
        Err(error) => {
            let code = error
                .split(':')
                .next()
                .filter(|value| !value.is_empty())
                .unwrap_or("GATEWAY_ERROR");
            let _ = app.emit(
                "chat://error",
                serde_json::json!({ "sessionId": input.session_id, "correlationId": correlation_id, "code": code, "message": error }),
            );
            Err(error)
        }
    }
}

#[tauri::command]
pub fn chat_stop(session_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let stops = state
        .chat_stops
        .lock()
        .map_err(|_| "chat stop lock poisoned")?;
    Ok(stops
        .get(&session_id)
        .map(|sender| sender.send(true).is_ok())
        .unwrap_or(false))
}
