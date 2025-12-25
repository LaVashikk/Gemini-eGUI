use std::path::PathBuf;
use yup_oauth2::{InstalledFlowAuthenticator, InstalledFlowReturnMethod, AccessToken};
use yup_oauth2::authenticator_delegate::InstalledFlowDelegate;
use serde::Deserialize;
use reqwest::Client;
use crate::error::AdapterError;
use std::future::Future;
use std::pin::Pin;

// These keys are taken from the gemini-cli source code (packages/core/src/mcp/oauth-provider.ts)
// These are standard keys for "Gemini Code Assist" (Desktop app)
const OAUTH_CLIENT_ID: &str = "681255809395-oo8ft2oprdrnp9e3aqf6av3hmdib135j.apps.googleusercontent.com";
const OAUTH_CLIENT_SECRET: &str = "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";

// Scopes required for Code Assist to work
const SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/cloud-platform",
    "https://www.googleapis.com/auth/userinfo.email",
];

#[derive(Clone, Debug)]
pub struct AuthSession {
    pub access_token: String,
    pub project_id: String,
}

pub struct GoogleAuthManager {
    _cache_path: PathBuf,
}

struct BrowserFlowDelegate;

impl InstalledFlowDelegate for BrowserFlowDelegate {
    fn present_user_url<'a>(
        &'a self,
        url: &'a str,
        _need_code: bool,
    ) -> Pin<Box<dyn Future<Output = Result<String, String>> + Send + 'a>> {
        Box::pin(async move {
            log::info!("Opening browser to:\n{}", url); 
            if let Err(e) = open::that(url) {
                log::error!("Failed to open browser: {}", e);
                log::warn!("Please open the URL manually.");
            }
            Ok(String::new())
        })
    }
}

impl GoogleAuthManager {
    pub fn new() -> Self {
        // Determine the path for storing the token:
        let proj_dirs = directories::ProjectDirs::from("com", "gemini-gui", "app")
            .expect("Could not determine config directory");

        let cache_dir = proj_dirs.config_dir();
        std::fs::create_dir_all(cache_dir).ok();

        Self {
            _cache_path: cache_dir.join("token_cache.json"),
        }
    }

    /// Starts the login process.
    /// 1. Opens the browser.
    /// 2. User logs in to Google.
    /// 3. Returns the Access Token.
    pub async fn login(&self) -> Result<String, AdapterError> {
        let secret = yup_oauth2::ApplicationSecret {
            client_id: OAUTH_CLIENT_ID.to_string(),
            client_secret: OAUTH_CLIENT_SECRET.to_string(),
            token_uri: "https://oauth2.googleapis.com/token".to_string(),
            auth_uri: "https://accounts.google.com/o/oauth2/auth".to_string(),
            redirect_uris: vec!["http://localhost".to_string()], 
            ..Default::default()
        };

        // Create the authenticator
        let auth = InstalledFlowAuthenticator::builder(
            secret,
            InstalledFlowReturnMethod::HTTPRedirect, // Will start a local server
        )
        .persist_tokens_to_disk(&self._cache_path)
        .flow_delegate(Box::new(BrowserFlowDelegate))
        .build()
        .await
        .map_err(|e| AdapterError::StreamError(format!("Auth builder failed: {}", e)))?;

        // Obtain the token
        let token: AccessToken = auth
            .token(SCOPES)
            .await
            .map_err(|e| AdapterError::StreamError(format!("Failed to get token: {}", e)))?;

        Ok(token.token().map(|s| s.to_string()).unwrap_or_default())
    }

    /// Clears the token cache file from disk.
    pub fn clear_token_cache(&self) {
        if self._cache_path.exists() {
            let _ = std::fs::remove_file(&self._cache_path);
            log::info!("Token cache cleared: {:?}", self._cache_path);
        }
    }

    ///  finds the list of Google Cloud projects available to the user.
    /// This is Automaticallyneeded so the user can select a project_id.
    pub async fn list_projects(&self, access_token: &str) -> Result<Vec<String>, AdapterError> {
        let client = Client::new();
        let url = "https://cloudresourcemanager.googleapis.com/v1/projects";

        let response = client
            .get(url)
            .bearer_auth(access_token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AdapterError::ApiError {
                code: response.status().as_u16(),
                message: "Failed to list projects".to_string()
            });
        }

        #[derive(Deserialize)]
        struct Project {
            #[serde(rename = "projectId")]
            project_id: String,
            #[serde(rename = "lifecycleState")]
            state: String,
        }

        #[derive(Deserialize)]
        struct ProjectsResponse {
            projects: Option<Vec<Project>>,
        }

        let resp: ProjectsResponse = response.json().await?;

        let active_projects = resp.projects
            .unwrap_or_default()
            .into_iter()
            .filter(|p| p.state == "ACTIVE")
            .map(|p| p.project_id)
            .collect();

        Ok(active_projects)
    }
}
