//! Files are used to upload documents that can be used with features like [Fine-tuning](https://platform.openai.com/docs/api-reference/fine-tunes).

use crate::client::OpenAI;
use crate::interfaces::files;
pub use crate::interfaces::files::*;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::Form;

pub struct Files<'a> {
    openai: &'a OpenAI,
}

impl<'a> Files<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
    /// Returns a list of files that belong to the user's organization.
    pub async fn list(&self) -> OpenAIResponse<files::FileListResponse> {
        self.openai.get("/files", &()).await
    }

    /// Upload a file that contains document(s) to be used across various endpoints/features.
    /// Currently, the size of all the files uploaded by one organization can be up to 1 GB.
    /// Please contact us if you need to increase the storage limit.
    pub async fn upload(
        &self,
        req: &files::UploadFileRequest,
    ) -> OpenAIResponse<files::FileResponse> {
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");

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
    pub async fn delete(&self, file_id: &str) -> OpenAIResponse<files::DeleteFileResponse> {
        self.openai.delete(&format!("/files/{file_id}"), &()).await
    }

    /// Returns information about a specific file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    pub async fn retrieve(&self, file_id: &str) -> OpenAIResponse<files::FileResponse> {
        self.openai.get(&format!("/files/{file_id}"), &()).await
    }

    /// Returns the contents of the specified file.
    ///
    /// # Path parameters
    ///
    /// - `file_id` - The ID of the file to use for this request
    ///
    /// TODO: Since free accounts cannot download fine-tune training files, I have to verify this api until purchase a Plus.
    pub async fn retrieve_content(&self, file_id: &str) -> OpenAIResponse<String> {
        self.openai
            .get(&format!("/files/{file_id}/content"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::files::UploadFileRequest;
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn upload_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/files"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "file-123",
                "object": "file",
                "bytes": 42,
                "created_at": 1700000000_u32,
                "filename": "data.jsonl",
                "purpose": "fine-tune"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .file(File {
                buffer: vec![1, 2, 3],
                filename: "data.jsonl".into(),
            })
            .purpose("fine-tune")
            .build();

        let res = client.files().upload(&req).await.unwrap();
        assert_eq!(res.id, "file-123");
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/files/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No such file",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "file_not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.files().delete("does-not-exist").await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.code, Some(serde_json::json!("file_not_found")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
