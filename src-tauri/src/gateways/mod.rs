use crate::domain::{
    GatewayProfile, GenerationJob, ImageRequest, JobKind, JobStatus, VideoRequest,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Default)]
pub struct GatewayRegistry {
    profiles: HashMap<String, GatewayProfile>,
}

impl GatewayRegistry {
    pub fn profiles(&self) -> Vec<GatewayProfile> {
        if self.profiles.is_empty() {
            return vec![GatewayProfile {
                id: "mock-default".into(),
                name: "Mock Gateway".into(),
                base_url: "mock://local".into(),
                protocol: "openai-compatible".into(),
                api_key_ref: "system-keychain:mock-default".into(),
                enabled: true,
                is_default: true,
            }];
        }
        self.profiles.values().cloned().collect()
    }
    pub fn models(&self, _profile_id: &str) -> Vec<String> {
        vec![
            "gpt-4.1".into(),
            "grok-imagine-video".into(),
            "veo-3".into(),
        ]
    }
    pub fn test(&self, _profile_id: &str) -> serde_json::Value {
        serde_json::json!({"ok": true, "latencyMs": 42})
    }
    pub fn create_video_job(&self, request: &VideoRequest) -> GenerationJob {
        GenerationJob {
            id: Uuid::new_v4(),
            gateway_profile_id: request.gateway_profile_id.clone(),
            kind: JobKind::Video,
            operation: Some(request.operation.clone()),
            model_id: Some(request.model_id.clone()),
            status: JobStatus::Queued,
            progress: 0.0,
            request_json: serde_json::to_value(request).unwrap_or_default(),
            error_message: None,
            created_at: Utc::now(),
        }
    }
    pub fn create_image_job(&self, request: &ImageRequest) -> GenerationJob {
        GenerationJob {
            id: Uuid::new_v4(),
            gateway_profile_id: request.gateway_profile_id.clone(),
            kind: JobKind::Image,
            operation: None,
            model_id: Some(request.model_id.clone()),
            status: JobStatus::Queued,
            progress: 0.0,
            request_json: serde_json::to_value(request).unwrap_or_default(),
            error_message: None,
            created_at: Utc::now(),
        }
    }
}
