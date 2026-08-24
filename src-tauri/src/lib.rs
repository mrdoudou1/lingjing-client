use std::sync::Mutex;

mod assets;
mod commands;
mod domain;
mod gateways;
mod jobs;
mod persistence;

pub struct AppState {
    pub jobs: Mutex<jobs::JobManager>,
    pub gateways: Mutex<gateways::GatewayRegistry>,
    pub assets: Mutex<assets::AssetStore>,
    pub settings: Mutex<persistence::SettingsStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            jobs: Mutex::new(jobs::JobManager::default()),
            gateways: Mutex::new(gateways::GatewayRegistry::default()),
            assets: Mutex::new(assets::AssetStore::default()),
            settings: Mutex::new(persistence::SettingsStore::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::gateway_list_profiles,
            commands::gateway_test_connection,
            commands::gateway_refresh_models,
            commands::chat_send,
            commands::video_create_job,
            commands::image_create_job,
            commands::job_get,
            commands::job_cancel,
            commands::job_retry,
            commands::asset_list,
            commands::settings_get,
            commands::settings_update,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lingjing");
}
