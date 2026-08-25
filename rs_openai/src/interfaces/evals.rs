//! Evaluation definitions for grading model outputs against a defined criteria set, plus
//! their Runs and Runs' Output Items sub-resources -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md and
//! docs/superpowers/specs/2026-08-20-evals-runs-design.md.

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

    /// Server-generated on the response; a caller never sets this on create, so it's always
    /// omitted from the outbound request (`skip_serializing_if`) and only ever populated when
    /// deserializing an `Eval`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
}

/// Deprecated by OpenAI in favor of `EvalLogsDataSourceConfig`; still a live wire type.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalStoredCompletionsDataSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,

    /// Server-generated on the response; see `EvalLogsDataSourceConfig.schema`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<serde_json::Value>,
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

// Runs sub-resource, appended here (matching this crate's established precedent of
// colocating a sub-resource's types in the parent's flat interfaces file when the
// parent module isn't already split into a directory) -- see
// docs/superpowers/specs/2026-08-20-evals-runs-design.md.

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateEvalRunRequest {
    pub data_source: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalApiError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunPerModelUsage {
    pub cached_tokens: i64,
    pub completion_tokens: i64,
    pub invocation_count: i64,
    #[serde(rename = "model_name")]
    pub run_model_name: String,
    pub prompt_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunPerTestingCriteriaResult {
    pub failed: i64,
    pub passed: i64,
    pub testing_criteria: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunResultCounts {
    pub errored: i64,
    pub failed: i64,
    pub passed: i64,
    pub total: i64,
}

/// Used uniformly for create/retrieve/list/cancel -- confirmed byte-identical across
/// all four operations' response types in the source.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRun {
    pub id: String,
    pub created_at: i64,
    pub data_source: serde_json::Value,
    pub error: EvalApiError,
    pub eval_id: String,
    pub metadata: Option<serde_json::Value>,
    pub model: String,
    pub name: String,
    pub object: String,
    pub per_model_usage: Vec<EvalRunPerModelUsage>,
    pub per_testing_criteria_results: Vec<EvalRunPerTestingCriteriaResult>,
    pub report_url: String,
    pub result_counts: EvalRunResultCounts,
    pub status: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListEvalRunsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListEvalRunsResponse {
    pub object: String,
    pub data: Vec<EvalRun>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

/// Confirmed asymmetry vs. every other delete-response type in this crate: the ID
/// field is named `run_id`, not `id`.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteEvalRunResponse {
    pub deleted: Option<bool>,
    pub object: Option<String>,
    pub run_id: Option<String>,
}

// Output Items sub-resource (nested under Runs), appended here too.

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItemResult {
    pub name: String,
    pub passed: bool,
    pub score: f64,
    pub sample: Option<serde_json::Value>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItemSampleInput {
    pub content: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItemSampleOutput {
    pub content: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItemSampleUsage {
    pub cached_tokens: i64,
    pub completion_tokens: i64,
    pub prompt_tokens: i64,
    pub total_tokens: i64,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItemSample {
    pub error: EvalApiError,
    pub finish_reason: String,
    pub input: Vec<EvalRunOutputItemSampleInput>,
    pub max_completion_tokens: i64,
    pub model: String,
    pub output: Vec<EvalRunOutputItemSampleOutput>,
    pub seed: i64,
    pub temperature: f64,
    pub top_p: f64,
    pub usage: EvalRunOutputItemSampleUsage,
}

/// Used uniformly for retrieve/list -- confirmed identical shape.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct EvalRunOutputItem {
    pub id: String,
    pub created_at: i64,
    pub datasource_item: serde_json::Value,
    pub datasource_item_id: i64,
    pub eval_id: String,
    pub object: String,
    pub results: Vec<EvalRunOutputItemResult>,
    pub run_id: String,
    pub sample: EvalRunOutputItemSample,
    pub status: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListEvalRunOutputItemsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListEvalRunOutputItemsResponse {
    pub object: String,
    pub data: Vec<EvalRunOutputItem>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
