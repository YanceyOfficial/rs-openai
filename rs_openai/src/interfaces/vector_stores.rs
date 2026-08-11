use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct StaticChunkingStrategy {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_chunk_size_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_overlap_tokens: Option<u32>,
}

/// How a file is split into searchable chunks. `Other` is response-only -- OpenAI echoes this
/// back for any strategy not created via `Auto`/`Static` through this API (e.g. dashboard-set).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChunkingStrategy {
    Auto,
    Static {
        #[serde(rename = "static")]
        config: StaticChunkingStrategy,
    },
    Other,
}

/// Distinct from `shared::types::ExpiresAfter` (Files/Uploads, `anchor: "created_at"` +
/// `seconds`) -- Vector Stores' expiration uses a different anchor value and a different
/// duration unit (`days`), so reusing that type would either be wrong or force it to accept
/// anchor/unit combinations no real endpoint supports.
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VectorStoreExpiresAfterAnchor {
    LastActiveAt,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct VectorStoreExpiresAfter {
    pub anchor: VectorStoreExpiresAfterAnchor,
    pub days: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VectorStoreStatus {
    Expired,
    InProgress,
    Completed,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreFileCounts {
    pub total: u32,
    pub completed: u32,
    pub in_progress: u32,
    pub failed: u32,
    pub cancelled: u32,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateVectorStoreRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunking_strategy: Option<ChunkingStrategy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<VectorStoreExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStore {
    pub id: String,
    pub object: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub status: VectorStoreStatus,
    pub created_at: i64,
    pub last_active_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub expires_after: Option<VectorStoreExpiresAfter>,
    pub usage_bytes: u64,
    pub file_counts: VectorStoreFileCounts,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct ListVectorStoresRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListVectorStoresResponse {
    pub object: String,
    pub data: Vec<VectorStore>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateVectorStoreRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<VectorStoreExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVectorStoreResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum VectorStoreSearchQuery {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct VectorStoreSearchRankingOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranker: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score_threshold: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct VectorStoreSearchRequest {
    pub query: VectorStoreSearchQuery,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub filters: Option<serde_json::Value>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_num_results: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub ranking_options: Option<VectorStoreSearchRankingOptions>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rewrite_query: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResultContent {
    pub r#type: String,
    pub text: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResult {
    pub file_id: String,
    pub filename: String,
    pub score: f32,
    pub attributes: Option<std::collections::HashMap<String, serde_json::Value>>,
    pub content: Vec<VectorStoreSearchResultContent>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VectorStoreSearchResponse {
    pub object: String,
    pub search_query: Vec<String>,
    pub data: Vec<VectorStoreSearchResult>,
    pub has_more: bool,
    pub next_page: Option<String>,
}
