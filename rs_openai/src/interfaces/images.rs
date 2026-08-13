//! Given a prompt and/or an input image, the model will generate a new image.
//!
//! Related guide: [Image generation](https://platform.openai.com/docs/guides/images)

use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Default, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ResponseFormat {
    #[default]
    #[strum(serialize = "url")]
    Url,

    #[strum(serialize = "b64_json")]
    #[serde(rename = "b64_json")]
    B64Json,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Quality {
    #[strum(serialize = "standard")]
    Standard, // dall-e-3
    #[strum(serialize = "hd")]
    Hd, // dall-e-3
    #[strum(serialize = "low")]
    Low, // gpt-image
    #[strum(serialize = "medium")]
    Medium, // gpt-image
    #[strum(serialize = "high")]
    High, // gpt-image
    #[strum(serialize = "auto")]
    Auto, // gpt-image
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Background {
    #[strum(serialize = "transparent")]
    Transparent,
    #[strum(serialize = "opaque")]
    Opaque,
    #[strum(serialize = "auto")]
    Auto,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[strum(serialize = "png")]
    Png,
    #[strum(serialize = "jpeg")]
    Jpeg,
    #[strum(serialize = "webp")]
    Webp,
}

/// Content-moderation strictness for gpt-image models. Request-only — OpenAI does not echo
/// this back on the response, unlike `Quality`/`Background`/`OutputFormat`.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Moderation {
    #[strum(serialize = "low")]
    Low,
    #[strum(serialize = "auto")]
    Auto,
}

/// `dall-e-3`-only: nudges the model toward a more natural, less hyper-real/dramatic look.
/// Request-only.
#[derive(Debug, Serialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    #[strum(serialize = "vivid")]
    Vivid,
    #[strum(serialize = "natural")]
    Natural,
}

/// Edit-only, gpt-image models: how closely the edited output should match the input
/// image's fine details. Request-only.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum InputFidelity {
    #[strum(serialize = "high")]
    High,
    #[strum(serialize = "low")]
    Low,
}

/// The size of the generated/edited image(s). Valid values differ per endpoint:
/// - `create` (generate): all 8 values below.
/// - `create_edit`: `Auto`, `S1024x1024`, `S1536x1024`, `S1024x1536` only.
/// - `create_variations`: `S1024x1024`, `S1536x1024`, `S1024x1536` only (no `256x256`/
///   `512x512`/`Auto` — dall-e-2 only).
#[derive(Default, Debug, Serialize, Deserialize, Clone, PartialEq, strum::Display)]
pub enum ImageSize {
    #[strum(serialize = "256x256")]
    #[serde(rename = "256x256")]
    S256x256,

    #[strum(serialize = "512x512")]
    #[serde(rename = "512x512")]
    S512x512,

    #[strum(serialize = "1024x1024")]
    #[serde(rename = "1024x1024")]
    S1024x1024,

    #[strum(serialize = "1536x1024")]
    #[serde(rename = "1536x1024")]
    S1536x1024,

    #[strum(serialize = "1024x1536")]
    #[serde(rename = "1024x1536")]
    S1024x1536,

    #[strum(serialize = "1792x1024")]
    #[serde(rename = "1792x1024")]
    S1792x1024,

    #[strum(serialize = "1024x1792")]
    #[serde(rename = "1024x1792")]
    S1024x1792,

    #[default]
    #[strum(serialize = "auto")]
    #[serde(rename = "auto")]
    Auto,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageRequest {
    /// A text description of the desired image(s). The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// All 8 `ImageSize` values are valid for generate — see the type's own doc comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    /// Only meaningful for `dall-e-2`/`dall-e-3` — gpt-image models always return base64 and
    /// ignore this field.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user, which can help OpenAI to monitor and detect abuse.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,

    /// ID of the model to use (e.g. `"gpt-image-1.5"`, `"gpt-image-1"`, `"gpt-image-1-mini"`,
    /// `"dall-e-2"` (default), `"dall-e-3"`).
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Compression level 0-100 (percentage). `output_format` must be `jpeg` or `webp`.
    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u8>,

    /// gpt-image models only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<Moderation>,

    /// `dall-e-3` only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<Style>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageEditRequest {
    /// The image(s) to edit. Sent as repeated `image[]` multipart parts (up to 16 for
    /// gpt-image models). Each entry must be a valid PNG, WebP, or JPG file, less than 25MB.
    /// If no `mask` is provided, an image with transparency will have that transparency used
    /// as the mask.
    pub images: Vec<File>,

    /// A text description of the desired edit. The maximum length is 1000 characters.
    #[builder(into)]
    pub prompt: String,

    /// An additional image whose fully transparent areas indicate where the input image(s)
    /// should be edited. Sent as a single `mask` part.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mask: Option<File>,

    /// ID of the model to use. Edit is gpt-image-only — no `dall-e-2`/`dall-e-3` support
    /// (e.g. `"gpt-image-1.5"`, `"gpt-image-1"`, `"gpt-image-1-mini"`, `"chatgpt-image-latest"`).
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub quality: Option<Quality>,

    /// Only `Auto`, `S1024x1024`, `S1536x1024`, `S1024x1536` are valid for edit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<Background>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<OutputFormat>,

    /// Compression level 0-100 (percentage). `output_format` must be `jpeg` or `webp`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_compression: Option<u8>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub moderation: Option<Moderation>,

    /// How closely the edited output should match the input image's fine details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_fidelity: Option<InputFidelity>,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>,

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateImageVariationRequest {
    /// The image to use as the basis for the variation(s). Must be a valid PNG file, less than 4MB, and square.
    pub image: File,

    /// The number of images to generate. Must be between 1 and 10.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub n: Option<u8>, // default: 1, min: 1, max: 10

    /// Only `S1024x1024`, `S1536x1024`, `S1024x1536` are valid for variation — dall-e-2 only,
    /// no `256x256`/`512x512`/`Auto`. See `ImageSize`'s own doc comment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<ImageSize>,

    /// The format in which the generated images are returned. Must be one of `url` or `b64_json`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseFormat>, // default: "url"

    /// A unique identifier representing your end-user.
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageData {
    pub url: Option<String>,
    pub b64_json: Option<String>,
    /// Only present for `dall-e-3` generations — the prompt actually used, which may differ
    /// from the requested prompt due to automatic revision.
    pub revised_prompt: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageResponse {
    pub created: i64,
    pub data: Vec<ImageData>,
    pub background: Option<Background>,
    pub output_format: Option<OutputFormat>,
    pub quality: Option<Quality>,
    pub size: Option<ImageSize>,
    /// Token accounting for gpt-image-model generations/edits. `None` for dall-e models.
    pub usage: Option<ImageUsage>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageUsage {
    pub input_tokens: u32,
    pub input_tokens_details: ImageUsageTokenDetails,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub output_tokens_details: Option<ImageUsageTokenDetails>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ImageUsageTokenDetails {
    pub image_tokens: u32,
    pub text_tokens: u32,
}
