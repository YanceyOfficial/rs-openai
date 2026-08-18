//! Typed webhook event envelope -- see docs/superpowers/specs/2026-08-18-webhooks-design.md.

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct WebhookEvent {
    pub id: String,
    pub object: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub created_at: i64,
    pub data: serde_json::Value,
}
