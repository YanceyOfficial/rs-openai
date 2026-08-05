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

    /// Creates an edited or extended image given one or more original images and a prompt.
    pub async fn create_edit(
        &self,
        req: &images::CreateImageEditRequest,
    ) -> OpenAIResponse<images::ImageResponse> {
        let mut form = Form::new().text("prompt", req.prompt.to_string());

        for image in &req.images {
            let file_part = reqwest::multipart::Part::stream(image.buffer.clone())
                .file_name(image.filename.clone())
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("image[]", file_part);
        }

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(mask.buffer.clone())
                .file_name(mask.filename)
                .mime_str("application/octet-stream")
                .expect("\"application/octet-stream\" is always a valid mime type");

            form = form.part("mask", file_part);
        }

        if let Some(model) = req.model.clone() {
            form = form.text("model", model);
        }

        if let Some(quality) = req.quality.clone() {
            form = form.text("quality", quality.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(background) = req.background.clone() {
            form = form.text("background", background.to_string());
        }

        if let Some(output_format) = req.output_format.clone() {
            form = form.text("output_format", output_format.to_string());
        }

        if let Some(output_compression) = req.output_compression {
            form = form.text("output_compression", output_compression.to_string());
        }

        if let Some(moderation) = req.moderation.clone() {
            form = form.text("moderation", moderation.to_string());
        }

        if let Some(input_fidelity) = req.input_fidelity.clone() {
            form = form.text("input_fidelity", input_fidelity.to_string());
        }

        if let Some(n) = req.n {
            form = form.text("n", n.to_string());
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
        Background, CreateImageEditRequest, CreateImageRequest, ImageSize, InputFidelity,
        Moderation, OutputFormat, Quality, ResponseFormat, Style,
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

    /// Counts non-overlapping occurrences of `needle` in the raw request body and matches if
    /// the count is at least `min_count`. Used to prove multiple `image[]` multipart parts
    /// were actually sent, not just that some bytes appear once.
    struct ContainsAtLeastNInBody(&'static [u8], usize);

    impl wiremock::Match for ContainsAtLeastNInBody {
        fn matches(&self, request: &Request) -> bool {
            let mut count = 0;
            let mut start = 0;
            while let Some(pos) = request.body[start..]
                .windows(self.0.len())
                .position(|window| window == self.0)
            {
                count += 1;
                start += pos + self.0.len();
            }
            count >= self.1
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

    /// Regression test for a real bug: `ImageData` used to be a two-variant enum
    /// (`Url(String) | B64Json(String)`) that only coincidentally deserialized today's
    /// single-field mock responses (Rust's default externally-tagged newtype-variant JSON
    /// happens to look like `{"url": "..."}"`) — it could never represent a real dall-e-3
    /// response, which has `url` AND `revised_prompt` together on the same item.
    #[tokio::test]
    async fn create_deserializes_dalle3_response_with_url_and_revised_prompt() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{
                    "url": "https://example.com/image.png",
                    "revised_prompt": "A vivid red bicycle leaning against a brick wall"
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a red bicycle")
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(
            res.data[0].url.as_deref(),
            Some("https://example.com/image.png")
        );
        assert_eq!(
            res.data[0].revised_prompt.as_deref(),
            Some("A vivid red bicycle leaning against a brick wall")
        );
        assert_eq!(res.data[0].b64_json, None);
    }

    #[tokio::test]
    async fn create_deserializes_response_level_fields_including_usage() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "background": "transparent",
                "output_format": "png",
                "quality": "high",
                "size": "1024x1024",
                "usage": {
                    "input_tokens": 50,
                    "input_tokens_details": {"image_tokens": 30, "text_tokens": 20},
                    "output_tokens": 1200,
                    "total_tokens": 1250,
                    "output_tokens_details": {"image_tokens": 1200, "text_tokens": 0}
                },
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a transparent PNG icon")
            .model("gpt-image-1")
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.background, Some(Background::Transparent));
        assert_eq!(res.output_format, Some(OutputFormat::Png));
        assert_eq!(res.quality, Some(Quality::High));
        assert_eq!(res.size, Some(ImageSize::S1024x1024));

        let usage = res.usage.as_ref().expect("expected usage to be present");
        assert_eq!(usage.input_tokens, 50);
        assert_eq!(usage.input_tokens_details.image_tokens, 30);
        assert_eq!(usage.input_tokens_details.text_tokens, 20);
        assert_eq!(usage.output_tokens, 1200);
        assert_eq!(usage.total_tokens, 1250);
        let output_details = usage
            .output_tokens_details
            .as_ref()
            .expect("expected output_tokens_details to be present");
        assert_eq!(output_details.image_tokens, 1200);
        assert_eq!(output_details.text_tokens, 0);
    }

    #[tokio::test]
    async fn create_serializes_gpt_image_capability_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/images/generations"))
            .and(body_json(serde_json::json!({
                "prompt": "a vivid sunset over mountains",
                "model": "gpt-image-1",
                "quality": "high",
                "background": "opaque",
                "output_format": "png",
                "output_compression": 80,
                "moderation": "low",
                "style": "vivid"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageRequest::builder()
            .prompt("a vivid sunset over mountains")
            .model("gpt-image-1")
            .quality(Quality::High)
            .background(Background::Opaque)
            .output_format(OutputFormat::Png)
            .output_compression(80)
            .moderation(Moderation::Low)
            .style(Style::Vivid)
            .build();

        let res = client.images().create(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
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
            .images(vec![File {
                buffer: image_bytes.to_vec(),
                filename: "image.png".into(),
            }])
            .mask(File {
                buffer: mask_bytes.to_vec(),
                filename: "mask.png".into(),
            })
            .prompt("add a hat")
            .build();

        let res = client.images().create_edit(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    /// Proves multiple images are sent as repeated `image[]` multipart parts (the real
    /// OpenAI edit endpoint's wire field name — confirmed via live docs, see
    /// docs/superpowers/specs/2026-08-04-images-model-field-design.md), each with its own
    /// distinct bytes, not collapsed into a single part or dropped.
    #[tokio::test]
    async fn create_edit_sends_multiple_images_as_repeated_image_bracket_parts() {
        let mock_server = MockServer::start().await;

        let first_image_bytes: &[u8] = &[10, 20, 30];
        let second_image_bytes: &[u8] = &[40, 50, 60];

        Mock::given(method("POST"))
            .and(path("/images/edits"))
            .and(ContainsInBody(first_image_bytes))
            .and(ContainsInBody(second_image_bytes))
            .and(ContainsAtLeastNInBody(br#"name="image[]""#, 2))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created": 1700000000_i64,
                "data": [{"b64_json": "aGVsbG8="}]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateImageEditRequest::builder()
            .images(vec![
                File {
                    buffer: first_image_bytes.to_vec(),
                    filename: "first.png".into(),
                },
                File {
                    buffer: second_image_bytes.to_vec(),
                    filename: "second.png".into(),
                },
            ])
            .prompt("combine these into one scene")
            .model("gpt-image-1")
            .quality(Quality::High)
            .input_fidelity(InputFidelity::High)
            .build();

        let res = client.images().create_edit(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }
}
