use super::SqliteStore;
use crate::domain::{ChatSession, GatewayProfile};
use serde::{Deserialize, Serialize};

#[test]
fn migration_creates_settings_store() {
    let store = SqliteStore::in_memory().expect("sqlite should initialize");
    store
        .set_setting("theme", "\"dark\"")
        .expect("setting should save");
    assert_eq!(store.get_setting("theme").unwrap(), Some("\"dark\"".into()));
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct Snapshot {
    id: String,
    status: String,
}

#[test]
fn snapshots_survive_repository_round_trip() {
    let store = SqliteStore::in_memory().expect("sqlite should initialize");
    let snapshot = Snapshot {
        id: "job-1".into(),
        status: "queued".into(),
    };
    store
        .save_snapshot("jobs", "job-1", &snapshot)
        .expect("snapshot should save");
    let restored: Snapshot = store
        .get_snapshot("jobs", "job-1")
        .unwrap()
        .expect("snapshot should restore");
    assert_eq!(restored, snapshot);
}

#[test]
fn gateway_profiles_round_trip_and_keep_one_default() {
    let store = SqliteStore::in_memory().expect("sqlite should initialize");
    let first = GatewayProfile {
        id: "first".into(),
        name: "First".into(),
        base_url: "mock://first".into(),
        protocol: "openai-compatible".into(),
        api_key_ref: "system-keychain:first".into(),
        enabled: true,
        is_default: true,
        created_at: None,
        updated_at: None,
    };
    let second = GatewayProfile {
        id: "second".into(),
        name: "Second".into(),
        base_url: "mock://second".into(),
        protocol: "openai-compatible".into(),
        api_key_ref: "system-keychain:second".into(),
        enabled: true,
        is_default: true,
        created_at: None,
        updated_at: None,
    };
    store.upsert_gateway_profile(&first).unwrap();
    store.upsert_gateway_profile(&second).unwrap();
    let profiles = store.list_gateway_profiles().unwrap();
    assert_eq!(profiles.len(), 2);
    assert_eq!(
        profiles.iter().filter(|profile| profile.is_default).count(),
        1
    );
    assert_eq!(profiles[0].id, "second");
}

#[test]
fn chat_sessions_round_trip_with_messages() {
    let store = SqliteStore::in_memory().expect("sqlite should initialize");
    let session = ChatSession {
        id: "session-1".into(),
        title: "Test".into(),
        model_id: "gpt-4.1".into(),
        gateway_profile_id: "mock-default".into(),
        messages: vec![crate::domain::ChatMessage {
            id: "message-1".into(),
            role: "user".into(),
            content: "hello".into(),
            status: "completed".into(),
            created_at: "2026-08-24T00:00:00Z".into(),
        }],
        created_at: "2026-08-24T00:00:00Z".into(),
        updated_at: "2026-08-24T00:00:00Z".into(),
    };
    store.save_chat_session(&session).unwrap();
    let restored = store.list_chat_sessions().unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].messages[0].content, "hello");
    store.delete_chat_session("session-1").unwrap();
    assert!(store.list_chat_sessions().unwrap().is_empty());
}

#[test]
fn model_snapshots_refresh_by_gateway() {
    let store = SqliteStore::in_memory().expect("sqlite should initialize");
    store
        .save_model_snapshots(
            "gateway-1",
            &[
                (
                    "gpt-4.1".into(),
                    serde_json::json!({ "chat": { "streaming": true } }),
                ),
                (
                    "flux-pro".into(),
                    serde_json::json!({ "image": { "supportsEdit": true } }),
                ),
            ],
        )
        .unwrap();
    let snapshots = store.list_model_snapshots("gateway-1").unwrap();
    assert_eq!(snapshots.len(), 2);
    assert_eq!(snapshots[0].gateway_profile_id, "gateway-1");
    let flux = snapshots
        .iter()
        .find(|snapshot| snapshot.model_id == "flux-pro")
        .unwrap();
    assert_eq!(
        flux.capabilities_json,
        serde_json::json!({ "image": { "supportsEdit": true } })
    );
}
