use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateModerationRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateModerationRequest {
    /// Input (or inputs) to classify. Can be a single string, an array of strings, or an array of multi-modal input objects similar to other models.
    pub input: ModerationInput,

    /// The content moderation model you would like to use. Learn more in [the moderation guide](https://platform.openai.com/docs/guides/moderation), and learn about available models [here](https://platform.openai.com/docs/models/moderation).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>, // default: "omni-moderation-latest"
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum ModerationInput {
    /// A string of text to classify for moderation.
    String(String),
    /// An array of strings to classify for moderation.
    ArrayOfString(Vec<String>),
    /// An array of multi-modal inputs to the moderation model.
    ArrayOfMultiModalInput(Vec<ArrayOfMultiModalInput>),
}

#[derive(Debug, Serialize, Clone)]
pub enum ArrayOfMultiModalInput {
    /// An object describing an image to classify.
    Image(Image),
    /// An object describing text to classify.
    Text(Text),
}

#[derive(Debug, Serialize, Clone)]
pub struct Image {
    /// Always `image_url`.
    pub r#type: String,
    /// Contains either an image URL or a data URL for a base64 encoded image.
    pub image_url: ImageUrl,
}

#[derive(Debug, Serialize, Clone)]
pub struct ImageUrl {
    /// Either a URL of the image or the base64 encoded image data.
    pub url: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct Text {
    /// Always `text`.
    pub r#type: String,
    /// A string of text to classify.
    pub text: String,
}

/// Represents if a given text input is potentially harmful.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ModerationResponse {
    /// The unique identifier for the moderation request.
    pub id: String,
    /// The unique identifier for the moderation request.
    pub model: String,
    /// A list of moderation objects.
    pub results: Vec<ModerationCategory>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ModerationCategory {
    /// Whether any of the below categories are flagged.
    pub flagged: bool,
    /// A list of the categories, and whether they are flagged or not.
    pub categories: Categories,
    /// A list of the categories along with their scores as predicted by model.
    pub category_scores: CategoryScores,
    /// A list of the categories along with the input type(s) that the score applies to.
    pub category_applied_input_types: CategoryAppliedInputTypes,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Categories {
    /// Content that expresses, incites, or promotes harassing language towards any target.
    pub harassment: bool,
    /// Harassment content that also includes violence or serious harm towards any target.
    #[serde(rename = "harassment/threatening")]
    pub harassment_threatening: bool,
    /// Content meant to arouse sexual excitement, such as the description of sexual activity, or that promotes sexual services (excluding sex education and wellness).
    pub sexual: bool,
    /// Content that expresses, incites, or promotes hate based on race, gender, ethnicity, religion, nationality, sexual orientation, disability status, or caste. Hateful content aimed at non-protected groups (e.g., chess players) is harassment.
    pub hate: bool,
    #[serde(rename = "hate/threatening")]
    /// Hateful content that also includes violence or serious harm towards the targeted group based on race, gender, ethnicity, religion, nationality, sexual orientation, disability status, or caste.
    pub hate_threatening: bool,
    /// Content that includes instructions or advice that facilitate the planning or execution of wrongdoing, or that gives advice or instruction on how to commit illicit acts. For example, "how to shoplift" would fit this category.
    pub illicit: bool,
    /// Content that includes instructions or advice that facilitate the planning or execution of wrongdoing that also includes violence, or that gives advice or instruction on the procurement of any weapon.
    #[serde(rename = "illicit/violent")]
    pub illicit_violent: bool,
    /// Content where the speaker expresses that they are engaging or intend to engage in acts of self-harm, such as suicide, cutting, and eating disorders.
    #[serde(rename = "self-harm/intent")]
    pub self_harm_intent: bool,
    /// Content that encourages performing acts of self-harm, such as suicide, cutting, and eating disorders, or that gives instructions or advice on how to commit such acts.
    #[serde(rename = "self-harm/instructions")]
    pub self_harm_instructions: bool,
    /// Content that promotes, encourages, or depicts acts of self-harm, such as suicide, cutting, and eating disorders.
    #[serde(rename = "self-harm")]
    pub self_harm: bool,
    /// Sexual content that includes an individual who is under 18 years old.
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: bool,
    /// Content that depicts death, violence, or physical injury.
    pub violence: bool,
    /// Content that depicts death, violence, or physical injury in graphic detail.
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CategoryScores {
    /// The score for the category 'harassment'.
    pub harassment: f64,
    /// The score for the category 'harassment/threatening'.
    #[serde(rename = "harassment/threatening")]
    pub harassment_threatening: f64,
    /// The score for the category 'sexual'.
    pub sexual: f64,
    /// The score for the category 'hate'.
    pub hate: f64,
    /// The score for the category 'hate/threatening'.
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: f64,
    /// The score for the category 'illicit'.
    pub illicit: f64,
    #[serde(rename = "illicit/violent")]
    /// The score for the category 'illicit/violent'.
    pub illicit_violent: f64,
    /// The score for the category 'self-harm/intent'.
    #[serde(rename = "self-harm/intent")]
    pub self_harm_intent: f64,
    /// The score for the category 'self-harm/instructions'.
    #[serde(rename = "self-harm/instructions")]
    pub self_harm_instructions: f64,
    /// The score for the category 'self-harm'.
    #[serde(rename = "self-harm")]
    pub self_harm: f64,
    /// The score for the category 'sexual/minors'.
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: f64,
    /// The score for the category 'violence'.
    pub violence: f64,
    /// The score for the category 'violence/graphic'.
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: f64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CategoryAppliedInputTypes {
    /// The applied input type(s) for the category 'harassment'.
    pub harassment: Vec<String>,
    /// The applied input type(s) for the category 'harassment/threatening'.
    #[serde(rename = "harassment/threatening")]
    pub harassment_threatening: Vec<String>,
    /// The applied input type(s) for the category 'sexual'.
    pub sexual: Vec<String>,
    // The applied input type(s) for the category 'hate'.
    pub hate: Vec<String>,
    /// The applied input type(s) for the category 'hate/threatening'.
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: Vec<String>,
    /// The applied input type(s) for the category 'illicit'.
    pub illicit: Vec<String>,
    #[serde(rename = "illicit/violent")]
    /// The applied input type(s) for the category 'illicit/violent'.
    pub illicit_violent: Vec<String>,
    /// The applied input type(s) for the category 'self-harm/intent'.
    #[serde(rename = "self-harm/intent")]
    pub self_harm_intent: Vec<String>,
    /// The applied input type(s) for the category 'self-harm/instructions'.
    #[serde(rename = "self-harm/instructions")]
    pub self_harm_instructions: Vec<String>,
    /// The applied input type(s) for the category 'self-harm'.
    #[serde(rename = "self-harm")]
    pub self_harm: Vec<String>,
    /// The applied input type(s) for the category 'sexual/minors'.
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: Vec<String>,
    /// The applied input type(s) for the category 'violence'.
    pub violence: Vec<String>,
    /// The applied input type(s) for the category 'violence/graphic'.
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: Vec<String>,
}
