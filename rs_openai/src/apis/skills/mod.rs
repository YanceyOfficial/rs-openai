//! Skills (`/v1/skills`) -- see docs/superpowers/specs/2026-08-27-skills-design.md.

pub mod versions;

use crate::client::OpenAI;
use crate::interfaces::skills;
pub use crate::interfaces::skills::*;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::{Form, Part};

/// Builds a multipart form for a `SkillFiles` value, matching `Images::create_edit`'s
/// already-proven repeated-part pattern: a single `files` part for a zip upload, or
/// repeated `files[]` parts (one per file, each carrying its own relative path as its
/// filename) for a directory upload.
fn skill_files_form(files: &skills::SkillFiles) -> Form {
    match files {
        skills::SkillFiles::Zip(file) => {
            let part = Part::stream(file.buffer.clone())
                .file_name(file.filename.clone())
                .mime_str("application/zip")
                .expect("\"application/zip\" is always a valid mime type");
            Form::new().part("files", part)
        }
        skills::SkillFiles::Directory(files) => {
            let mut form = Form::new();
            for file in files {
                let part = Part::stream(file.buffer.clone())
                    .file_name(file.filename.clone())
                    .mime_str("application/octet-stream")
                    .expect("\"application/octet-stream\" is always a valid mime type");
                form = form.part("files[]", part);
            }
            form
        }
    }
}

pub struct Skills<'a> {
    openai: &'a OpenAI,
}

impl<'a> Skills<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a new skill.
    pub async fn create(&self, req: &skills::CreateSkillRequest) -> OpenAIResponse<skills::Skill> {
        self.openai
            .post_form("/skills", skill_files_form(&req.files))
            .await
    }

    /// Lists all skills for the current project.
    pub async fn list(
        &self,
        req: &skills::ListSkillsRequest,
    ) -> OpenAIResponse<skills::ListSkillsResponse> {
        self.openai.get("/skills", req).await
    }
}

pub struct Skill<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Skill<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Gets a skill by its ID.
    pub async fn retrieve(&self) -> OpenAIResponse<skills::Skill> {
        self.openai.get(&format!("/skills/{}", self.id), &()).await
    }

    /// Updates the default version pointer for a skill.
    pub async fn update(&self, req: &skills::UpdateSkillRequest) -> OpenAIResponse<skills::Skill> {
        self.openai.post(&format!("/skills/{}", self.id), req).await
    }

    /// Deletes a skill by its ID.
    pub async fn delete(&self) -> OpenAIResponse<skills::DeletedSkill> {
        self.openai
            .delete(&format!("/skills/{}", self.id), &())
            .await
    }

    /// Downloads a skill zip bundle by its ID.
    pub async fn content(&self) -> OpenAIResponse<Vec<u8>> {
        self.openai
            .get_raw_bytes(&format!("/skills/{}/content", self.id))
            .await
    }

    /// Scoped accessor for this skill's versions.
    pub fn versions(&self) -> versions::SkillVersions<'a> {
        versions::SkillVersions::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific skill version.
    pub fn version(&self, version: impl Into<String>) -> versions::SkillVersion<'a> {
        versions::SkillVersion::new(self.openai, self.id.clone(), version.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::skills::{CreateSkillRequest, ListSkillsRequest, SkillFiles};
    use crate::shared::types::File;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }

    fn sample_skill_json() -> serde_json::Value {
        serde_json::json!({
            "id": "skill_1",
            "created_at": 1700000000_i64,
            "default_version": "1",
            "description": "Formats spreadsheets",
            "latest_version": "1",
            "name": "spreadsheet-formatter",
            "object": "skill"
        })
    }

    #[tokio::test]
    async fn create_sends_zip_as_single_files_part() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/skills"))
            .and(ContainsInBody(br#"name="files""#))
            .and(ContainsInBody(&[1, 2, 3]))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateSkillRequest::builder()
            .files(SkillFiles::Zip(File {
                buffer: vec![1, 2, 3],
                filename: "skill.zip".to_string(),
            }))
            .build();

        let res = client.skills().create(&req).await.unwrap();
        assert_eq!(res.id, "skill_1");
    }

    #[tokio::test]
    async fn create_sends_directory_as_repeated_files_bracket_parts() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/skills"))
            .and(ContainsInBody(br#"name="files[]""#))
            .and(ContainsInBody(br#"filename="my-skill/SKILL.md""#))
            .and(ContainsInBody(br#"filename="my-skill/scripts/helper.py""#))
            .and(ContainsInBody(b"---\nname: x\n---\n"))
            .and(ContainsInBody(b"print('hi')"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateSkillRequest::builder()
            .files(SkillFiles::Directory(vec![
                File {
                    buffer: b"---\nname: x\n---\n".to_vec(),
                    filename: "my-skill/SKILL.md".to_string(),
                },
                File {
                    buffer: b"print('hi')".to_vec(),
                    filename: "my-skill/scripts/helper.py".to_string(),
                },
            ]))
            .build();

        let res = client.skills().create(&req).await.unwrap();
        assert_eq!(res.id, "skill_1");
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/skills/skill_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_skill_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.skill("skill_1").retrieve().await.unwrap();
        assert_eq!(res.name, "spreadsheet-formatter");
    }

    #[tokio::test]
    async fn update_serializes_default_version_and_deserializes_response() {
        use wiremock::matchers::body_json;

        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/skills/skill_1"))
            .and(body_json(serde_json::json!({"default_version": "2"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "skill_1",
                "created_at": 1700000000_i64,
                "default_version": "2",
                "description": "Formats spreadsheets",
                "latest_version": "2",
                "name": "spreadsheet-formatter",
                "object": "skill"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = crate::interfaces::skills::UpdateSkillRequest::builder()
            .default_version("2")
            .build();
        let res = client.skill("skill_1").update(&req).await.unwrap();
        assert_eq!(res.default_version, "2");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        use wiremock::matchers::query_param;

        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/skills"))
            .and(query_param("limit", "10"))
            .and(query_param("order", "desc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [sample_skill_json()],
                "first_id": "skill_1",
                "last_id": "skill_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListSkillsRequest::builder()
            .limit(10_u32)
            .order(SortOrder::Desc)
            .build();
        let res = client.skills().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/skills/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No skill found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client.skill("does-not-exist").delete().await.unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }

    #[tokio::test]
    async fn content_downloads_raw_zip_bytes() {
        let mock_server = MockServer::start().await;
        let zip_bytes: &[u8] = &[0x50, 0x4B, 0x03, 0x04];

        Mock::given(method("GET"))
            .and(path("/skills/skill_1/content"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(zip_bytes, "application/zip"))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.skill("skill_1").content().await.unwrap();
        assert_eq!(res, zip_bytes);
    }
}
