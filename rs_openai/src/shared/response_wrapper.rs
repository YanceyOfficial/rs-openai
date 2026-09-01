//! Errors originating from API calls, parsing responses, and reading-or-writing to the file system.
use serde::Deserialize;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenAIError {
    /// Underlying error from reqwest library after an API call was made
    #[error("http error: {0}")]
    Reqwest(#[from] reqwest::Error),
    /// Error from the retry/middleware layer wrapping the HTTP client
    #[error("http middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),
    /// OpenAI returns error object with details of API call failure
    #[error("{:?}", .0.error)]
    ApiError(ApiErrorResponse),
    /// Error when a response cannot be deserialized into a Rust type
    #[error("failed to deserialize api response: {0}")]
    JSONDeserialize(serde_json::Error),
    /// Error when trying to stream completions SSE
    #[error("stream failed: {0}")]
    StreamError(String),
    /// Error from client side validation
    /// or when builder fails to build request before making API call
    #[error("invalid args: {0}")]
    InvalidArgument(String),
    /// Error constructing an HTTP header value (e.g. an org/project ID with invalid bytes)
    #[error("invalid header value: {0}")]
    InvalidHeaderValue(String),
    /// Error reading or writing a file (e.g. saving a text-to-speech response)
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    /// Error establishing or operating a Realtime API WebSocket connection. Stringified
    /// rather than `#[from] tokio_tungstenite::tungstenite::Error` to match this crate's
    /// existing convention for `StreamError` (avoids coupling the public error type to a
    /// specific transport crate's error representation).
    #[error("websocket error: {0}")]
    WebSocket(String),
}

#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ApiErrorDetail {
    pub message: String,
    pub r#type: String,
    pub param: Option<serde_json::Value>,
    pub code: Option<serde_json::Value>,
}

/// OpenAI API returns error object on failure
#[derive(Debug, Deserialize)]
#[non_exhaustive]
pub struct ApiErrorResponse {
    pub error: ApiErrorDetail,
}

pub type OpenAIResponse<T> = Result<T, OpenAIError>;
