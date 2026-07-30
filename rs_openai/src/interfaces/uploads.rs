use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// The File object (not file name) to be uploaded.
    pub file: File,

    /// The intended purpose of the uploaded file: "assistants", "vision", "batch", or "fine-tune".
    #[builder(into)]
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UploadFileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    pub status: String,
    pub expires_at: i64,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct AddUploadPartRequest {
    /// The chunk of bytes for this Part.
    pub data: File,
}

/// The upload [Part](https://platform.openai.com/docs/api-reference/uploads/part-object) object.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AddUploadPartResponse {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub upload_id: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CompleteUploadRequest {
    /// The ordered list of Part IDs.
    pub part_ids: Vec<String>,

    /// The optional md5 checksum for the file contents to verify if the bytes uploaded matches what you expect.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub md5: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadFile {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CompleteUploadResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    pub status: String,
    pub expires_at: i64,
    pub file: CompleteUploadFile,
}
