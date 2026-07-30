//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::client::OpenAI;
use crate::interfaces::images;
pub use crate::interfaces::images::*;
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
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new()
            .part("image", file_part)
            .text("prompt", req.prompt.to_string());

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(mask.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

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
            .expect("\"application/octet-stream\" is always a valid mime type");

        let mut form = Form::new().part("image", file_part);

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

        self.openai.post_form("/images/variations", form).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::images::{
        CreateImageEditRequest, CreateImageRequest, ImageSize, ResponseFormat,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use crate::shared::types::File;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }

    #[tokio::test]
    async fn create_serializes_1024_size_correctly() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .and(body_json(serde_json::json!({
                "prompt": "a red bicycle",
                "size": "1024x1024",
                "response_format": "url"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"url": "https://example.com/image.png"}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a red bicycle")
            .size(ImageSize::S1024x1024)
            .response_format(ResponseFormat::Url)
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "prompt is required",
                    "type": "invalid_request_error",
                    "param": "prompt",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder().prompt("").build();

        let err = client.images().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("prompt")))
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    /// Regression test for a bug where the "mask" multipart part streamed the main image's
    /// bytes (`req.image.buffer`) instead of the mask's own bytes (`mask.buffer`), silently
    /// uploading the image twice instead of image+mask. Uses distinct byte contents for the
    /// image and the mask so the assertion can tell them apart in the raw multipart body.
    #[tokio::test]
    async fn create_edit_sends_masks_own_bytes_not_the_images() {
        let mock_server = MockServer::start().await;

        let image_bytes: &[u8] = &[1, 2, 3];
        let mask_bytes: &[u8] = &[4, 5, 6];

        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .and(ContainsInBody(mask_bytes))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"url": "https://example.com/image.png"}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageEditRequest::builder()
            .image(File {
                buffer: image_bytes.to_vec(),
                filename: "image.png".into(),
            })
            .mask(File {
                buffer: mask_bytes.to_vec(),
                filename: "mask.png".into(),
            })
            .prompt("add a hat")
            .build();

        let res = client.images().create_edit(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }
}
