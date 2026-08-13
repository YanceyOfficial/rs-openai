//! ChatKit threads: `GET /chatkit/threads` list, `GET .../{thread_id}` retrieve,
//! `DELETE .../{thread_id}` delete. `GET .../{thread_id}/items` list items still not
//! implemented — see docs/superpowers/specs/2026-08-14-chatkit-design.md.

use crate::client::OpenAI;
use crate::interfaces::chatkit;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct ChatKitThreads<'a> {
    openai: &'a OpenAI,
}

impl<'a> ChatKitThreads<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists ChatKit threads.
    pub async fn list(
        &self,
        req: &chatkit::ListChatKitThreadsRequest,
    ) -> OpenAIResponse<chatkit::ListChatKitThreadsResponse> {
        self.openai.chatkit_get("/chatkit/threads", req).await
    }
}

pub struct ChatKitThread<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> ChatKitThread<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a ChatKit thread by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<chatkit::ChatKitThread> {
        self.openai
            .chatkit_get(&format!("/chatkit/threads/{}", self.id), &())
            .await
    }

    /// Deletes a ChatKit thread.
    pub async fn delete(&self) -> OpenAIResponse<chatkit::DeleteChatKitThreadResponse> {
        self.openai
            .chatkit_delete(&format!("/chatkit/threads/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this thread's items. Not yet implemented.
    pub fn items(&self) -> ChatKitThreadItems<'a> {
        ChatKitThreadItems::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct ChatKitThreadItems<'a> {
    openai: &'a OpenAI,
    thread_id: String,
}

impl<'a> ChatKitThreadItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, thread_id: String) -> Self {
        Self { openai, thread_id }
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chatkit::ListChatKitThreadsRequest;
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_serializes_pagination_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chatkit/threads"))
            .and(header("OpenAI-Beta", "chatkit_beta=v1"))
            .and(query_param("limit", "10"))
            .and(query_param("after", "thread_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [
                    {
                        "id": "thread_1",
                        "object": "chatkit.thread",
                        "created_at": 1700000000_i64,
                        "status": "active",
                        "title": "Support chat"
                    },
                    {
                        "id": "thread_2",
                        "object": "chatkit.thread",
                        "created_at": 1700000100_i64,
                        "status": "active",
                        "title": null
                    }
                ],
                "first_id": "thread_1",
                "last_id": "thread_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListChatKitThreadsRequest::builder()
            .after("thread_abc")
            .limit(10)
            .build();

        let res = client.chatkit().threads().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 2);
        assert_eq!(res.data[0].id, "thread_1");
        assert_eq!(res.data[0].title.as_deref(), Some("Support chat"));
        assert!(res.data[1].title.is_none());
        assert!(!res.has_more);
    }

    #[tokio::test]
    async fn retrieve_deserializes_thread() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/chatkit/threads/thread_9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_9",
                "object": "chatkit.thread",
                "created_at": 1700000000_i64,
                "status": "active",
                "title": "Billing question"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .chatkit()
            .thread("thread_9")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "thread_9");
        assert_eq!(res.title.as_deref(), Some("Billing question"));
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/chatkit/threads/thread_9"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "thread_9",
                "object": "chatkit.thread.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.chatkit().thread("thread_9").delete().await.unwrap();
        assert!(res.deleted);
    }
}
