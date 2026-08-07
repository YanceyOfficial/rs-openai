use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseInput {
    Text(String),
    Items(Vec<ResponseInputItem>),
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum ResponseInputRole {
    User,
    Assistant,
    System,
    Developer,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseInputItem {
    Message {
        role: ResponseInputRole,
        content: String,
    },
    FunctionCallOutput {
        call_id: String,
        output: String,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ResponseFunctionTool {
    #[builder(into)]
    pub name: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// A JSON Schema object describing the function's parameters.
    pub parameters: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub strict: Option<bool>,
}

/// A tool the model may call. Extensible union — this slice implements only `Function`;
/// later slices add `WebSearch`/`FileSearch`/`CodeInterpreter`/`ComputerUse`/
/// `ImageGeneration`/`Mcp` as new non-breaking variants.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ResponseTool {
    Function(ResponseFunctionTool),
}

/// The plain string modes `tool_choice` can take.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum ResponseToolChoiceMode {
    None,
    Auto,
    Required,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResponseToolChoiceFunctionName {
    pub name: String,
}

/// Controls which (if any) tool is called by the model. Either a plain mode string
/// (`"none"`/`"auto"`/`"required"`) or an object forcing one specific function. Mirrors
/// `crate::interfaces::chat::ToolChoice`'s shape exactly, but kept as a distinct type since
/// the two APIs document this parameter separately.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum ResponseToolChoice {
    Mode(ResponseToolChoiceMode),
    Specific {
        #[serde(rename = "type")]
        kind: String,
        function: ResponseToolChoiceFunctionName,
    },
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum TruncationStrategy {
    Auto,
    Disabled,
}
