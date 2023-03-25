//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::{OpenAI, OpenAIResponse};
use crate::shared::response_wrapper::OpenAIError;
use crate::shared::types::FileMeta;
use derive_builder::Builder;
use reqwest::multipart::Form;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    #[strum(serialize = "url")]
    Url,
    #[strum(serialize = "b64_json")]
    #[serde(rename = "b64_json")]
    B64Json,
}

#[derive(Default, Debug, Serialize, Clone, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    S256x256,
    #[strum(serialize = "512x512")]
    S512x512,
    #[default]
    #[strum(serialize = "1024x1024")]
    S1024x1024,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageRequest {
    /// A text description of the desired image(s). The maximum length is 1000 characters.
    pub prompt: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>, // default: 1, min: 1, max: 10

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageEditRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageEditRequest {
    /// The image to edit. Must be a valid PNG file, less than 4MB, and square.
    /// If mask is not provided, image must have transparency, which will be used as the mask.
    pub image: FileMeta,

    /// A text description of the desired image(s). The maximum length is 1000 characters.
    pub prompt: String,

    /// An additional image whose fully transparent areas (e.g. where alpha is zero) indicate where `image` should be edited.
    /// Must be a valid PNG file, less than 4MB, and have the same dimensions as `image`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<FileMeta>,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    /// [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageVariationRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageVariationRequest {
    /// The image to use as the basis for the variation(s). Must be a valid PNG file, less than 4MB, and square.
    pub image: FileMeta,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    /// [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}


#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ImageData {
    Url(String),
    #[serde(rename = "b64_json")]
    B64Json(String),
}
#[derive(Debug, Deserialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
}

pub struct Images<'a> {
    openai: &'a OpenAI<'a>,
}

impl<'a> Images<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an image given a prompt.
    #[tokio::main]
    pub async fn create(&self, req: &CreateImageRequest) -> OpenAIResponse<ResponseFormat> {
        self.openai.post("/images/generations", req).await
    }

    /// Creates an edited or extended image given an original image and a prompt.
    #[tokio::main]
    pub async fn create_edit(
        &self,
        req: &CreateImageEditRequest,
    ) -> OpenAIResponse<ResponseFormat> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new()
            .part("image", file_part)
            .text("prompt", req.prompt.to_string());

        if let Some(mask) = req.mask.clone() {
            let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
                .file_name(mask.filename.clone())
                .mime_str("application/octet-stream")
                .unwrap();

            form = form.part("mask", file_part);
        }

        if let Some(n) = req.n.clone() {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user.to_string());
        }

        self.openai.post_form("/images/edits", form).await
    }

    /// Creates a variation of a given image.
    #[tokio::main]
    pub async fn create_variations(
        &self,
        req: &CreateImageVariationRequest,
    ) -> OpenAIResponse<ResponseFormat> {
        let file_part = reqwest::multipart::Part::stream(req.image.buffer.clone())
            .file_name(req.image.filename.clone())
            .mime_str("application/octet-stream")
            .unwrap();

        let mut form = Form::new().part("image", file_part);

        if let Some(n) = req.n.clone() {
            form = form.text("n", n.to_string());
        }

        if let Some(size) = req.size.clone() {
            form = form.text("size", size.to_string());
        }

        if let Some(response_format) = req.response_format.clone() {
            form = form.text("response_format", response_format.to_string());
        }

        if let Some(user) = req.user.clone() {
            form = form.text("user", user.to_string());
        }

        self.openai.post_form("/images/variations", form).await
    }
}
