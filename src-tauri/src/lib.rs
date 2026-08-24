use std::sync::Mutex;
use tauri::{Emitter, Manager};

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
    pub database: Mutex<persistence::SqliteStore>,
    pub secrets: Mutex<persistence::SecretStore>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState {
            jobs: Mutex::new(jobs::JobManager::default()),
            gateways: Mutex::new(gateways::GatewayRegistry::default()),
            assets: Mutex::new(assets::AssetStore::default()),
            settings: Mutex::new(persistence::SettingsStore::default()),
            database: Mutex::new(
                persistence::SqliteStore::open_default().expect("sqlite init failed"),
            ),
            secrets: Mutex::new(persistence::SecretStore::default()),
        })
        .invoke_handler(tauri::generate_handler![
            commands::gateway_list_profiles,
            commands::gateway_test_connection,
            commands::gateway_refresh_models,
            commands::gateway_create_profile,
            commands::gateway_update_profile,
            commands::gateway_delete_profile,
            commands::gateway_set_default,
            commands::gateway_set_api_key,
            commands::chat_send,
            commands::chat_stream,
            commands::video_create_job,
            commands::image_create_job,
            commands::audio_tts,
            commands::audio_stt,
            commands::job_get,
            commands::job_cancel,
            commands::job_retry,
            commands::job_list,
            commands::asset_list,
            commands::asset_delete,
            commands::asset_open_location,
            commands::storage_usage,
            commands::settings_get,
            commands::settings_update,
        ])
        .build(tauri::generate_context!())
        .expect("error while building Lingjing")
        .run(|app_handle, event| {
            if let tauri::RunEvent::ExitRequested { .. } = event {
                let state = app_handle.state::<AppState>();
                if let Ok(mut jobs) = state.jobs.lock() {
                    let stopped = jobs.stop_all();
                    if let Ok(database) = state.database.lock() {
                        for job in stopped {
                            let _ = database.save_snapshot("jobs", &job.id.to_string(), &job);
                        }
                    }
                }
                let _ = app_handle.emit(
                    "app://shutdown",
                    serde_json::json!({ "reason": "exit-requested" }),
                );
            }
        });
}
