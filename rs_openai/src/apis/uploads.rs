//! Create large uploads of API requests for asynchronous processing.
//! The Upload API returns completions within 24 hours for a 50% discount. Related guide: [Upload](https://platform.openai.com/docs/guides/upload)

use crate::client::OpenAI;
use crate::interfaces::uploads;
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
        let file_part = reqwest::multipart::Part::stream(req.file.buffer.clone())
            .file_name(req.file.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let form = Form::new()
            .part("file", file_part)
            .text("purpose", req.purpose.to_string());

        self.openai.post_form("/uploads", form).await
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
            .unwrap();
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
