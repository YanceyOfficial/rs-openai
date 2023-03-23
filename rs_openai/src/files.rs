//! Files are used to upload documents that can be used with features like [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tunes).

use super::{OpenAI, OpenAIResponse};
use crate::error::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "UploadFileRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct UploadFileRequest {
    /// Name of the [JSON Lines](https://jsonlines.readthedocs.io/en/latest/) file to be uploaded.
    ///
    /// If the `purpose` is set to "fine-tune", each line is a JSON record with "prompt" and "completion" fields representing your [training examples](https://platform.openai.com/docs/guides/fine-tuning/prepare-training-data).
    pub file: String,

    /// The intended purpose of the uploaded documents.
    ///
    /// Use "fine-tune" for [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tunes).
    /// This allows us to validate the format of the uploaded file.
    pub purpose: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u64,
    pub filename: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileListResponse {
    pub data: Vec<FileResponse>,
    pub object: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DeleteFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

pub struct Files<'a> {
    openai: &'a OpenAI<'a>,
}

impl<'a> Files<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
    /// Returns a list of files that belong to the user's organization.
    #[tokio::main]
    pub async fn list(&self) -> OpenAIResponse<FileListResponse> {
        self.openai.get("/files", &()).await
    }

    /// Delete a file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    #[tokio::main]
    pub async fn delete(&self, file_id: &str) -> OpenAIResponse<DeleteFileResponse> {
        self.openai.delete(&format!("/files/{file_id}"), &()).await
    }

    /// Upload a file that contains document(s) to be used across various endpoints/features.
    /// Currently, the size of all the files uploaded by one organization can be up to 1 GB.
    /// Please contact us if you need to increase the storage limit.
    #[tokio::main]
    pub async fn upload(&self, req: &UploadFileRequest) -> OpenAIResponse<FileResponse> {
        self.openai.post("/files", req).await
    }

    /// Returns information about a specific file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    #[tokio::main]
    pub async fn retrieve(&self, file_id: &str) -> OpenAIResponse<FileResponse> {
        self.openai.get(&format!("/files/{file_id}"), &()).await
    }

    /// Returns the contents of the specified file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    #[tokio::main]
    pub async fn retrieve_content(&self, file_id: &str) -> OpenAIResponse<String> {
        self.openai
            .get(&format!("/files/{file_id}/content"), &())
            .await
    }
}
