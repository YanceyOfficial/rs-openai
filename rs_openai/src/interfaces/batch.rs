use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum BatchEndpoint {
    #[strum(serialize = "/v1/responses")]
    #[serde(rename = "/v1/responses")]
    Responses,
    #[strum(serialize = "/v1/chat/completions")]
    #[serde(rename = "/v1/chat/completions")]
    ChatCompletions,
    #[strum(serialize = "/v1/embeddings")]
    #[serde(rename = "/v1/embeddings")]
    Embeddings,
    #[strum(serialize = "/v1/completions")]
    #[serde(rename = "/v1/completions")]
    Completions,
    #[strum(serialize = "/v1/moderations")]
    #[serde(rename = "/v1/moderations")]
    Moderations,
    #[strum(serialize = "/v1/images/generations")]
    #[serde(rename = "/v1/images/generations")]
    ImagesGenerations,
    #[strum(serialize = "/v1/images/edits")]
    #[serde(rename = "/v1/images/edits")]
    ImagesEdits,
    #[strum(serialize = "/v1/videos")]
    #[serde(rename = "/v1/videos")]
    Videos,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateBatchRequest {
    /// The ID of an uploaded file that contains training data. Must be uploaded with purpose `batch`.
    #[builder(into)]
    pub input_file_id: String,

    /// The endpoint to be used for all requests in the batch.
    pub endpoint: BatchEndpoint,

    /// The time frame within which the batch should be processed. Currently only `24h` is supported.
    #[builder(into)]
    pub completion_window: String,

    /// Optional custom metadata for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    /// When the batch's output/error files should expire. If not set, they have no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_expires_after: Option<crate::shared::types::ExpiresAfter>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListBatchRequest {
    /// A cursor for use in pagination.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RequestCounts {
    total: u32,
    completed: u32,
    failed: u32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct BatchResponse {
    pub id: String,
    object: String,
    endpoint: String,
    errors: Option<String>,
    input_file_id: String,
    completion_window: String,
    status: String,
    output_file_id: Option<String>,
    error_file_id: Option<String>,
    created_at: i64,
    in_progress_at: Option<i64>,
    expires_at: Option<i64>,
    finalizing_at: Option<i64>,
    completed_at: Option<i64>,
    failed_at: Option<i64>,
    expired_at: Option<i64>,
    cancelling_at: Option<i64>,
    cancelled_at: Option<i64>,
    request_counts: RequestCounts,
    metadata: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListBatchResponse {
    object: String,
    data: Vec<BatchResponse>,
}
