//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::client::OpenAI;
use crate::interfaces::images;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::Form;

pub struct Images<'a> {
    openai: &'a OpenAI,
}

impl<'a> Images<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an image given a prompt.
    pub async fn create(
        &self,
        req: &images::CreateImageRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        self.openai.post("/images/generations", req).await
    }

    /// Creates an edited or extended image given an original image and a prompt.
    pub async fn create_edit(
        &self,
        req: &images::CreateImageEditRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new()
            .part("image", file_part)
            .text("prompt", req.prompt.to_string());

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .unwrap();

            form = form.part("mask", file_part);
        }

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/edits", form).await
    }

    /// Creates a variation of a given image.
    pub async fn create_variations(
        &self,
        req: &images::CreateImageVariationRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new().part("image", file_part);

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size);
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user);
        }

        self.openai.post_form("/images/variations", form).await
    }
}
