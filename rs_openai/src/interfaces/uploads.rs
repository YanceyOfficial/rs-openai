use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// The number of bytes in the file you are uploading.
    pub bytes: u64,

    /// The name of the file to upload.
    #[builder(into)]
    pub filename: String,

    /// The MIME type of the file.
    ///
    /// This must fall within the supported MIME types for your file purpose. See the
    /// supported MIME types for assistants and vision.
    #[builder(into)]
    pub mime_type: String,

    /// The intended purpose of the uploaded file. Only `Assistants`, `Batch`, `FineTune`, and
    /// `Vision` are valid here — a subset of the full set `FilePurpose` supports for Files.
    pub purpose: crate::shared::types::FilePurpose,

    /// When this Upload should expire. If not set, the Upload has no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum UploadStatus {
    Pending,
    Completed,
    Cancelled,
    Expired,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UploadFileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: i64,
    pub filename: String,
    pub purpose: String,
    pub status: UploadStatus,
    pub expires_at: i64,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct AddUploadPartRequest {
    /// The chunk of bytes for this Part.
    pub data: crate::shared::types::File,
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
    pub status: UploadStatus,
    pub expires_at: i64,
    /// `None` until the upload actually completes — this response object is also returned by
    /// the create/cancel endpoints, before a file exists.
    pub file: Option<CompleteUploadFile>,
}
