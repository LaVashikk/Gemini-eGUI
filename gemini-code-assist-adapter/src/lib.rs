pub mod error;
pub mod models;
pub mod auth;

use std::pin::Pin;
use futures::{Stream, StreamExt};
use reqwest::Client;
use eventsource_stream::Eventsource;
use gemini_rust::{GenerateContentRequest, GenerationResponse};
use crate::error::AdapterError;
use crate::models::{ClientMetadata, CodeAssistEnvelope, CodeAssistResponseEnvelope, LoadCodeAssistRequest, LoadCodeAssistResponse, LroResponse, OnboardUserRequest};

// const BASE_URL: &str = "https://cloudaicompanion.googleapis.com/v1internal";
const BASE_URL: &str = "https://cloudcode-pa.googleapis.com/v1internal";


/// Adapter client for working with Gemini Code Assist.
#[derive(Clone)]
pub struct CodeAssistClient {
    http_client: Client,
    project_id: String,
    auth_token: String,
    model: String,
}

fn sanitize_model_name(model: &str) -> String {
    // Remove "models/" prefix if present
    model.strip_prefix("models/").unwrap_or(model).to_string()
}


impl CodeAssistClient {
    /// Creates a new client.
    ///
    /// # Arguments
    /// * `auth_token` - OAuth2 Access Token (obtained via gcloud auth print-access-token or OAuth flow)
    /// * `project_id` - Google Cloud Project ID (e.g., "my-genai-project")
    pub fn new(auth_token: String, project_id: String) -> Self {
        Self {
            http_client: Client::new(),
            project_id,
            auth_token,
            model: "models/gemini-3-flash-preview".to_string(),
        }
    }

    /// Activates the user/project in the Code Assist system.
    pub async fn onboard_user(&mut self) -> Result<(), AdapterError> {
        let url = format!("{}:onboardUser", BASE_URL);

        let request = OnboardUserRequest {
            tier_id: "free-tier".to_string(), // todo!
            cloudaicompanion_project: Some(self.project_id.clone()),
            metadata: ClientMetadata::default(),
        };

        log::debug!("Onboarding user for project: {}", self.project_id);

        let mut lro: LroResponse = self.http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&request)
            .send()
            .await?
            .json()
            .await?;

        let mut attempts = 0;
        while lro.done != Some(true) && attempts < 5 {
            log::debug!("Onboarding in progress... waiting");
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;

            // Repeat request (it is idempotent or returns status)
            lro = self.http_client
                .post(&url)
                .bearer_auth(&self.auth_token)
                .json(&request)
                .send()
                .await?
                .json()
                .await?;

            attempts += 1;
        }

        if let Some(resp) = lro.response {
            if let Some(proj) = resp.cloudaicompanion_project {
                log::debug!("Onboarding complete. Project confirmed: {}", proj.id);
                self.project_id = proj.id; // Update ID if server issued a different one
                return Ok(());
            }
        }

        // If we are here, it means either done=true but no response, or timeout
        log::debug!("Onboarding finished (assumed success or already done).");
        Ok(())
    }

    pub async fn load_code_assist(&self) -> Result<String, AdapterError> {
        let url = format!("{}:loadCodeAssist", BASE_URL);

        // We try to send the project we found during login
        let request = LoadCodeAssistRequest {
            cloudaicompanion_project: Some(self.project_id.clone()),
            metadata: ClientMetadata::default(),
        };

        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let code = response.status().as_u16();
             let text = response.text().await.unwrap_or_default();
             return Err(AdapterError::ApiError {
                code,
                message: format!("Handshake failed: {}", text),
            });
        }

        let data: LoadCodeAssistResponse = response.json().await?;

        // Logic from setup.ts gemini-cli:
        let effective_project = data.cloudaicompanion_project.unwrap_or(self.project_id.clone());

        log::debug!("Handshake success. Tier: {:?}. Using Project: {}",
            data.current_tier.map(|t| t.id),
            effective_project
        );

        Ok(effective_project)
    }

    // Helper method to update project_id inside client after handshake
    pub fn set_project_id(&mut self, project_id: String) {
        self.project_id = project_id;
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Performs a standard (non-streaming) request.
    /// Accepts standard `GenerateContentRequest` from gemini-rust.
    pub async fn generate_content(
        &self,
        request: &GenerateContentRequest,
    ) -> Result<GenerationResponse, AdapterError> {
        let url = format!("{}:generateContent", BASE_URL);

        let mut request_json = serde_json::to_value(request)?;
        let session_id = uuid::Uuid::new_v4().to_string();
        if let Some(obj) = request_json.as_object_mut() {
            obj.insert("session_id".to_string(), serde_json::json!(session_id));
        }

        let envelope = CodeAssistEnvelope {
            model: sanitize_model_name(&self.model),
            project: self.project_id.clone(),
            user_prompt_id: Some(uuid::Uuid::new_v4().to_string()),
            request: request_json,
        };

        log::debug!("Sending Envelope: {}", serde_json::to_string_pretty(&envelope).unwrap());
        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&envelope)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AdapterError::ApiError {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let envelope_resp: CodeAssistResponseEnvelope = response.json().await?;
        Ok(envelope_resp.response)
    }

    /// Performs a streaming request.
    /// Returns a stream of `GenerationResponse` (chunks).
    pub async fn generate_content_stream(
        &self,
        request: &GenerateContentRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<GenerationResponse, AdapterError>> + Send>>, AdapterError> {
        let url = format!("{}:streamGenerateContent?alt=sse", BASE_URL);
        let mut request_json = serde_json::to_value(request)?;

        let session_id = uuid::Uuid::new_v4().to_string();

        if let Some(obj) = request_json.as_object_mut() {
            obj.insert("session_id".to_string(), serde_json::json!(session_id));
        }

        let envelope = CodeAssistEnvelope {
            model: sanitize_model_name(&self.model),
            project: self.project_id.clone(),
            user_prompt_id: Some(uuid::Uuid::new_v4().to_string()),
            request: request_json,
        };
        log::debug!("Sending Envelope: {}", serde_json::to_string_pretty(&envelope).unwrap());


        let response = self.http_client
            .post(&url)
            .bearer_auth(&self.auth_token)
            .json(&envelope)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AdapterError::ApiError {
                code: response.status().as_u16(),
                message: response.text().await.unwrap_or_default(),
            });
        }

        let stream = response.bytes_stream().eventsource();

        let mapped_stream = stream.map(|event_result| {
            match event_result {
                Ok(event) => {
                    if event.data == "[DONE]" {
                        return None;
                    }

                    match serde_json::from_str::<CodeAssistResponseEnvelope>(&event.data) {
                        Ok(envelope) => Some(Ok(envelope.response)),
                        Err(e) => Some(Err(AdapterError::SerdeError(e))),
                    }
                }
                Err(e) => Some(Err(AdapterError::StreamError(e.to_string()))),
            }
        })
        .filter_map(|x| async { x }); // Remove None

        Ok(Box::pin(mapped_stream))
    }
}
