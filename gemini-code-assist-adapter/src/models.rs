use serde::{Deserialize, Serialize};
use gemini_rust::GenerationResponse;

/// Request wrapper for Code Assist API.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodeAssistEnvelope {
    pub model: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_prompt_id: Option<String>,
    pub request: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct CodeAssistResponseEnvelope {
    pub response: GenerationResponse,
    pub trace_id: Option<String>,
}

// Structures for handshake (kept as they were in the previous response)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientMetadata {
    pub ide_type: String,
    pub ide_version: String,
    pub plugin_version: String,
    pub platform: String,
    pub plugin_type: String,
}

impl Default for ClientMetadata {
    fn default() -> Self {
        Self {
            ide_type: "GEMINI_CLI".to_string(),
            ide_version: "0.21.0".to_string(),
            plugin_version: "0.21.0".to_string(),
            platform: "LINUX_AMD64".to_string(),
            plugin_type: "GEMINI".to_string(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloudaicompanion_project: Option<String>,
    pub metadata: ClientMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadCodeAssistResponse {
    pub cloudaicompanion_project: Option<String>,
    pub current_tier: Option<Tier>,
}

#[derive(Debug, Deserialize)]
pub struct Tier {
    pub id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardUserRequest {
    pub tier_id: String, // "free-tier"
    pub cloudaicompanion_project: Option<String>,
    pub metadata: ClientMetadata,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LroResponse {
    pub name: String,
    pub done: Option<bool>,
    pub response: Option<OnboardUserResponse>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardUserResponse {
    pub cloudaicompanion_project: Option<ProjectInfo>,
}

#[derive(Debug, Deserialize)]
pub struct ProjectInfo {
    pub id: String,
}
