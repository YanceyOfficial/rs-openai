use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateMessageRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct CreateMessageRequest {
    /// The role of the entity that is creating the message. Allowed values include:
    /// - `user:` Indicates the message is sent by an actual user and should be used in most cases to represent user-generated messages.
    /// - `assistant:` Indicates the message is generated by the assistant. Use this value to insert messages from the assistant into the conversation.
    pub role: String,
    /// The content of the message in array of text and/or images.
    pub content: Content,
    /// A list of files attached to the message, and the tools they should be added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    /// This can be useful for storing additional information about the object in a structured format.
    /// Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(untagged)]
pub enum Content {
    /// The text contents of the message.
    TextContent(String),
    /// An array of content parts with a defined type, each can be of type `text` or images can be passed with `image_url` or `image_file`.
    /// Image types are only supported on [Vision-compatible models](https://platform.openai.com/docs/models/overview).
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

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageFileType {
    /// Always `image_file`.
    pub r#type: String,
    pub image_file: ImageFile,
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageFile {
    /// The [File](https://platform.openai.com/docs/api-reference/files) ID of the image in the message content. Set `purpose="vision"` when uploading the File if you need to later display the file content.
    pub file_id: String,

    /// Specifies the detail level of the image if specified by the user. `low` uses fewer tokens, you can opt in to high resolution using `high`.
    pub detail: Option<String>, // Defaults to auto
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageUrlType {
    /// The type of the content part.
    pub r#type: String,
    pub image_url: ImageUrl,
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct ImageUrl {
    /// The external URL of the image, must be a supported image types: jpeg, jpg, png, gif, webp.
    pub url: String,
    /// Specifies the detail level of the image if specified by the user. `low` uses fewer tokens, you can opt in to high resolution using `high`. Defaults to auto
    pub detail: Option<String>, // Defaults to auto
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextType {
    /// Always `text`.
    pub r#type: String,
    /// Text content to be sent to the model
    pub text: String,
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct Attachment {
    /// The ID of the file to attach to the message.
    pub file_id: Option<String>,
    /// The tools to add this file to.
    pub tools: Option<Vec<Tool>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum Tool {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct CodeInterpreterTool {
    /// The type of tool being defined: `code_interpreter`
    pub r#type: String,
}

#[derive(Builder, Clone, Debug, Default, Serialize, Deserialize)]
pub struct FileSearchTool {
    /// The type of tool being defined: `file_search`
    pub r#type: String,
}

/// Represents a thread that contains [messages](https://platform.openai.com/docs/api-reference/messages).
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct MessageResponse {
    /// The identifier, which can be referenced in API endpoints.
    pub id: String,
    /// The object type, which is always `thread`.
    pub object: String,
    /// The Unix timestamp (in seconds) for when the thread was created.
    pub created_at: u64,
    /// The [thread](https://platform.openai.com/docs/api-reference/threads) ID that this message belongs to.
    pub thread_id: String,
    /// The status of the message, which can be either `in_progress`, `incomplete`, or `completed`.
    pub status: String,
    /// On an incomplete message, details about why the message is incomplete.
    pub incomplete_details: Option<IncompleteDetails>,
    /// The Unix timestamp (in seconds) for when the message was completed.
    pub completed_at: u64,
    /// The Unix timestamp (in seconds) for when the message was marked as incomplete.
    pub incomplete_at: u64,
    /// The entity that produced the message. One of `user` or `assistant`.
    pub role: String,
    /// The content of the message in array of text and/or images.
    pub content: Content,
    /// If applicable, the ID of the [assistant](https://platform.openai.com/docs/api-reference/assistants) that authored this message.
    pub assistant_id: Option<String>,
    /// The ID of the [run](https://platform.openai.com/docs/api-reference/runs) associated with the creation of this message. Value is `null` when messages are created manually using the create message or create thread endpoints.
    pub run_id: Option<String>,
    /// A list of files attached to the message, and the tools they were added to.
    pub attachments: Option<Vec<Attachment>>,
    /// Set of 16 key-value pairs that can be attached to an object.
    /// This can be useful for storing additional information about the object in a structured format.
    /// Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    pub metadata: HashMap<String, serde_json::Value>,
}

/// The reason the message is incomplete.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct IncompleteDetails {
    pub reason: String,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "ListMessageRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ListMessageRequest {
    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    pub limit: Option<i32>, // Defaults to 20
    /// Sort order by the `created_at` timestamp of the objects. `asc` for ascending order and `desc` for descending order.
    pub order: Option<String>,
    /// A cursor for use in pagination. `after` is an object ID that defines your place in the list. For instance, if you make a list request and receive 100 objects, ending with obj_foo, your subsequent call can include after=obj_foo in order to fetch the next page of the list.
    pub after: Option<String>,
    /// A cursor for use in pagination. `before` is an object ID that defines your place in the list. For instance, if you make a list request and receive 100 objects, ending with obj_foo, your subsequent call can include before=obj_foo in order to fetch the previous page of the list.
    pub before: Option<String>,
    /// Filter messages by the run ID that generated them.
    pub run_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListMessageResponse {
    /// The identifier, which can be referenced in API endpoints.
    pub object: String,
    pub data: Vec<MessageResponse>,
    first_id: String,
    last_id: String,
    has_more: bool,
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "ModifyMessageRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ModifyMessageRequest {
    /// Set of 16 key-value pairs that can be attached to an object.
    /// This can be useful for storing additional information about the object in a structured format.
    /// Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteMessageResponse {
    pub id: String,
    pub object: String,
    pub delete: bool,
}
