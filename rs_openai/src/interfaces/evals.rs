//! Evaluation definitions for grading model outputs against a defined criteria set. Runs are
//! not yet implemented (their request/response shape was not confirmed by research) -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

/// Request-side only. The `custom` eval's response shape genuinely differs -- see
/// `EvalCustomDataSourceConfigResponse`.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalCustomDataSourceConfig {
    pub item_schema: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_sample_schema: Option<bool>,
}

/// Response-side only. OpenAI echoes back a server-generated `schema` for a `custom` eval,
/// not the `item_schema`/`include_sample_schema` fields sent on create -- confirmed via live
/// docs. Do not merge this back into `EvalCustomDataSourceConfig`; the two directions have
/// genuinely different wire shapes.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalCustomDataSourceConfigResponse {
    pub schema: serde_json::Value,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalLogsDataSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Deprecated by OpenAI in favor of `EvalLogsDataSourceConfig`; still a live wire type.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalStoredCompletionsDataSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Request-side only (`CreateEvalRequest.data_source_config`) -- see `EvalDataSourceConfigResponse`
/// for the response-side union, which differs only in the `Custom` variant's payload type.
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvalDataSourceConfig {
    Custom(EvalCustomDataSourceConfig),
    Logs(EvalLogsDataSourceConfig),
    StoredCompletions(EvalStoredCompletionsDataSourceConfig),
}

/// Response-side only (`Eval.data_source_config`). `Logs`/`StoredCompletions` echo back
/// unchanged, but `Custom` carries a server-generated `schema` instead of the request's
/// `item_schema`/`include_sample_schema` -- see `EvalCustomDataSourceConfigResponse`.
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvalDataSourceConfigResponse {
    Custom(EvalCustomDataSourceConfigResponse),
    Logs(EvalLogsDataSourceConfig),
    StoredCompletions(EvalStoredCompletionsDataSourceConfig),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EvalStringCheckOperation {
    Eq,
    Ne,
    Like,
    Ilike,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalStringCheckGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub input: String,
    #[builder(into)]
    pub reference: String,
    pub operation: EvalStringCheckOperation,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EvalTextSimilarityMetric {
    #[serde(rename = "cosine")]
    Cosine,
    #[serde(rename = "fuzzy_match")]
    FuzzyMatch,
    #[serde(rename = "bleu")]
    Bleu,
    #[serde(rename = "gleu")]
    Gleu,
    #[serde(rename = "meteor")]
    Meteor,
    #[serde(rename = "rouge_1")]
    Rouge1,
    #[serde(rename = "rouge_2")]
    Rouge2,
    #[serde(rename = "rouge_3")]
    Rouge3,
    #[serde(rename = "rouge_4")]
    Rouge4,
    #[serde(rename = "rouge_5")]
    Rouge5,
    #[serde(rename = "rouge_l")]
    RougeL,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalTextSimilarityGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub input: String,
    #[builder(into)]
    pub reference: String,
    pub evaluation_metric: EvalTextSimilarityMetric,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalPythonGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub source: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tag: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalGraderMessage {
    pub content: serde_json::Value,
    #[builder(into)]
    pub role: String,
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalScoreModelSamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completions_tokens: Option<u32>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalScoreModelGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub model: String,
    pub input: Vec<EvalGraderMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Vec<f32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_params: Option<EvalScoreModelSamplingParams>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalLabelModelGrader {
    #[builder(into)]
    pub name: String,
    pub input: Vec<EvalGraderMessage>,
    pub labels: Vec<String>,
    #[builder(into)]
    pub model: String,
    pub passing_labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvalGrader {
    StringCheck(EvalStringCheckGrader),
    TextSimilarity(EvalTextSimilarityGrader),
    Python(EvalPythonGrader),
    ScoreModel(EvalScoreModelGrader),
    LabelModel(EvalLabelModelGrader),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateEvalRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    pub data_source_config: EvalDataSourceConfig,
    pub testing_criteria: Vec<EvalGrader>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Eval {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub data_source_config: EvalDataSourceConfigResponse,
    pub testing_criteria: Vec<EvalGrader>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct UpdateEvalRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListEvalsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListEvalsResponse {
    pub object: String,
    pub data: Vec<Eval>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteEvalResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
