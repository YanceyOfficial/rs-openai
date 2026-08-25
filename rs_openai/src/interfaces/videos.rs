//! Sora video generation. Core CRUD + content download only -- input_reference, characters,
//! extend, remix, and edit are deliberately unimplemented, since the whole API sunsets
//! **2026-09-24** (see `apis::videos`'s module doc for the confirming source) -- see
//! docs/superpowers/specs/2026-08-18-videos-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum VideoSize {
    #[serde(rename = "1280x720")]
    S1280x720,
    #[serde(rename = "1920x1080")]
    S1920x1080,
    #[serde(rename = "1080x1920")]
    S1080x1920,
    #[serde(rename = "720x480")]
    S720x480,
    #[serde(rename = "480x720")]
    S480x720,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateVideoRequest {
    #[builder(into)]
    pub model: String,

    #[builder(into)]
    pub prompt: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<VideoSize>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seconds: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct VideoError {
    pub message: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Video {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub model: String,
    pub progress: Option<u32>,
    pub seconds: Option<String>,
    pub size: Option<VideoSize>,
    pub error: Option<VideoError>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListVideosRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListVideosResponse {
    pub object: String,
    pub data: Vec<Video>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteVideoResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum VideoContentVariant {
    Thumbnail,
    Spritesheet,
}
