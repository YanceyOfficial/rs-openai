//! Given a input text, outputs if the model classifies it as violating OpenAI's content policy.
//!
//! Related guide: [Moderations](https://platform.openai.com/docs/guides/moderation)

use crate::{OpenAI, OpenAIResponse};
use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum ModerationInput {
    String(String),
    ArrayOfString(Vec<String>),
}

#[derive(Debug, Serialize, Default, Clone)]
pub enum ModerationModel {
    #[default]
    #[serde(rename = "text-moderation-latest")]
    Latest,
    #[serde(rename = "text-moderation-stable")]
    Stable,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateModerationRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateModerationRequest {
    /// The input text to classify.
    pub input: ModerationInput,

    /// Two content moderations models are available: `text-moderation-stable` and `text-moderation-latest`.
    ///
    /// The default is `text-moderation-latest` which will be automatically upgraded over time.
    /// This ensures you are always using our most accurate model.
    /// If you use `text-moderation-stable`, we will provide advanced notice before updating the model.
    /// Accuracy of `text-moderation-stable` may be slightly lower than for `text-moderation-latest`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<ModerationModel>, // default: "text-moderation-latest"
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModerationResponse {
    pub id: String,
    pub model: String,
    pub results: Vec<ModerationCategory>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModerationCategory {
    pub categories: ModerationCategories,
    pub category_scores: ModerationCategoryScores,
    pub flagged: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModerationCategories {
    pub sexual: bool,
    pub hate: bool,
    pub violence: bool,
    #[serde(rename = "self-harm")]
    pub self_harm: bool,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: bool,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: bool,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModerationCategoryScores {
    pub sexual: f32,
    pub hate: f32,
    pub violence: f32,
    #[serde(rename = "self-harm")]
    pub self_harm: f32,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: f32,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: f32,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: f32,
}

pub struct Moderations<'a> {
    openai: &'a OpenAI,
}

impl<'a> Moderations<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Classifies if text violates OpenAI's Content Policy.
    pub async fn create(&self, req: &CreateModerationRequest) -> OpenAIResponse<ModerationResponse> {
        self.openai.post("/moderations", req).await
    }
}
