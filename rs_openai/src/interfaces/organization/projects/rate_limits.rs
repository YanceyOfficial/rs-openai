//! A project's rate limits (`/organization/projects/{project_id}/rate_limits`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectRateLimit {
    pub id: String,
    pub max_requests_per_1_minute: i64,
    pub max_tokens_per_1_minute: i64,
    pub model: String,
    pub object: String,
    pub batch_1_day_max_input_tokens: Option<i64>,
    pub max_audio_megabytes_per_1_minute: Option<i64>,
    pub max_images_per_1_minute: Option<i64>,
    pub max_requests_per_1_day: Option<i64>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectRateLimitsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectRateLimitsResponse {
    pub object: String,
    pub data: Vec<ProjectRateLimit>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectRateLimitRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_1_day_max_input_tokens: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_audio_megabytes_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_images_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_day: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_1_minute: Option<i64>,
}
