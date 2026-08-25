//! Project-scoped model permissions
//! (`/organization/projects/{project_id}/model_permissions`): retrieve, update, delete --
//! see docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::model_permissions;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectModelPermissions<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectModelPermissions<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Retrieves this project's model allowlist/denylist policy.
    pub async fn retrieve(&self) -> OpenAIResponse<model_permissions::ProjectModelPermissions> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/model_permissions",
                    self.project_id
                ),
                &(),
            )
            .await
    }

    /// Sets this project's model allowlist/denylist policy.
    pub async fn update(
        &self,
        req: &model_permissions::UpdateModelPermissionsRequest,
    ) -> OpenAIResponse<model_permissions::ProjectModelPermissions> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/model_permissions",
                    self.project_id
                ),
                req,
            )
            .await
    }

    /// Deletes this project's model permissions policy.
    pub async fn delete(&self) -> OpenAIResponse<model_permissions::ModelPermissionsDeleted> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/model_permissions",
                    self.project_id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::model_permissions::{
        ModelPermissionsMode, UpdateModelPermissionsRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/model_permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "mode": "allow_list",
                "model_ids": ["gpt-5", "gpt-5-mini"],
                "object": "project.model_permissions"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .project("proj_1")
            .model_permissions()
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.mode.to_string(), "allow_list");
        assert_eq!(res.model_ids.len(), 2);
    }

    #[tokio::test]
    async fn update_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/model_permissions"))
            .and(body_json(serde_json::json!({
                "mode": "deny_list",
                "model_ids": ["gpt-3.5-turbo"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "mode": "deny_list",
                "model_ids": ["gpt-3.5-turbo"],
                "object": "project.model_permissions"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateModelPermissionsRequest::builder()
            .mode(ModelPermissionsMode::DenyList)
            .model_ids(vec!["gpt-3.5-turbo".to_string()])
            .build();
        let res = client
            .organization()
            .project("proj_1")
            .model_permissions()
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.mode.to_string(), "deny_list");
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/model_permissions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "project.model_permissions.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .project("proj_1")
            .model_permissions()
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }
}
