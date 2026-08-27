//! Skills (`/v1/skills`): a versioned, reusable bundle of files (instructions + scripts +
//! assets) anchored by a `SKILL.md` manifest -- see
//! docs/superpowers/specs/2026-08-27-skills-design.md.

use crate::shared::types::File;
use bon::Builder;
use serde::{Deserialize, Serialize};

/// Skill files to upload: either a single zip file, or multiple files forming a directory
/// bundle (each file's `filename` should be its full relative path within the skill's
/// top-level folder, e.g. `my-skill/SKILL.md`) -- confirmed via source
/// (`files: Union[SequenceNotStr[FileTypes], FileTypes]`) and the guide's curl examples.
/// Never actually serialized through serde in practice -- `apis::skills` builds a
/// `reqwest::multipart::Form` directly from this enum's variants -- but derives `Serialize`
/// (via `File`'s own real impl) for consistency with every other request type in this crate.
#[derive(Debug, Serialize, Clone)]
#[serde(untagged)]
pub enum SkillFiles {
    Zip(File),
    Directory(Vec<File>),
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateSkillRequest {
    pub files: SkillFiles,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Skill {
    pub id: String,
    pub created_at: i64,
    pub default_version: String,
    pub description: String,
    pub latest_version: String,
    pub name: String,
    pub object: String,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateSkillRequest {
    #[builder(into)]
    pub default_version: String,
}

#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct ListSkillsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListSkillsResponse {
    pub object: String,
    pub data: Vec<Skill>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeletedSkill {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateSkillVersionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<bool>,

    pub files: SkillFiles,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct SkillVersion {
    pub id: String,
    pub created_at: i64,
    pub description: String,
    pub name: String,
    pub object: String,
    pub skill_id: String,
    pub version: String,
}

#[derive(Builder, Debug, Serialize, Clone, Default)]
pub struct ListSkillVersionsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListSkillVersionsResponse {
    pub object: String,
    pub data: Vec<SkillVersion>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeletedSkillVersion {
    pub id: String,
    pub deleted: bool,
    pub object: String,
    pub version: String,
}
