use std::collections::HashMap;

use crate::shared::response_wrapper::OpenAIError;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "CreateAssistantRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct AssistantRequest {
    /// ID of the model to use. You can use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models,
    /// or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    pub model: String,

    /// The name of the assistant. The maximum length is 256 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// The description of the assistant. The maximum length is 512 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The system instructions that the assistant uses. The maximum length is 256,000 characters.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>,

    /// A list of tool enabled on the assistant. There can be a maximum of 128 tools per assistant.
    /// Tools can be of types `code_interpreter`, `file_search`, or `function`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tools>>,

    /// A set of resources that are used by the assistant's tools. The resources are specific to the type of tool.
    /// For example, the code_interpreter tool requires a list of file IDs, while the `file_search` tool requires a list of vector store IDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_resources: Option<ToolResources>,

    /// Set of 16 key-value pairs that can be attached to an object.
    /// This can be useful for storing additional information about the object in a structured format.
    /// Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,

    /// What sampling temperature to use, between 0 and 2.
    /// Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>, // Defaults to 1

    /// An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass.
    /// So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    /// We generally recommend altering this or temperature but not both.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>, // Defaults to 1

    /// Specifies the format that the model must output.
    /// Compatible with [GPT-4o](https://platform.openai.com/docs/models/gpt-4o), [GPT-4 Turbo](https://platform.openai.com/docs/models/gpt-4-turbo-and-gpt-4), and all GPT-3.5 Turbo models since `gpt-3.5-turbo-1106`.
    ///
    /// Setting to `{ "type": "json_schema", "json_schema": {...} }`
    /// enables Structured Outputs which ensures the model will match your supplied JSON schema.
    /// Learn more in the [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
    ///
    /// Setting to `{ "type": "json_object" }` enables JSON mode, which ensures the message the model generates is valid JSON.
    ///
    /// **Important:** when using JSON mode, you must also instruct the model to produce JSON yourself via a system or user message.
    /// Without this, the model may generate an unending stream of whitespace until the generation reaches the token limit, resulting in a long-running and seemingly "stuck" request.
    /// Also note that the message content may be partially cut off if `finish_reason="length"`,
    /// which indicates the generation exceeded `max_tokens` or the conversation exceeded the max context length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<ResponseType>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum Tools {
    CodeInterpreterTool(CodeInterpreterTool),
    FileSearchTool(FileSearchTool),
    FunctionTool(FunctionTool),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CodeInterpreterTool {
    /// The type of tool being defined: `code_interpreter`
    pub r#type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileSearchTool {
    /// The type of tool being defined: `file_search`
    pub r#type: String,

    /// Overrides for the file search tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_search: Option<FileSearch>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileSearch {
    /// The maximum number of results the file search tool should output.
    /// The default is 20 for `gpt-4*` models and 5 for `gpt-3.5-turbo`. This number should be between 1 and 50 inclusive.
    ///
    /// Note that the file search tool may output fewer than `max_num_results` results. See the [file search tool documentation](https://platform.openai.com/docs/assistants/tools/file-search/customizing-file-search-settings) for more information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<i32>,

    /// The ranking options for the file search. If not specified, the file search tool will use the auto ranker and a score_threshold of 0.
    /// See the [file search tool documentation](https://platform.openai.com/docs/assistants/tools/file-search/customizing-file-search-settings) for more information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<RankingOptions>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RankingOptions {
    /// The ranker to use for the file search. If not specified will use the `auto` ranker.
    #[serde(skip_serializing_if = "Option::is_none")]
    ranker: Option<String>,

    /// The score threshold for the file search. All values must be a floating point number between 0 and 1.
    score_threshold: i32,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FunctionTool {
    /// The type of tool being defined: `function`
    pub r#type: String,

    pub function: Function,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Function {
    /// A description of what the function does, used by the model to choose when and how to call the function.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The name of the function to be called. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 64.
    pub name: String,

    /// The parameters the functions accepts, described as a JSON Schema object. See the `guide` for examples, and the `JSON Schema reference` for documentation about the format.
    ///
    /// Omitting `parameters` defines a function with an empty parameter list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<serde_json::Value>,

    /// Whether to enable strict schema adherence when generating the function call.
    /// If set to true, the model will follow the exact schema defined in the `parameters` field.
    /// Only a subset of JSON Schema is supported when `strict` is `true`. Learn more about Structured Outputs in the [function calling guide](https://platform.openai.com/docs/api-reference/assistants/docs/guides/function-calling).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>, // Defaults to false
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum ToolResources {
    CodeInterpreterToolResources(CodeInterpreterToolResources),
    FileSearchToolResources(FileSearchToolResources),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CodeInterpreterToolResources {
    /// A list of file IDs made available to the `code_interpreter` tool. There can be a maximum of 20 files associated with the tool.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>, // Defaults to []
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct FileSearchToolResources {
    /// The [vector store](https://platform.openai.com/docs/api-reference/vector-stores/object) attached to this assistant.
    /// There can be a maximum of 1 vector store attached to the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_store_ids: Option<Vec<String>>,

    /// A helper to create a [vector store](https://platform.openai.com/docs/api-reference/vector-stores/object) with file_ids and attach it to this assistant.
    /// There can be a maximum of 1 vector store attached to the assistant.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector_stores: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStores {
    /// A list of [file](https://platform.openai.com/docs/api-reference/files) IDs to add to the vector store. There can be a maximum of 10000 files in a vector store.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    /// The chunking strategy used to chunk the file(s). If not set, will use the `auto` strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<ChunkingStrategy>,

    /// Set of 16 key-value pairs that can be attached to a vector store.
    /// This can be useful for storing additional information about the vector store in a structured format.
    /// Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum ChunkingStrategy {
    AutoChunkingStrategy(AutoChunkingStrategy),
    StaticChunkingStrategy(StaticChunkingStrategy),
}

/// The default strategy. This strategy currently uses a `max_chunk_size_tokens` of 800 and `chunk_overlap_tokens` of 400.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AutoChunkingStrategy {
    /// Always `auto`.
    pub r#type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct StaticChunkingStrategy {
    /// Always `static`.
    pub r#type: String,

    pub r#static: Static,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Static {
    /// The maximum number of tokens in each chunk. The default value is 800.
    /// The minimum value is 100 and the maximum value is 4096.
    pub max_chunk_size_tokens: i32,

    /// The number of tokens that overlap between chunks. The default value is `400`.
    /// Note that the overlap must not exceed half of `max_chunk_size_tokens`.
    pub chunk_overlap_tokens: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum ResponseType {
    /// `auto` is the default value
    #[strum(serialize = "auto")]
    AutoType(String),
    TextType(TextType),
    JsonObjectType(JsonObjectType),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct TextType {
    /// The type of response format being defined: `text`
    pub r#type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct JsonObjectType {
    /// The type of response format being defined: `json_object`
    pub r#type: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct JsonSchemaType {
    /// The type of response format being defined: `json_schema`
    pub r#type: String,
    pub json_schema: JsonSchema,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct JsonSchema {
    /// A description of what the response format is for, used by the model to determine how to respond in the format.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// The name of the response format. Must be a-z, A-Z, 0-9, or contain underscores and dashes, with a maximum length of 64.
    pub name: String,

    /// The schema for the response format, described as a JSON Schema object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,

    /// Whether to enable strict schema adherence when generating the output.
    /// If set to true, the model will always follow the exact schema defined in the `schema` field.
    /// Only a subset of JSON Schema is supported when `strict` is `true`.
    /// To learn more, read the [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>, // Defaults to false
}

/// Represents an `assistant` that can call the model and use tools.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AssistantResponse {
    /// The identifier, which can be referenced in API endpoints.
    id: String,
    /// The object type, which is always `assistant`.
    object: String,
    /// The Unix timestamp (in seconds) for when the assistant was created.
    created_at: u64,
    /// The name of the assistant. The maximum length is 256 characters.
    name: Option<String>,
    /// The description of the assistant. The maximum length is 512 characters.
    description: Option<String>,
    /// ID of the model to use. You can use the [List models](https://platform.openai.com/docs/api-reference/models/list) API to see all of your available models,
    /// or see our [Model overview](https://platform.openai.com/docs/models/overview) for descriptions of them.
    model: String,
    /// The system instructions that the assistant uses. The maximum length is 256,000 characters.
    instructions: Option<String>,
    /// A list of tool enabled on the assistant. There can be a maximum of 128 tools per assistant. Tools can be of types `code_interpreter`, `file_search`, or `function`.
    tools: Vec<Tools>,
    // A set of resources that are used by the assistant's tools. The resources are specific to the type of tool. For example, the `code_interpreter` tool requires a list of file IDs, while the `file_search` tool requires a list of vector store IDs.
    tool_resources: Option<ToolResources>,
    /// Set of 16 key-value pairs that can be attached to an object. This can be useful for storing additional information about the object in a structured format. Keys can be a maximum of 64 characters long and values can be a maximum of 512 characters long.
    metadata: HashMap<String, serde_json::Value>,
    /// What sampling temperature to use, between 0 and 2. Higher values like 0.8 will make the output more random, while lower values like 0.2 will make it more focused and deterministic.
    temperature: f64,
    /// An alternative to sampling with temperature, called nucleus sampling, where the model considers the results of the tokens with top_p probability mass. So 0.1 means only the tokens comprising the top 10% probability mass are considered.
    ///
    /// We generally recommend altering this or temperature but not both.
    top_p: f64,
    /// Specifies the format that the model must output.
    /// Compatible with [GPT-4o](https://platform.openai.com/docs/models/gpt-4o), [GPT-4 Turbo](https://platform.openai.com/docs/models/gpt-4-turbo-and-gpt-4), and all GPT-3.5 Turbo models since `gpt-3.5-turbo-1106`.
    ///
    /// Setting to `{ "type": "json_schema", "json_schema": {...} }`
    /// enables Structured Outputs which ensures the model will match your supplied JSON schema.
    /// Learn more in the [Structured Outputs guide](https://platform.openai.com/docs/guides/structured-outputs).
    ///
    /// Setting to `{ "type": "json_object" }` enables JSON mode, which ensures the message the model generates is valid JSON.
    ///
    /// **Important:** when using JSON mode, you must also instruct the model to produce JSON yourself via a system or user message.
    /// Without this, the model may generate an unending stream of whitespace until the generation reaches the token limit, resulting in a long-running and seemingly "stuck" request.
    /// Also note that the message content may be partially cut off if `finish_reason="length"`,
    /// which indicates the generation exceeded `max_tokens` or the conversation exceeded the max context length.
    response_format: ResponseType, // "auto" or object
}

#[derive(Builder, Clone, Debug, Default, Serialize)]
#[builder(name = "ListAssistantRequestBuilder")]
#[builder(pattern = "mutable")]
#[builder(setter(into, strip_option), default)]
#[builder(derive(Debug))]
#[builder(build_fn(error = "OpenAIError"))]
pub struct ListAssistantRequest {
    /// A limit on the number of objects to be returned. Limit can range between 1 and 100, and the default is 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<String>, // Defaults to 20

    /// Sort order by the `created_at` timestamp of the objects. `asc` for ascending order and `desc` for descending order.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<String>, // Defaults to desc

    /// A cursor for use in pagination. `after` is an object ID that defines your place in the list.
    /// For instance, if you make a list request and receive 100 objects, ending with obj_foo,
    /// your subsequent call can include after=obj_foo in order to fetch the next page of the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    /// A cursor for use in pagination. `before` is an object ID that defines your place in the list.
    /// For instance, if you make a list request and receive 100 objects, ending with obj_foo,
    /// your subsequent call can include before=obj_foo in order to fetch the previous page of the list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListAssistantResponse {
    object: String,
    data: Vec<AssistantResponse>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteAssistantResponse {
    id: String,
    object: String,
    delete: bool,
}
