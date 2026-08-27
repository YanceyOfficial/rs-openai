//! A skill's versions (`/v1/skills/{skill_id}/versions`) -- immutable once created; only
//! create/retrieve/list/delete/content, no update -- see
//! docs/superpowers/specs/2026-08-27-skills-design.md.

use crate::client::OpenAI;
use crate::interfaces::skills;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::{Form, Part};

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

pub struct SkillVersions<'a> {
    openai: &'a OpenAI,
    skill_id: String,
}

impl<'a> SkillVersions<'a> {
    pub(crate) fn new(openai: &'a OpenAI, skill_id: String) -> Self {
        Self { openai, skill_id }
    }

    /// Creates a new immutable skill version. `req.default` sets it as the skill's default
    /// version; the multipart form's `default` field is sent as a text part alongside the
    /// file part(s), matching this crate's established text-plus-file multipart pattern
    /// (`Files::upload`'s `purpose` field).
    pub async fn create(
        &self,
        req: &skills::CreateSkillVersionRequest,
    ) -> OpenAIResponse<skills::SkillVersion> {
        let mut form = skill_files_form(&req.files);
        if let Some(default) = req.default {
            form = form.text("default", default.to_string());
        }
        self.openai
            .post_form(&format!("/skills/{}/versions", self.skill_id), form)
            .await
    }

    /// Lists skill versions for this skill.
    pub async fn list(
        &self,
        req: &skills::ListSkillVersionsRequest,
    ) -> OpenAIResponse<skills::ListSkillVersionsResponse> {
        self.openai
            .get(&format!("/skills/{}/versions", self.skill_id), req)
            .await
    }
}

pub struct SkillVersion<'a> {
    openai: &'a OpenAI,
    skill_id: String,
    version: String,
}

impl<'a> SkillVersion<'a> {
    pub(crate) fn new(openai: &'a OpenAI, skill_id: String, version: String) -> Self {
        Self {
            openai,
            skill_id,
            version,
        }
    }

    /// Gets a specific skill version.
    pub async fn retrieve(&self) -> OpenAIResponse<skills::SkillVersion> {
        self.openai
            .get(
                &format!("/skills/{}/versions/{}", self.skill_id, self.version),
                &(),
            )
            .await
    }

    /// Deletes a skill version.
    pub async fn delete(&self) -> OpenAIResponse<skills::DeletedSkillVersion> {
        self.openai
            .delete(
                &format!("/skills/{}/versions/{}", self.skill_id, self.version),
                &(),
            )
            .await
    }

    /// Downloads a skill version zip bundle.
    pub async fn content(&self) -> OpenAIResponse<Vec<u8>> {
        self.openai
            .get_raw_bytes(&format!(
                "/skills/{}/versions/{}/content",
                self.skill_id, self.version
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::skills::{
        CreateSkillVersionRequest, ListSkillVersionsRequest, SkillFiles,
    };
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

    fn sample_version_json() -> serde_json::Value {
        serde_json::json!({
            "id": "skillver_1",
            "created_at": 1700000000_i64,
            "description": "Formats spreadsheets",
            "name": "spreadsheet-formatter",
            "object": "skill.version",
            "skill_id": "skill_1",
            "version": "2"
        })
    }

    #[tokio::test]
    async fn create_sends_default_text_field_alongside_zip_part() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/skills/skill_1/versions"))
            .and(ContainsInBody(br#"name="default""#))
            .and(ContainsInBody(b"true"))
            .and(ContainsInBody(br#"name="files""#))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_version_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateSkillVersionRequest::builder()
            .default(true)
            .files(SkillFiles::Zip(File {
                buffer: vec![1, 2, 3],
                filename: "skill.zip".to_string(),
            }))
            .build();

        let res = client
            .skill("skill_1")
            .versions()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.version, "2");
    }

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/skills/skill_1/versions/2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_version_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .skill("skill_1")
            .version("2")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.skill_id, "skill_1");
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/skills/skill_1/versions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [sample_version_json()],
                "first_id": "skillver_1",
                "last_id": "skillver_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListSkillVersionsRequest::builder().build();
        let res = client.skill("skill_1").versions().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/skills/skill_1/versions/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No skill version found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .skill("skill_1")
            .version("does-not-exist")
            .delete()
            .await
            .unwrap_err();
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
            .and(path("/skills/skill_1/versions/2/content"))
            .respond_with(ResponseTemplate::new(200).set_body_raw(zip_bytes, "application/zip"))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .skill("skill_1")
            .version("2")
            .content()
            .await
            .unwrap();
        assert_eq!(res, zip_bytes);
    }
}
