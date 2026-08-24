use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobKind {
    Chat,
    Image,
    Video,
    Tts,
    Stt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Succeeded,
    Failed,
    Canceled,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GatewayProfile {
    pub id: String,
    pub name: String,
    pub base_url: String,
    pub protocol: String,
    pub api_key_ref: String,
    pub enabled: bool,
    pub is_default: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationJob {
    pub id: Uuid,
    pub gateway_profile_id: String,
    pub kind: JobKind,
    pub operation: Option<String>,
    pub model_id: Option<String>,
    pub status: JobStatus,
    pub progress: f32,
    pub request_json: serde_json::Value,
    pub error_message: Option<String>,
    #[serde(default)]
    pub remote_job_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRequest {
    pub gateway_profile_id: String,
    pub model_id: String,
    pub operation: String,
    pub prompt: String,
    pub source_video_asset_id: Option<String>,
    pub first_frame_asset_id: Option<String>,
    pub reference_image_asset_ids: Vec<String>,
    pub reference_voice_ids: Vec<String>,
    pub duration_sec: Option<u32>,
    #[serde(default)]
    pub extension_duration_sec: Option<u32>,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRequest {
    pub gateway_profile_id: String,
    pub model_id: String,
    pub prompt: String,
    pub count: u32,
    pub aspect_ratio: Option<String>,
    pub resolution: Option<String>,
    pub quality: Option<String>,
    pub reference_asset_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioRequest {
    pub gateway_profile_id: String,
    pub model_id: String,
    pub kind: String,
    pub text: Option<String>,
    pub source_file_name: Option<String>,
    #[serde(default)]
    pub source_file_base64: Option<String>,
    pub voice: Option<String>,
    pub language: Option<String>,
    pub format: String,
    pub speed: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSendInput {
    pub gateway_profile_id: String,
    pub model_id: String,
    pub session_id: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub id: String,
    pub role: String,
    pub content: String,
    pub status: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub id: String,
    pub title: String,
    pub model_id: String,
    pub gateway_profile_id: String,
    pub messages: Vec<ChatMessage>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Asset {
    pub id: String,
    pub job_id: Option<Uuid>,
    pub kind: String,
    pub mime_type: String,
    pub local_path: String,
    pub thumbnail_path: Option<String>,
    pub size_bytes: u64,
    #[serde(default)]
    pub favorite: bool,
    pub created_at: DateTime<Utc>,
}
