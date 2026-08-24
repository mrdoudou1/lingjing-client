use crate::AppState;
use tauri::State;

#[tauri::command]
pub fn asset_list(state: State<'_, AppState>) -> Result<Vec<crate::domain::Asset>, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    Ok(assets.list())
}

#[tauri::command]
pub fn asset_register(
    asset: crate::domain::Asset,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .save_snapshot("assets", &asset.id, &asset)
            .map_err(|error| error.to_string())?;
    }
    let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    assets.upsert(asset);
    Ok(())
}

#[tauri::command]
pub fn asset_toggle_favorite(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<crate::domain::Asset>, String> {
    let asset = {
        let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
        assets.toggle_favorite(&id)
    };
    if let Some(ref asset) = asset {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .save_snapshot("assets", &asset.id, asset)
            .map_err(|error| error.to_string())?;
    }
    Ok(asset)
}

#[tauri::command]
pub fn asset_delete(id: String, state: State<'_, AppState>) -> Result<(), String> {
    {
        let database = state
            .database
            .lock()
            .map_err(|_| "database lock poisoned")?;
        database
            .delete_snapshot("assets", &id)
            .map_err(|error| error.to_string())?;
    }
    let mut assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    assets.delete(&id);
    Ok(())
}

#[tauri::command]
pub fn asset_open_location(
    id: String,
    state: State<'_, AppState>,
) -> Result<Option<String>, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    let path = assets
        .list()
        .into_iter()
        .find(|asset| asset.id == id)
        .map(|asset| asset.local_path);
    let Some(path) = path else { return Ok(None) };
    if path.starts_with("mock://") {
        return Err("ASSET_NOT_LOCAL: asset has no local file".into());
    }
    let file = std::path::Path::new(&path);
    if !file.exists() {
        return Err("ASSET_NOT_FOUND: local file does not exist".into());
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("explorer")
            .arg("/select,")
            .arg(file)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open")
            .arg("-R")
            .arg(file)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let parent = file.parent().unwrap_or(file);
        std::process::Command::new("xdg-open")
            .arg(parent)
            .status()
            .map_err(|error| format!("ASSET_OPEN_FAILED: {error}"))?;
    }
    Ok(Some(path))
}

#[tauri::command]
pub fn asset_export(id: String, state: State<'_, AppState>) -> Result<String, String> {
    let asset = {
        let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
        assets.list().into_iter().find(|asset| asset.id == id)
    }
    .ok_or_else(|| "ASSET_NOT_FOUND: asset metadata missing".to_string())?;
    if asset.local_path.starts_with("mock://") {
        return Err("ASSET_NOT_LOCAL: asset has no local file".into());
    }
    let source = std::path::Path::new(&asset.local_path);
    let media = state.media.lock().map_err(|_| "media lock poisoned")?;
    if !media.is_under_root(source) {
        return Err("ASSET_EXPORT_BLOCKED: path is outside the asset store".into());
    }
    if !source.exists() {
        return Err("ASSET_NOT_FOUND: local file does not exist".into());
    }
    let downloads = std::env::var("USERPROFILE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .join("Downloads");
    std::fs::create_dir_all(&downloads).map_err(|error| format!("STORAGE_FAILED: {error}"))?;
    let extension = source
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("bin");
    let destination = downloads.join(format!("{id}.{extension}"));
    std::fs::copy(source, &destination).map_err(|error| format!("DOWNLOAD_FAILED: {error}"))?;
    Ok(destination.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn storage_usage(state: State<'_, AppState>) -> Result<u64, String> {
    let assets = state.assets.lock().map_err(|_| "asset lock poisoned")?;
    Ok(assets.usage())
}
