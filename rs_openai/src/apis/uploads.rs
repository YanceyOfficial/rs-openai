//! Create large uploads of API requests for asynchronous processing.
//! The Upload API returns completions within 24 hours for a 50% discount. Related guide: [Upload](https://platform.openai.com/docs/guides/upload)

use crate::client::OpenAI;
use crate::interfaces::uploads;
pub use crate::interfaces::uploads::*;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::Form;

pub struct Upload<'a> {
    openai: &'a OpenAI,
}

impl<'a> Upload<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an intermediate [Upload](https://platform.openai.com/docs/api-reference/uploads/object) object that you can add Parts to. Currently,
    /// an Upload can accept at most 8 GB in total and expires after an hour after you create it.
    ///
    /// Once you complete the Upload, we will create a [File](https://platform.openai.com/docs/api-reference/files/object) object that contains all the parts you uploaded.
    /// This File is usable in the rest of our platform as a regular File object.
    ///
    /// For certain `purposes`, the correct `mime_type` must be specified. Please refer to documentation for the supported MIME types for your use case:
    ///
    /// - [Assistants](https://platform.openai.com/docs/assistants/tools/file-search/supported-files)
    ///
    /// For guidance on the proper filename extensions for each purpose, please follow the documentation on [creating a File](https://platform.openai.com/docs/api-reference/files/create).
    pub async fn upload(
        &self,
        req: &uploads::UploadFileRequest,
    ) -> OpenAIResponse<uploads::UploadFileResponse> {
        self.openai.post("/uploads", req).await
    }

    /// Adds a [Part](https://platform.openai.com/docs/api-reference/uploads/part-object) to an [Upload](https://platform.openai.com/docs/api-reference/uploads/object) object. A Part represents a chunk of bytes from the file you are trying to upload.
    ///
    /// Each Part can be at most 64 MB, and you can add Parts until you hit the Upload maximum of 8 GB.
    ///
    /// It is possible to add multiple Parts in parallel. You can decide the intended order of the Parts when you [complete the Upload](https://platform.openai.com/docs/api-reference/uploads/complete).
    pub async fn add_upload_part(
        &self,
        upload_id: &str,
        req: &uploads::AddUploadPartRequest,
    ) -> OpenAIResponse<uploads::AddUploadPartResponse> {
        let file_part = reqwest::multipart::Part::stream(req.data.buffer.clone())
            .file_name(req.data.filename.clone())
            .mime_str("application/octet-stream")
            .expect("\"application/octet-stream\" is always a valid mime type");
        let form = Form::new().part("data", file_part);

        self.openai
            .post_form(&format!("/uploads/{upload_id}/parts"), form)
            .await
    }

    /// Completes the [Upload](https://platform.openai.com/docs/api-reference/uploads/object).
    ///
    /// Within the returned Upload object, there is a nested [File](https://platform.openai.com/docs/api-reference/files/object) object that is ready to use in the rest of the platform.
    ///
    /// You can specify the order of the Parts by passing in an ordered list of the Part IDs.
    ///
    /// The number of bytes uploaded upon completion must match the number of bytes initially specified when creating the Upload object. No Parts may be added after an Upload is completed.
    pub async fn complete_upload(
        &self,
        upload_id: &str,
        req: &uploads::CompleteUploadRequest,
    ) -> OpenAIResponse<uploads::CompleteUploadResponse> {
        self.openai
            .post(&format!("/uploads/{upload_id}/complete"), req)
            .await
    }

    /// Cancels the Upload. No Parts may be added after an Upload is cancelled.
    pub async fn cancel_upload(
        &self,
        upload_id: &str,
    ) -> OpenAIResponse<uploads::UploadFileResponse> {
        self.openai
            .post(&format!("/uploads/{upload_id}/cancel"), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::uploads::{CompleteUploadRequest, UploadFileRequest, UploadStatus};
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::FilePurpose;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    /// Regression test for a real bug: `upload()` used to send a multipart file to
    /// `POST /uploads`, but that endpoint is JSON-only — it takes upload *metadata* to open a
    /// session, not a file (actual bytes go through the separate, already-correct
    /// `add_upload_part`). This asserts the request body is JSON with exactly the four
    /// metadata fields, proving the fix rather than just exercising a happy path that would
    /// have passed under the old, broken implementation too.
    #[tokio::test]
    async fn upload_sends_json_metadata_not_multipart() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads"))
            .and(body_json(serde_json::json!({
                "bytes": 1024,
                "filename": "data.jsonl",
                "mime_type": "application/jsonl",
                "purpose": "fine-tune"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "upload_123", "object": "upload", "bytes": 1024,
                "created_at": 1700000000_i64, "filename": "data.jsonl",
                "purpose": "fine-tune", "status": "pending", "expires_at": 1700003600_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UploadFileRequest::builder()
            .bytes(1024_u64)
            .filename("data.jsonl")
            .mime_type("application/jsonl")
            .purpose(FilePurpose::FineTune)
            .build();

        let res = client.upload().upload(&req).await.unwrap();
        assert_eq!(res.id, "upload_123");
        assert_eq!(res.status, UploadStatus::Pending);
    }

    #[tokio::test]
    async fn complete_upload_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/uploads/upload_123/complete"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "byte count mismatch",
                    "type": "invalid_request_error",
                    "param": "part_ids",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CompleteUploadRequest::builder()
            .part_ids(vec!["part_1".to_string()])
            .build();

        let err = client
            .upload()
            .complete_upload("upload_123", &req)
            .await
            .unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("part_ids")))
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }
}
