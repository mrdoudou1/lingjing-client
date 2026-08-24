use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn settings_get(
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<String, serde_json::Value>, String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    database.get_settings().map_err(|error| error.to_string())
}

#[tauri::command]
pub fn settings_update(
    update: crate::persistence::SettingsUpdate,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let database = state
        .database
        .lock()
        .map_err(|_| "database lock poisoned")?;
    for (key, value) in &update.values {
        let raw = serde_json::to_string(value).map_err(|error| error.to_string())?;
        database
            .set_setting(key, &raw)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}
