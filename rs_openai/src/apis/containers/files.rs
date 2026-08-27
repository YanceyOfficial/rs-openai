//! A container's files (`/containers/{container_id}/files`), including binary content
//! retrieve — see docs/superpowers/specs/2026-08-14-containers-design.md and
//! docs/superpowers/specs/2026-08-28-containers-skills-raw-upload-design.md.

use crate::client::OpenAI;
use crate::interfaces::containers;
use crate::shared::response_wrapper::OpenAIResponse;
use reqwest::multipart::{Form, Part};

pub struct ContainerFiles<'a> {
    openai: &'a OpenAI,
    container_id: String,
}

impl<'a> ContainerFiles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String) -> Self {
        Self {
            openai,
            container_id,
        }
    }

    /// Attaches a file to this container, either an already-uploaded file (by `file_id`,
    /// sent as a plain JSON body) or raw file bytes (sent as multipart, one `file` part).
    pub async fn create(
        &self,
        req: &containers::CreateContainerFileRequest,
    ) -> OpenAIResponse<containers::ContainerFile> {
        let route = format!("/containers/{}/files", self.container_id);
        match &req.source {
            containers::ContainerFileSource::FileId(file_id) => {
                self.openai
                    .post(&route, &serde_json::json!({"file_id": file_id}))
                    .await
            }
            containers::ContainerFileSource::Upload(file) => {
                let part = Part::stream(file.buffer.clone())
                    .file_name(file.filename.clone())
                    .mime_str("application/octet-stream")
                    .expect("\"application/octet-stream\" is always a valid mime type");
                let form = Form::new().part("file", part);
                self.openai.post_form(&route, form).await
            }
        }
    }

    /// Lists this container's files.
    pub async fn list(
        &self,
        req: &containers::ListContainerFilesRequest,
    ) -> OpenAIResponse<containers::ListContainerFilesResponse> {
        self.openai
            .get(&format!("/containers/{}/files", self.container_id), req)
            .await
    }

    /// Scoped accessor for a specific file in this container.
    pub fn file(&self, id: impl Into<String>) -> ContainerFile<'a> {
        ContainerFile::new(self.openai, self.container_id.clone(), id.into())
    }
}

pub struct ContainerFile<'a> {
    openai: &'a OpenAI,
    container_id: String,
    id: String,
}

impl<'a> ContainerFile<'a> {
    pub(crate) fn new(openai: &'a OpenAI, container_id: String, id: String) -> Self {
        Self {
            openai,
            container_id,
            id,
        }
    }

    /// Retrieves this file's metadata.
    pub async fn retrieve(&self) -> OpenAIResponse<containers::ContainerFile> {
        self.openai
            .get(
                &format!("/containers/{}/files/{}", self.container_id, self.id),
                &(),
            )
            .await
    }

    /// Deletes this file from the container.
    pub async fn delete(&self) -> OpenAIResponse<containers::DeleteContainerFileResponse> {
        self.openai
            .delete(
                &format!("/containers/{}/files/{}", self.container_id, self.id),
                &(),
            )
            .await
    }

    /// Downloads this file's raw content as bytes (may be binary, e.g. a generated image).
    pub async fn content(&self) -> OpenAIResponse<Vec<u8>> {
        self.openai
            .get_raw_bytes(&format!(
                "/containers/{}/files/{}/content",
                self.container_id, self.id
            ))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::containers::{
        ContainerFileSource, CreateContainerFileRequest, ListContainerFilesRequest,
    };
    use crate::shared::types::File;
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, Request, ResponseTemplate};

    fn sample_container_file_json() -> serde_json::Value {
        serde_json::json!({
            "id": "cfile_1",
            "bytes": 1024,
            "container_id": "cntr_1",
            "created_at": 1700000000_i64,
            "object": "container.file",
            "path": "/mnt/data/file_abc.csv",
            "source": "user"
        })
    }

    #[tokio::test]
    async fn create_serializes_file_id_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers/cntr_1/files"))
            .and(body_json(serde_json::json!({"file_id": "file_abc"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_container_file_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerFileRequest::builder()
            .source(ContainerFileSource::FileId("file_abc".to_string()))
            .build();

        let res = client
            .container("cntr_1")
            .files()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "cfile_1");
        assert_eq!(res.container_id, "cntr_1");
        assert_eq!(res.source, "user");
    }

    struct ContainsInBody(&'static [u8]);

    impl wiremock::Match for ContainsInBody {
        fn matches(&self, request: &Request) -> bool {
            request
                .body
                .windows(self.0.len())
                .any(|window| window == self.0)
        }
    }

    #[tokio::test]
    async fn create_sends_raw_bytes_as_multipart_file_part() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/containers/cntr_1/files"))
            .and(ContainsInBody(br#"name="file""#))
            .and(ContainsInBody(&[1, 2, 3, 4]))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_container_file_json()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateContainerFileRequest::builder()
            .source(ContainerFileSource::Upload(File {
                buffer: vec![1, 2, 3, 4],
                filename: "notes.csv".to_string(),
            }))
            .build();

        let res = client
            .container("cntr_1")
            .files()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "cfile_1");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "cfile_2",
                    "bytes": 512,
                    "container_id": "cntr_1",
                    "created_at": 1700000000_i64,
                    "object": "container.file",
                    "path": "/mnt/data/output.png",
                    "source": "assistant"
                }],
                "first_id": "cfile_2",
                "last_id": "cfile_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListContainerFilesRequest::builder().limit(5).build();
        let res = client.container("cntr_1").files().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].source, "assistant");
    }

    #[tokio::test]
    async fn retrieve_deserializes_file() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files/cfile_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cfile_3",
                "bytes": 256,
                "container_id": "cntr_1",
                "created_at": 1700000000_i64,
                "object": "container.file",
                "path": "/mnt/data/notes.txt",
                "source": "user"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_3")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "cfile_3");
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/containers/cntr_1/files/cfile_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cfile_3",
                "object": "container.file.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_3")
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }

    #[tokio::test]
    async fn content_returns_raw_bytes_without_utf8_corruption() {
        let mock_server = MockServer::start().await;

        // Deliberately invalid UTF-8 (0xFF, 0xFE are never valid standalone UTF-8 bytes) --
        // proves `content()` round-trips genuinely binary data byte-for-byte, unlike this
        // crate's existing `.text()`-based download helpers, which would silently replace
        // these bytes during a lossy UTF-8 decode.
        let binary_content: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01, 0x89, 0x50, 0x4E, 0x47];

        Mock::given(method("GET"))
            .and(path("/containers/cntr_1/files/cfile_4/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(binary_content.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .container("cntr_1")
            .files()
            .file("cfile_4")
            .content()
            .await
            .unwrap();
        assert_eq!(res, binary_content);
    }
}
