//! Sora video generation -- see docs/superpowers/specs/2026-08-18-videos-design.md.

use crate::client::OpenAI;
use crate::interfaces::videos;
pub use crate::interfaces::videos::*;
use crate::shared::response_wrapper::OpenAIResponse;

/// The Sora Video API is deprecated and sunsets **2026-09-24** (confirmed via every method
/// in OpenAI's own SDK carrying a `@deprecated` notice with this exact date). This module
/// receives no further feature work -- its deferred pieces (`input_reference`, `characters`,
/// `extend`, `remix`, `edit`) are deliberately not implemented, matching the crate's existing
/// treatment of the equally-sunsetting Assistants API (`apis::assistants`).
pub struct Videos<'a> {
    openai: &'a OpenAI,
}

impl<'a> Videos<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a video generation job.
    pub async fn create(&self, req: &videos::CreateVideoRequest) -> OpenAIResponse<videos::Video> {
        self.openai.post("/videos", req).await
    }

    /// Lists videos.
    pub async fn list(
        &self,
        req: &videos::ListVideosRequest,
    ) -> OpenAIResponse<videos::ListVideosResponse> {
        self.openai.get("/videos", req).await
    }
}

pub struct Video<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Video<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a video by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<videos::Video> {
        self.openai.get(&format!("/videos/{}", self.id), &()).await
    }

    /// Deletes a video.
    pub async fn delete(&self) -> OpenAIResponse<videos::DeleteVideoResponse> {
        self.openai
            .delete(&format!("/videos/{}", self.id), &())
            .await
    }

    /// Downloads this video's raw content. `variant` selects a supporting asset
    /// (thumbnail/spritesheet) instead of the video itself; `None` downloads the video.
    pub async fn content(
        &self,
        variant: Option<videos::VideoContentVariant>,
    ) -> OpenAIResponse<Vec<u8>> {
        let route = match variant {
            Some(v) => format!("/videos/{}/content?variant={v}", self.id),
            None => format!("/videos/{}/content", self.id),
        };
        self.openai.get_raw_bytes(&route).await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::videos::{
        CreateVideoRequest, ListVideosRequest, VideoContentVariant, VideoSize,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/videos"))
            .and(body_json(serde_json::json!({
                "model": "sora-2",
                "prompt": "A gray tabby cat playing piano",
                "size": "1280x720",
                "seconds": 8
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_1",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "queued",
                "model": "sora-2",
                "progress": 0,
                "seconds": "8",
                "size": "1280x720",
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVideoRequest::builder()
            .model("sora-2")
            .prompt("A gray tabby cat playing piano")
            .size(VideoSize::S1280x720)
            .seconds(8)
            .build();

        let res = client.videos().create(&req).await.unwrap();
        assert_eq!(res.id, "video_1");
        assert_eq!(res.status, "queued");
        assert_eq!(res.seconds.as_deref(), Some("8"));
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/videos"))
            .and(body_json(serde_json::json!({
                "model": "sora-2",
                "prompt": "A quiet forest at dawn"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_2",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "queued",
                "model": "sora-2",
                "progress": null,
                "seconds": null,
                "size": null,
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateVideoRequest::builder()
            .model("sora-2")
            .prompt("A quiet forest at dawn")
            .build();

        let res = client.videos().create(&req).await.unwrap();
        assert_eq!(res.id, "video_2");
        assert!(res.seconds.is_none());
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/videos"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "video_3",
                    "object": "video",
                    "created_at": 1758941485_i64,
                    "status": "completed",
                    "model": "sora-2",
                    "progress": 100,
                    "seconds": "8",
                    "size": "1280x720",
                    "error": null
                }],
                "first_id": "video_3",
                "last_id": "video_3",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListVideosRequest::builder().limit(5).build();
        let res = client.videos().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "video_3");
    }

    #[tokio::test]
    async fn retrieve_deserializes_video() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/videos/video_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_4",
                "object": "video",
                "created_at": 1758941485_i64,
                "status": "failed",
                "model": "sora-2",
                "progress": null,
                "seconds": null,
                "size": null,
                "error": {"message": "Content policy violation"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_4").retrieve().await.unwrap();
        assert_eq!(res.status, "failed");
        assert_eq!(
            res.error.as_ref().map(|e| e.message.as_str()),
            Some("Content policy violation")
        );
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/videos/video_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "video_4",
                "object": "video.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_4").delete().await.unwrap();
        assert!(res.deleted);
    }

    #[tokio::test]
    async fn content_downloads_raw_bytes_with_no_variant() {
        let mock_server = MockServer::start().await;
        let video_bytes: Vec<u8> = vec![0x00, 0x00, 0x00, 0x18, 0x66, 0x74, 0x79, 0x70];

        Mock::given(method("GET"))
            .and(path("/videos/video_5/content"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(video_bytes.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.video("video_5").content(None).await.unwrap();
        assert_eq!(res, video_bytes);
    }

    #[tokio::test]
    async fn content_appends_variant_query_param() {
        let mock_server = MockServer::start().await;
        let thumbnail_bytes: Vec<u8> = vec![0xFF, 0xD8, 0xFF, 0xE0];

        Mock::given(method("GET"))
            .and(path("/videos/video_5/content"))
            .and(query_param("variant", "thumbnail"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(thumbnail_bytes.clone()))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .video("video_5")
            .content(Some(VideoContentVariant::Thumbnail))
            .await
            .unwrap();
        assert_eq!(res, thumbnail_bytes);
    }
}
