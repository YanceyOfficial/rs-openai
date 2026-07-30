use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateMessageRequest {
    /// The role of the entity that is creating the message: `user` or `assistant`.
    #[builder(into)]
    pub role: String,
    /// The content of the message in array of text and/or images.
    pub content: Content,
    /// A list of files attached to the message, and the tools they should be added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(untagged)]
pub enum Content {
    /// The text contents of the message.
    TextContent(String),
    /// An array of content parts with a defined type.
    ArrayOfContentParts(Vec<ArrayOfContentParts>),
}

impl Default for Content {
    fn default() -> Self {
        Content::TextContent("".to_string())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(untagged)]
pub enum ArrayOfContentParts {
    /// References an image [File](https://platform.openai.com/docs/api-reference/files) in the content of a message.
    ImageFileType(ImageFileType),
    /// References an image URL in the content of a message.
    ImageUrlType(ImageUrlType),
    /// The text content that is part of a message.
    TextType(TextType),
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageFileType {
    /// Always `image_file`.
    #[builder(into)]
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageFile {
    /// The [File](https://platform.openai.com/docs/api-reference/files) ID of the image in the message content.
    #[builder(into)]
    pub file_id: String,

    /// `low` uses fewer tokens, you can opt in to high resolution using `high`.
    #[builder(into)]
    pub detail: Option<String>, // Defaults to auto
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageUrlType {
    /// The type of the content part.
    #[builder(into)]
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The external URL of the image: jpeg, jpg, png, gif, or webp.
    #[builder(into)]
    pub url: String,
    /// `low` uses fewer tokens, you can opt in to high resolution using `high`. Defaults to auto.
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct TextType {
    /// Always `text`.
    #[builder(into)]
    pub r#type: String,
    /// Text content to be sent to the model.
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct Attachment {
    /// The ID of the file to attach to the message.
    #[builder(into)]
    pub file_id: Option<String>,
    /// The tools to add this file to.
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct CodeInterpreterTool {
    /// The type of tool being defined: `code_interpreter`.
    #[builder(into)]
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Serialize, Deserialize)]
pub struct FileSearchTool {
    /// The type of tool being defined: `file_search`.
    #[builder(into)]
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub thread_id: String,
    pub status: String,
    pub incomplete_details: Option<IncompleteDetails>,
    pub completed_at: u64,
    pub incomplete_at: u64,
    pub role: String,
    pub content: Content,
    pub assistant_id: Option<String>,
    pub run_id: Option<String>,
    pub attachments: Option<Vec<Attachment>>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The reason the message is incomplete.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct IncompleteDetails {
    pub reason: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ListMessageRequest {
    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    pub limit: Option<i32>,
    /// Sort order by the `created_at` timestamp of the objects.
    #[builder(into)]
    pub order: Option<String>,
    /// A cursor for use in pagination.
    #[builder(into)]
    pub after: Option<String>,
    /// A cursor for use in pagination.
    #[builder(into)]
    pub before: Option<String>,
    /// Filter messages by the run ID that generated them.
    #[builder(into)]
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListMessageResponse {
    pub object: String,
    pub data: Vec<MessageResponse>,
    first_id: String,
    last_id: String,
    has_more: bool,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ModifyMessageRequest {
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteMessageResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
