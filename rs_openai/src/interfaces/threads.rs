use super::assistants::ToolResources;
use bon::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CreateThreadRequest {
    /// A list of [messages](https://platform.openai.com/docs/api-reference/messages) to start the thread with.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub messages: Option<Vec<Message>>,

    /// A set of resources that are used by the assistant's tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Message {
    /// The role of the entity that is creating the message: `user` or `assistant`.
    #[builder(into)]
    pub role: String,
    pub content: Content,
    /// A list of files attached to the message, and the tools they should be added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Content {
    TextContent(String),
    ArrayOfContentParts(Vec<ArrayOfContentParts>),
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum ArrayOfContentParts {
    ImageFileType(ImageFileType),
    ImageUrlType(ImageUrlType),
    TextType(TextType),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFileType {
    #[builder(into)]
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageFile {
    #[builder(into)]
    pub file_id: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrlType {
    #[builder(into)]
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ImageUrl {
    #[builder(into)]
    pub url: String,
    #[builder(into)]
    pub detail: Option<String>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct TextType {
    #[builder(into)]
    pub r#type: String,
    #[builder(into)]
    pub text: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct Attachment {
    #[builder(into)]
    pub file_id: Option<String>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct CodeInterpreterTool {
    #[builder(into)]
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct FileSearchTool {
    #[builder(into)]
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ThreadResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub metadata: HashMap<String, serde_json::Value>,
    pub tool_resources: Option<ToolResources>,
}

#[derive(Builder, Clone, Debug, Serialize)]
pub struct ModifyThreadRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteThreadResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
