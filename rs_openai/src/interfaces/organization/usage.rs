//! Organization usage reporting (`/organization/usage/*`, `/organization/costs`) -- see
//! docs/superpowers/specs/2026-08-20-org-usage-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "object")]
pub enum UsageResult {
    #[serde(rename = "organization.usage.completions.result")]
    Completions(UsageCompletionsResult),
    #[serde(rename = "organization.usage.embeddings.result")]
    Embeddings(UsageEmbeddingsResult),
    #[serde(rename = "organization.usage.moderations.result")]
    Moderations(UsageModerationsResult),
    #[serde(rename = "organization.usage.images.result")]
    Images(UsageImagesResult),
    #[serde(rename = "organization.usage.audio_speeches.result")]
    AudioSpeeches(UsageAudioSpeechesResult),
    #[serde(rename = "organization.usage.audio_transcriptions.result")]
    AudioTranscriptions(UsageAudioTranscriptionsResult),
    #[serde(rename = "organization.usage.vector_stores.result")]
    VectorStores(UsageVectorStoresResult),
    #[serde(rename = "organization.usage.code_interpreter_sessions.result")]
    CodeInterpreterSessions(UsageCodeInterpreterSessionsResult),
    #[serde(rename = "organization.usage.file_searches.result")]
    FileSearches(UsageFileSearchesResult),
    #[serde(rename = "organization.usage.web_searches.result")]
    WebSearches(UsageWebSearchesResult),
    #[serde(rename = "organization.costs.result")]
    Costs(UsageCostsResult),
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageBucket {
    pub start_time: i64,
    pub end_time: i64,
    pub object: String,
    pub results: Vec<UsageResult>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageResponse {
    pub data: Vec<UsageBucket>,
    pub has_more: bool,
    pub next_page: Option<String>,
    pub object: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCompletionsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub output_tokens: i64,
    pub api_key_id: Option<String>,
    pub batch: Option<bool>,
    pub input_audio_tokens: Option<i64>,
    pub input_cache_write_tokens: Option<i64>,
    pub input_cached_audio_tokens: Option<i64>,
    pub input_cached_image_tokens: Option<i64>,
    pub input_cached_text_tokens: Option<i64>,
    pub input_cached_tokens: Option<i64>,
    pub input_image_tokens: Option<i64>,
    pub input_text_tokens: Option<i64>,
    pub input_uncached_tokens: Option<i64>,
    pub model: Option<String>,
    pub output_audio_tokens: Option<i64>,
    pub output_image_tokens: Option<i64>,
    pub output_text_tokens: Option<i64>,
    pub project_id: Option<String>,
    pub service_tier: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageEmbeddingsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageModerationsResult {
    pub input_tokens: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageImagesResult {
    pub images: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub size: Option<String>,
    pub source: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageAudioSpeechesResult {
    pub characters: i64,
    pub num_model_requests: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageAudioTranscriptionsResult {
    pub num_model_requests: i64,
    pub seconds: i64,
    pub api_key_id: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageVectorStoresResult {
    pub usage_bytes: i64,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCodeInterpreterSessionsResult {
    pub num_sessions: i64,
    pub project_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageFileSearchesResult {
    pub num_requests: i64,
    pub api_key_id: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
    pub vector_store_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageWebSearchesResult {
    pub num_model_requests: i64,
    pub num_requests: i64,
    pub api_key_id: Option<String>,
    pub context_level: Option<String>,
    pub model: Option<String>,
    pub project_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCostsAmount {
    pub currency: Option<String>,
    pub value: Option<f64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UsageCostsResult {
    pub amount: Option<UsageCostsAmount>,
    pub api_key_id: Option<String>,
    pub line_item: Option<String>,
    pub project_id: Option<String>,
    pub quantity: Option<f64>,
}

/// Query params for `GET /organization/usage/completions`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`, `batch`,
/// `service_tier`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageCompletionsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub batch: Option<bool>,
    pub group_by: Option<Vec<String>>,
}

impl UsageCompletionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(v) = self.batch {
            pairs.push(("batch".to_string(), v.to_string()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/embeddings`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageEmbeddingsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageEmbeddingsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/moderations`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageModerationsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageModerationsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/audio_speeches`. `group_by` accepts any
/// combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageAudioSpeechesRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageAudioSpeechesRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}

/// Query params for `GET /organization/usage/audio_transcriptions`. `group_by` accepts
/// any combination of `project_id`, `user_id`, `api_key_id`, `model`.
#[derive(Builder, Debug, Clone, Default)]
pub struct UsageAudioTranscriptionsRequest {
    pub start_time: i64,
    pub end_time: Option<i64>,
    #[builder(into)]
    pub bucket_width: Option<String>,
    pub limit: Option<u32>,
    #[builder(into)]
    pub page: Option<String>,
    pub project_ids: Option<Vec<String>>,
    pub api_key_ids: Option<Vec<String>>,
    pub user_ids: Option<Vec<String>>,
    pub models: Option<Vec<String>>,
    pub group_by: Option<Vec<String>>,
}

impl UsageAudioTranscriptionsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        pairs.push(("start_time".to_string(), self.start_time.to_string()));
        if let Some(v) = self.end_time {
            pairs.push(("end_time".to_string(), v.to_string()));
        }
        if let Some(v) = &self.bucket_width {
            pairs.push(("bucket_width".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.page {
            pairs.push(("page".to_string(), v.clone()));
        }
        if let Some(values) = &self.project_ids {
            for v in values {
                pairs.push(("project_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.api_key_ids {
            for v in values {
                pairs.push(("api_key_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.user_ids {
            for v in values {
                pairs.push(("user_ids".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.models {
            for v in values {
                pairs.push(("models".to_string(), v.clone()));
            }
        }
        if let Some(values) = &self.group_by {
            for v in values {
                pairs.push(("group_by".to_string(), v.clone()));
            }
        }
        pairs
    }
}
