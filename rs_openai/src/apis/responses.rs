//! Model responses — OpenAI's now-recommended primary API, superseding Chat Completions and
//! (eventually) Assistants. Core surface only this round: plain-text input/output and
//! function-calling, non-streaming — see
//! docs/superpowers/specs/2026-08-07-responses-core-design.md.

use crate::client::OpenAI;
#[allow(unused_imports)]
use crate::interfaces::responses;
pub use crate::interfaces::responses::*;

#[allow(dead_code)]
pub struct Responses<'a> {
    openai: &'a OpenAI,
}

impl<'a> Responses<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}

#[allow(dead_code)]
pub struct Response<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Response<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }
}

#[cfg(test)]
mod tests {
    use crate::interfaces::responses::{
        ResponseFunctionTool, ResponseInputItem, ResponseInputRole, ResponseTool,
        ResponseToolChoice, ResponseToolChoiceFunctionName, ResponseToolChoiceMode,
    };

    #[test]
    fn response_input_item_message_serializes_with_type_tag() {
        let item = ResponseInputItem::Message {
            role: ResponseInputRole::User,
            content: "hello".to_string(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "message", "role": "user", "content": "hello"})
        );
    }

    #[test]
    fn response_input_item_function_call_output_serializes_with_type_tag() {
        let item = ResponseInputItem::FunctionCallOutput {
            call_id: "call_123".to_string(),
            output: "72F and sunny".to_string(),
        };

        let json = serde_json::to_value(&item).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function_call_output",
                "call_id": "call_123",
                "output": "72F and sunny"
            })
        );
    }

    #[test]
    fn response_tool_function_serializes_with_type_tag() {
        let tool = ResponseTool::Function(
            ResponseFunctionTool::builder()
                .name("get_weather")
                .parameters(serde_json::json!({"type": "object"}))
                .build(),
        );

        let json = serde_json::to_value(&tool).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "function",
                "name": "get_weather",
                "parameters": {"type": "object"}
            })
        );
    }

    #[test]
    fn response_tool_choice_serializes_mode_as_bare_string() {
        let choice = ResponseToolChoice::Mode(ResponseToolChoiceMode::Auto);
        assert_eq!(
            serde_json::to_value(&choice).unwrap(),
            serde_json::json!("auto")
        );
    }

    #[test]
    fn response_tool_choice_serializes_specific_function_as_object() {
        let choice = ResponseToolChoice::Specific {
            kind: "function".to_string(),
            function: ResponseToolChoiceFunctionName {
                name: "get_weather".to_string(),
            },
        };

        let json = serde_json::to_value(&choice).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"type": "function", "function": {"name": "get_weather"}})
        );
    }
}
