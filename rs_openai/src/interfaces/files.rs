use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    /// Name of the [JSON Lines](https://jsonlines.readthedocs.io/en/latest/) file to be uploaded.
    pub file: File,

    /// The intended purpose of the uploaded documents.
    pub purpose: crate::shared::types::FilePurpose,

    /// When this file should expire. If not set, the file has no expiration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
    pub expires_at: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileListResponse {
    pub data: Vec<FileResponse>,
    pub object: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
