//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use super::{OpenAI, OpenAIResponse};
use crate::error::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageRequest {
    /// A text description of the desired image(s). The maximum length is 1000 characters.
    pub prompt: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse. [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageEditRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageEditRequest {
    /// The image to edit. Must be a valid PNG file, less than 4MB, and square.
    /// If mask is not provided, image must have transparency, which will be used as the mask.
    pub image: String,

    /// An additional image whose fully transparent areas (e.g. where alpha is zero) indicate where `image` should be edited.
    /// Must be a valid PNG file, less than 4MB, and have the same dimensions as `image`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<String>,

    /// A text description of the desired image(s). The maximum length is 1000 characters.
    pub prompt: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    /// [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateImageVariationRequestArgs")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateImageVariationRequest {
    /// The image to use as the basis for the variation(s). Must be a valid PNG file, less than 4MB, and square.
    pub image: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<i32>,

    /// The size of the generated images. Must be one of `256x256`, `512x512`, or `1024x1024`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<String>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<String>,

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    /// [Learn more](https://beta.openai.com/docs/api-reference/authentication)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Url {
    pub url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<Url>,
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
    pub async fn create_image(&self, req: &CreateImageRequest) -> OpenAIResponse<ImageResponse> {
        self.openai.post("/images/generations", req).await
    }

    /// Creates an edited or extended image given an original image and a prompt.
    #[tokio::main]
    pub async fn create_edit(&self, req: &CreateImageEditRequest) -> OpenAIResponse<ImageResponse> {
        self.openai.post("/images/edits", req).await
    }

    /// Creates a variation of a given image.
    #[tokio::main]
    pub async fn create_variations(
        &self,
        req: &CreateImageVariationRequest,
    ) -> OpenAIResponse<ImageResponse> {
        self.openai.post("/images/variations", req).await
    }
}
