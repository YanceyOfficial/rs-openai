//! Files are used to upload documents that can be used with features like [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tunes).

use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::FileMeta;
use crate::{OpenAI, OpenAIResponse};
use derive_builder::Builder;
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "UploadFileRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct UploadFileRequest {
    /// Name of the [JSON Lines](https://jsonlines.readthedocs.io/en/latest/) file to be uploaded.
    ///
    /// If the `purpose` is set to "fine-tune", each line is a JSON record with "prompt" and "completion" fields representing your [training examples](https://platform.openai.com/docs/guides/fine-tuning/prepare-training-data).
    pub file: FileMeta,

    /// The intended purpose of the uploaded documents.
    ///
    /// Use "fine-tune" for [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tunes).
    /// This allows us to validate the format of the uploaded file.
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileResponse {
    pub id: String,
    pub object: String,
    pub bytes: u64,
    pub created_at: u32,
    pub filename: String,
    pub purpose: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct FileListResponse {
    pub data: Vec<FileResponse>,
    pub object: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DeleteFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

pub struct Files<'a> {
    openai: &'a OpenAI,
}

impl<'a> Files<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
    /// Returns a list of files that belong to the user's organization.
    pub async fn list(&self) -> OpenAIResponse<FileListResponse> {
        self.openai.get("/files", &()).await
    }

    /// Upload a file that contains document(s) to be used across various endpoints/features.
    /// Currently, the size of all the files uploaded by one organization can be up to 1 GB.
    /// Please contact us if you need to increase the storage limit.
    pub async fn upload(&self, req: &UploadFileRequest) -> OpenAIResponse<FileResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let form = Form::new()
            .part("file", file_part)
            .text("purpose", req.purpose.to_string());

        self.openai.post_form("/files", form).await
    }

    /// Delete a file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    pub async fn delete(&self, file_id: &str) -> OpenAIResponse<DeleteFileResponse> {
        self.openai.delete(&format!("/files/{file_id}"), &()).await
    }

    /// Returns information about a specific file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    pub async fn retrieve(&self, file_id: &str) -> OpenAIResponse<FileResponse> {
        self.openai.get(&format!("/files/{file_id}"), &()).await
    }

    /// Returns the contents of the specified file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    pub async fn retrieve_content(&self, file_id: &str) -> OpenAIResponse<String> {
        self.openai
            .get(&format!("/files/{file_id}/content"), &())
            .await
    }
}
