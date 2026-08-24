use crate::domain::{
    AudioRequest, GatewayProfile, GenerationJob, ImageRequest, JobKind, JobStatus, VideoRequest,
};
use chrono::Utc;
use std::collections::HashMap;
use uuid::Uuid;

pub mod http;

#[derive(Default)]
pub struct GatewayRegistry {
    profiles: HashMap<String, GatewayProfile>,
}

impl GatewayRegistry {
    pub fn mock_profile() -> GatewayProfile {
        GatewayProfile {
            id: "mock-default".into(),
            name: "Mock Gateway".into(),
            base_url: "mock://local".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:mock-default".into(),
            enabled: true,
            is_default: true,
            created_at: Some("1970-01-01T00:00:00Z".into()),
            updated_at: Some("1970-01-01T00:00:00Z".into()),
        }
    }

    pub fn from_profiles(profiles: Vec<GatewayProfile>) -> Self {
        Self {
            profiles: profiles
                .into_iter()
                .map(|profile| (profile.id.clone(), profile))
                .collect(),
        }
    }

    pub fn profile(&self, id: &str) -> Option<GatewayProfile> {
        self.profiles
            .get(id)
            .cloned()
            .or_else(|| (id == "mock-default" && self.profiles.is_empty()).then(Self::mock_profile))
    }
    pub fn profiles(&self) -> Vec<GatewayProfile> {
        if self.profiles.is_empty() {
            return vec![Self::mock_profile()];
        }
        self.profiles.values().cloned().collect()
    }
    pub fn models(&self, _profile_id: &str) -> Vec<String> {
        vec![
            "gpt-4.1".into(),
            "grok-imagine-image-2.0".into(),
            "flux-pro".into(),
            "gpt-image-1".into(),
            "grok-imagine-video".into(),
            "veo-3".into(),
            "mock-audio".into(),
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
            remote_job_id: None,
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
            remote_job_id: None,
            created_at: Utc::now(),
        }
    }
    pub fn create_audio_job(&self, request: &AudioRequest) -> GenerationJob {
        GenerationJob {
            id: Uuid::new_v4(),
            gateway_profile_id: request.gateway_profile_id.clone(),
            kind: if request.kind == "stt" {
                JobKind::Stt
            } else {
                JobKind::Tts
            },
            operation: None,
            model_id: Some(request.model_id.clone()),
            status: JobStatus::Queued,
            progress: 0.0,
            request_json: serde_json::to_value(request).unwrap_or_default(),
            error_message: None,
            remote_job_id: None,
            created_at: Utc::now(),
        }
    }

    pub fn create(&mut self, profile: GatewayProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }
    pub fn update(&mut self, profile: GatewayProfile) {
        self.profiles.insert(profile.id.clone(), profile);
    }
    pub fn delete(&mut self, id: &str) {
        self.profiles.remove(id);
    }
    pub fn set_default(&mut self, id: &str) {
        for profile in self.profiles.values_mut() {
            profile.is_default = profile.id == id;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_registry_exposes_mock_profile_and_models() {
        let registry = GatewayRegistry::default();
        assert_eq!(registry.profiles()[0].id, "mock-default");
        assert!(registry
            .models("mock-default")
            .contains(&"grok-imagine-video".to_string()));
    }

    #[test]
    fn default_can_be_changed() {
        let mut registry = GatewayRegistry::default();
        registry.create(GatewayProfile {
            id: "second".into(),
            name: "Second".into(),
            base_url: "mock://second".into(),
            protocol: "openai-compatible".into(),
            api_key_ref: "system-keychain:second".into(),
            enabled: true,
            is_default: false,
            created_at: None,
            updated_at: None,
        });
        registry.set_default("second");
        assert!(registry
            .profiles()
            .iter()
            .any(|profile| profile.id == "second" && profile.is_default));
    }
}
