use thiserror::Error;

#[derive(Debug, Error)]
pub enum AdapterError {
    #[error("HTTP Request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Serialization/Deserialization failed: {0}")]
    SerdeError(#[from] serde_json::Error),

    #[error("API returned error: {code} - {message}")]
    ApiError { code: u16, message: String },

    #[error("Stream error: {0}")]
    StreamError(String),
}
