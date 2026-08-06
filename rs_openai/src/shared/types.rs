use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Clone, Default)]
pub struct File {
    pub buffer: Vec<u8>,
    pub filename: String,
}

#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum Stop {
    String(String),
    ArrayOfString(Vec<String>),
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum ExpiresAfterAnchor {
    #[strum(serialize = "created_at")]
    CreatedAt,
}

/// When an uploaded resource should expire. `seconds` is valid from 3600 to 2,592,000 per
/// OpenAI's docs; not enforced client-side.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ExpiresAfter {
    pub anchor: ExpiresAfterAnchor,
    pub seconds: u32,
}

/// The confirmed, closed set of file purposes across Files and Uploads. Files accepts all 8
/// values; Uploads accepts only `Assistants`/`Batch`/`FineTune`/`Vision`, a subset — see each
/// module's own doc comments for the exact per-endpoint validity.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum FilePurpose {
    Assistants,
    AssistantsOutput,
    Batch,
    BatchOutput,
    #[strum(serialize = "fine-tune")]
    #[serde(rename = "fine-tune")]
    FineTune,
    #[strum(serialize = "fine-tune-results")]
    #[serde(rename = "fine-tune-results")]
    FineTuneResults,
    Vision,
    UserData,
}

/// Shared between legacy Completions and Chat — both endpoints document the identical shape.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct StreamOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_usage: Option<bool>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expires_after_serializes_with_snake_case_anchor() {
        let value = ExpiresAfter::builder()
            .anchor(ExpiresAfterAnchor::CreatedAt)
            .seconds(3600)
            .build();

        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"anchor": "created_at", "seconds": 3600})
        );
    }

    #[test]
    fn file_purpose_serializes_hyphenated_variants_correctly() {
        assert_eq!(
            serde_json::to_value(FilePurpose::FineTune).unwrap(),
            serde_json::json!("fine-tune")
        );
        assert_eq!(
            serde_json::to_value(FilePurpose::FineTuneResults).unwrap(),
            serde_json::json!("fine-tune-results")
        );
        assert_eq!(
            serde_json::to_value(FilePurpose::AssistantsOutput).unwrap(),
            serde_json::json!("assistants_output")
        );
    }

    /// strum's `Display` derive ignores serde's attributes entirely — without the
    /// container-level `#[strum(serialize_all = "snake_case")]`, every variant lacking an
    /// explicit `#[strum(serialize = "...")]` override falls back to the literal (PascalCase)
    /// Rust variant name instead of the snake_case wire value. This exhaustively covers the
    /// `.to_string()` (strum) path for all 8 variants, which is what `Files::upload` sends
    /// over the wire via a multipart text field — the JSON/serde path alone wouldn't have
    /// caught this regression.
    #[test]
    fn file_purpose_to_string_matches_wire_value_for_all_variants() {
        assert_eq!(FilePurpose::Assistants.to_string(), "assistants");
        assert_eq!(
            FilePurpose::AssistantsOutput.to_string(),
            "assistants_output"
        );
        assert_eq!(FilePurpose::Batch.to_string(), "batch");
        assert_eq!(FilePurpose::BatchOutput.to_string(), "batch_output");
        assert_eq!(FilePurpose::FineTune.to_string(), "fine-tune");
        assert_eq!(
            FilePurpose::FineTuneResults.to_string(),
            "fine-tune-results"
        );
        assert_eq!(FilePurpose::Vision.to_string(), "vision");
        assert_eq!(FilePurpose::UserData.to_string(), "user_data");
    }

    #[test]
    fn stream_options_omits_absent_include_usage() {
        let value = StreamOptions::builder().build();
        let json = serde_json::to_value(&value).unwrap();
        assert_eq!(json, serde_json::json!({}));
    }
}
