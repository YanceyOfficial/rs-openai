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
