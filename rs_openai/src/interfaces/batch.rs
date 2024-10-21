use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateBatchRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateBatchRequest {
    /// The ID of an uploaded file that contains training data.
    ///
    /// See [upload file](https://platform.openai.com/docs/api-reference/files/upload) for how to upload a file.
    ///
    ///
    /// Your input file must be formatted as a [JSONL file](https://platform.openai.com/docs/api-reference/batch/request-input),
    /// and must be uploaded with the purpose `batch`. The file can contain up to 50,000 requests, and can be up to 100 MB in size.
    pub input_file_id: String,

    /// The endpoint to be used for all requests in the batch. Currently `/v1/chat/completions`, `/v1/embeddings`, and `/v1/completions` are supported.
    /// Note that `/v1/embeddings` batches are also restricted to a maximum of 50,000 embedding inputs across all requests in the batch.
    pub endpoint: String,

    /// The time frame within which the batch should be processed. Currently only `24h` is supported.
    pub completion_window: String,

    /// Optional custom metadata for the batch.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "ListBatchRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ListBatchRequest {
    /// A cursor for use in pagination. `after` is an object ID that defines your place in the list.
    /// For instance, if you make a list request and receive 100 objects, ending with obj_foo,
    /// your subsequent call can include after=obj_foo in order to fetch the next page of the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
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
    id: String,
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
