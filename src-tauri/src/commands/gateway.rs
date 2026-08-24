use crate::{domain::GatewayProfile, AppState};
use tauri::State;

#[tauri::command]
pub fn gateway_list_profiles(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    serde_json::to_value(registry.profiles()).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn gateway_test_connection(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    crate::gateways::http::GatewayHttpClient::default()
        .test_connection(&profile, key.as_deref())
        .await
}

#[tauri::command]
pub async fn gateway_refresh_models(
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    let profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    let key = state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .get(&profile.api_key_ref)?;
    let models = crate::gateways::http::GatewayHttpClient::default()
        .list_models(&profile, key.as_deref())
        .await?;
    let snapshots = models
        .iter()
        .map(|model| {
            (
                model.clone(),
                crate::gateways::GatewayRegistry::capabilities_for_model(model),
            )
        })
        .collect::<Vec<_>>();
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .save_model_snapshots(&profile.id, &snapshots)
        .map_err(|error| error.to_string())?;
    Ok(models)
}

#[tauri::command]
pub fn gateway_get_model_capabilities(
    profile_id: String,
    model_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let snapshot = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .list_model_snapshots(&profile_id)
        .map_err(|error| error.to_string())?
        .into_iter()
        .find(|snapshot| snapshot.model_id == model_id);
    Ok(snapshot
        .map(|value| value.capabilities_json)
        .unwrap_or_else(|| crate::gateways::GatewayRegistry::capabilities_for_model(&model_id)))
}

#[tauri::command]
pub fn gateway_create_profile(
    profile: GatewayProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .upsert_gateway_profile(&profile)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.create(profile);
    Ok(())
}

#[tauri::command]
pub fn gateway_update_profile(
    profile: GatewayProfile,
    state: State<'_, AppState>,
) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .upsert_gateway_profile(&profile)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.update(profile);
    Ok(())
}

#[tauri::command]
pub fn gateway_delete_profile(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let api_key_ref = state
        .gateways
        .lock()
        .map_err(|_| "gateway lock poisoned")?
        .profile(&id)
        .map(|profile| profile.api_key_ref);
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .delete_gateway_profile(&id)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.delete(&id);
    if let Some(reference) = api_key_ref {
        state
            .secrets
            .lock()
            .map_err(|_| "secret lock poisoned")?
            .remove(&reference)?;
    }
    Ok(())
}

#[tauri::command]
pub fn gateway_set_default(id: String, state: State<'_, AppState>) -> Result<(), String> {
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .set_default_gateway_profile(&id)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.set_default(&id);
    Ok(())
}

#[tauri::command]
pub fn gateway_set_api_key(
    profile_id: String,
    secret: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    if secret.trim().is_empty() {
        return Err("VALIDATION_FAILED: API key is empty".into());
    }
    let reference = format!("system-keychain:{profile_id}");
    state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .set(reference.clone(), secret)?;
    Ok(reference)
}

#[tauri::command]
pub fn gateway_clear_api_key(profile_id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut profile = {
        let registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
        registry
            .profile(&profile_id)
            .ok_or_else(|| "GATEWAY_NOT_FOUND".to_string())?
    };
    state
        .secrets
        .lock()
        .map_err(|_| "secret lock poisoned")?
        .remove(&profile.api_key_ref)?;
    profile.enabled = false;
    state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?
        .upsert_gateway_profile(&profile)
        .map_err(|error| error.to_string())?;
    let mut registry = state.gateways.lock().map_err(|_| "gateway lock poisoned")?;
    registry.update(profile);
    Ok(())
}
