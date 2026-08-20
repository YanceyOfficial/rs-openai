//! Organization invites (`/organization/invites`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::invites;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationInvites<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationInvites<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(
        &self,
        req: &invites::CreateInviteRequest,
    ) -> OpenAIResponse<invites::Invite> {
        self.openai.post("/organization/invites", req).await
    }

    pub async fn list(
        &self,
        req: &invites::ListInvitesRequest,
    ) -> OpenAIResponse<invites::ListInvitesResponse> {
        self.openai.get("/organization/invites", req).await
    }
}

pub struct OrganizationInvite<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationInvite<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<invites::Invite> {
        self.openai
            .get(&format!("/organization/invites/{}", self.id), &())
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<invites::DeleteInviteResponse> {
        self.openai
            .delete(&format!("/organization/invites/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::invites::{
        CreateInviteRequest, InviteProject, ListInvitesRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_projects_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/invites"))
            .and(body_json(serde_json::json!({
                "email": "new@example.com",
                "role": "reader",
                "projects": [{"id": "proj_1", "role": "member"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [{"id": "proj_1", "role": "member"}],
                "role": "reader",
                "status": "pending"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateInviteRequest::builder()
            .email("new@example.com")
            .role("reader")
            .projects(vec![
                InviteProject::builder().id("proj_1").role("member").build(),
            ])
            .build();
        let result = client.organization().invites().create(&req).await.unwrap();

        assert_eq!(result.id, "invite_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.projects.len(), 1);
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_projects() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/invites"))
            .and(body_json(serde_json::json!({
                "email": "new@example.com",
                "role": "owner"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_2",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [],
                "role": "owner",
                "status": "pending"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateInviteRequest::builder()
            .email("new@example.com")
            .role("owner")
            .build();
        let result = client.organization().invites().create(&req).await.unwrap();

        assert!(result.projects.is_empty());
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/invites"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListInvitesRequest::builder().limit(5).build();
        let result = client.organization().invites().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn retrieve_deserializes_invite() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/invites/invite_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [],
                "role": "reader",
                "status": "accepted",
                "accepted_at": 1700000100_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .invite("invite_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.status, "accepted");
        assert_eq!(result.accepted_at, Some(1700000100));
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/invites/invite_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "deleted": true,
                "object": "organization.invite.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .invite("invite_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
}
