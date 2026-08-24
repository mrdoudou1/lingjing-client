use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, Manager};
use tokio::sync::watch;

mod assets;
mod commands;
mod domain;
mod gateways;
mod jobs;
mod media;
mod persistence;

pub struct AppState {
    pub jobs: Mutex<jobs::JobManager>,
    pub gateways: Mutex<gateways::GatewayRegistry>,
    pub assets: Mutex<assets::AssetStore>,
    pub media: Mutex<media::MediaStore>,
    pub settings: Mutex<persistence::SettingsStore>,
    pub database: Mutex<persistence::SqliteStore>,
    pub secrets: Mutex<persistence::SecretStore>,
    pub chat_stops: Mutex<HashMap<String, watch::Sender<bool>>>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let database = persistence::SqliteStore::open_default().expect("sqlite init failed");
    let mut profiles = database
        .list_gateway_profiles()
        .expect("gateway profile load failed");
    let persisted_assets: Vec<domain::Asset> = database
        .list_snapshots("assets")
        .expect("asset load failed");
    if profiles.is_empty() {
        let default_profile = gateways::GatewayRegistry::mock_profile();
        database
            .upsert_gateway_profile(&default_profile)
            .expect("default gateway profile save failed");
        profiles.push(default_profile);
    }
    tauri::Builder::default()
        .manage(AppState {
            jobs: Mutex::new(jobs::JobManager::default()),
            gateways: Mutex::new(gateways::GatewayRegistry::from_profiles(profiles)),
            assets: Mutex::new(assets::AssetStore::from_assets(persisted_assets)),
            media: Mutex::new(media::MediaStore::default()),
            settings: Mutex::new(persistence::SettingsStore::default()),
            database: Mutex::new(database),
            secrets: Mutex::new(persistence::SecretStore::default()),
            chat_stops: Mutex::new(HashMap::new()),
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
            commands::gateway_clear_api_key,
            commands::chat_send,
            commands::chat_list_sessions,
            commands::chat_save_session,
            commands::chat_delete_session,
            commands::chat_stream,
            commands::chat_stop,
            commands::video_create_job,
            commands::video_poll_job,
            commands::image_create_job,
            commands::audio_tts,
            commands::audio_stt,
            commands::job_get,
            commands::job_cancel,
            commands::job_retry,
            commands::job_update,
            commands::job_list,
            commands::asset_list,
            commands::asset_register,
            commands::asset_toggle_favorite,
            commands::asset_delete,
            commands::asset_open_location,
            commands::asset_export,
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
