//! Sandboxed containers backing the Responses API's `code_interpreter` tool, and their
//! files — see docs/superpowers/specs/2026-08-14-containers-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ContainerExpiresAfterAnchor {
    LastActiveAt,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ContainerExpiresAfter {
    pub anchor: ContainerExpiresAfterAnchor,
    pub minutes: u32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum ContainerMemoryLimit {
    #[serde(rename = "1g")]
    OneGb,
    #[serde(rename = "4g")]
    FourGb,
    #[serde(rename = "16g")]
    SixteenGb,
    #[serde(rename = "64g")]
    SixtyFourGb,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ContainerDomainSecret {
    #[builder(into)]
    pub domain: String,

    #[builder(into)]
    pub name: String,

    #[builder(into)]
    pub value: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContainerNetworkPolicy {
    Disabled,
    Allowlist {
        allowed_domains: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        domain_secrets: Option<Vec<ContainerDomainSecret>>,
    },
}

/// Base64-encoded inline skill payload. `media_type` is always `"application/zip"` per
/// source -- modeled as a plain settable `String` rather than hardcoded, matching this
/// crate's convention for currently-single-valued-but-technically-open fields (e.g.
/// `SpendAlert`'s `currency`/`interval`).
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct InlineSkillSource {
    #[builder(into)]
    pub data: String,

    #[builder(into)]
    pub media_type: String,
}

/// A skill attached to a container, either by reference to an existing `/v1/skills` skill
/// or provided inline -- confirmed via source (`Skill: TypeAlias = Union[SkillReferenceParam,
/// InlineSkillParam]`).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContainerSkill {
    SkillReference {
        skill_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        version: Option<String>,
    },
    Inline {
        description: String,
        name: String,
        source: InlineSkillSource,
    },
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateContainerRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<ContainerExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_ids: Option<Vec<String>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_limit: Option<ContainerMemoryLimit>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub network_policy: Option<ContainerNetworkPolicy>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub skills: Option<Vec<ContainerSkill>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Container {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub status: String,
    pub expires_after: Option<ContainerExpiresAfter>,
    pub last_active_at: Option<i64>,
    pub memory_limit: Option<ContainerMemoryLimit>,
    pub network_policy: Option<ContainerNetworkPolicy>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListContainersRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListContainersResponse {
    pub object: String,
    pub data: Vec<Container>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteContainerResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

/// Either an already-uploaded file referenced by ID, or raw file bytes to upload directly --
/// confirmed via source (`file: FileTypes | Omit`, `file_id: str | Omit`, mutually
/// exclusive; the SDK sends a JSON body for `file_id` and a multipart body for `file`).
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum ContainerFileSource {
    FileId(String),
    Upload(crate::shared::types::File),
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateContainerFileRequest {
    pub source: ContainerFileSource,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ContainerFile {
    pub id: String,
    pub bytes: u64,
    pub container_id: String,
    pub created_at: i64,
    pub object: String,
    pub path: String,
    pub source: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListContainerFilesRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListContainerFilesResponse {
    pub object: String,
    pub data: Vec<ContainerFile>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteContainerFileResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
