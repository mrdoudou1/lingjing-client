use crate::domain::GatewayProfile;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolAdapter {
    OpenAiCompatible,
    NewApi,
    Sub2Api,
    Grok2Api,
    Custom,
}

impl ProtocolAdapter {
    pub fn from_profile(profile: &GatewayProfile) -> Self {
        match profile.protocol.to_ascii_lowercase().as_str() {
            "newapi" => Self::NewApi,
            "sub2api" => Self::Sub2Api,
            "grok2api" => Self::Grok2Api,
            "custom" => Self::Custom,
            _ => Self::OpenAiCompatible,
        }
    }

    pub fn endpoint(self, name: &str) -> String {
        // All built-in protocols currently expose the OpenAI-compatible paths.
        // Keeping path selection here isolates vendor differences to this adapter layer.
        match self {
            Self::OpenAiCompatible
            | Self::NewApi
            | Self::Sub2Api
            | Self::Grok2Api
            | Self::Custom => name.to_string(),
        }
    }
}

pub fn adapter_for(profile: &GatewayProfile) -> ProtocolAdapter {
    ProtocolAdapter::from_profile(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(protocol: &str) -> GatewayProfile {
        GatewayProfile {
            id: protocol.into(),
            name: protocol.into(),
            base_url: "https://example.test".into(),
            protocol: protocol.into(),
            api_key_ref: format!("key:{protocol}"),
            enabled: true,
            is_default: false,
            created_at: None,
            updated_at: None,
        }
    }

    #[test]
    fn registry_selects_each_supported_protocol() {
        assert_eq!(
            ProtocolAdapter::from_profile(&profile("newapi")),
            ProtocolAdapter::NewApi
        );
        assert_eq!(
            ProtocolAdapter::from_profile(&profile("sub2api")),
            ProtocolAdapter::Sub2Api
        );
        assert_eq!(
            ProtocolAdapter::from_profile(&profile("grok2api")),
            ProtocolAdapter::Grok2Api
        );
        assert_eq!(
            ProtocolAdapter::from_profile(&profile("custom")),
            ProtocolAdapter::Custom
        );
        assert_eq!(
            ProtocolAdapter::from_profile(&profile("unknown")),
            ProtocolAdapter::OpenAiCompatible
        );
    }
}
