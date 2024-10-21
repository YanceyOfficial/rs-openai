use crate::shared::{response_wrapper::OpenAIError, types::FileMeta};
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "UploadFileRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct UploadFileRequest {
    /// The File object (not file name) to be uploaded.
    pub file: FileMeta,

    /// The intended purpose of the uploaded file.

    /// Use "assistants" for [Assistants](https://platform.openai.com/docs/api-reference/assistants) and [Message](https://platform.openai.com/docs/api-reference/messages) files,
    /// "vision" for Assistants image file inputs, "batch" for [Batch API](https://platform.openai.com/docs/guides/batch), and "fine-tune" for [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tuning).
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UploadFileResponse {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
    status: String,
    expires_at: i64,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "AddUploadPartRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct AddUploadPartRequest {
    /// The chunk of bytes for this Part.
    pub data: FileMeta,
}

/// The upload [Part](https://platform.openai.com/docs/api-reference/uploads/part-object) object.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AddUploadPartResponse {
    id: String,
    object: String,
    created_at: i64,
    upload_id: String,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CompleteUploadBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CompleteUploadRequest {
    /// The ordered list of Part IDs.
    pub part_ids: Vec<String>,

    /// The optional md5 checksum for the file contents to verify if the bytes uploaded matches what you expect.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadFile {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadResponse {
    id: String,
    object: String,
    bytes: u64,
    created_at: i64,
    filename: String,
    purpose: String,
    status: String,
    expires_at: i64,
    file: CompleteUploadFile,
}
