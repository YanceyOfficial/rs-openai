//! Project-scoped certificates (`/organization/projects/{project_id}/certificates`):
//! list/activate/deactivate only -- a project cannot create/retrieve/update/delete a
//! certificate directly, only manage its activation state -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::certificates;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectCertificates<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectCertificates<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    /// Lists certificates for this project.
    pub async fn list(
        &self,
        req: &certificates::ListCertificatesRequest,
    ) -> OpenAIResponse<certificates::ListCertificatesResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/certificates", self.project_id),
                req,
            )
            .await
    }

    /// Activates certificates for this project (up to 10 at a time).
    pub async fn activate(
        &self,
        req: &certificates::ActivateCertificatesRequest,
    ) -> OpenAIResponse<certificates::CertificatesPage> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/certificates/activate",
                    self.project_id
                ),
                req,
            )
            .await
    }

    /// Deactivates certificates for this project (up to 10 at a time).
    pub async fn deactivate(
        &self,
        req: &certificates::DeactivateCertificatesRequest,
    ) -> OpenAIResponse<certificates::CertificatesPage> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/certificates/deactivate",
                    self.project_id
                ),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::certificates::{
        ActivateCertificatesRequest, DeactivateCertificatesRequest, ListCertificatesRequest,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/certificates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "cert_1",
                    "certificate_details": {"expires_at": null, "valid_at": null},
                    "created_at": 1700000000_i64,
                    "name": null,
                    "object": "organization.project.certificate",
                    "active": true
                }],
                "first_id": "cert_1",
                "last_id": "cert_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListCertificatesRequest::builder().build();
        let res = client
            .organization()
            .project("proj_1")
            .certificates()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].object, "organization.project.certificate");
    }

    #[tokio::test]
    async fn activate_serializes_ids_and_deserializes_page() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/certificates/activate"))
            .and(body_json(
                serde_json::json!({"certificate_ids": ["cert_1"]}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ActivateCertificatesRequest::builder()
            .certificate_ids(vec!["cert_1".to_string()])
            .build();
        client
            .organization()
            .project("proj_1")
            .certificates()
            .activate(&req)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn deactivate_serializes_ids_and_deserializes_page() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/organization/projects/proj_1/certificates/deactivate",
            ))
            .and(body_json(
                serde_json::json!({"certificate_ids": ["cert_1"]}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = DeactivateCertificatesRequest::builder()
            .certificate_ids(vec!["cert_1".to_string()])
            .build();
        client
            .organization()
            .project("proj_1")
            .certificates()
            .deactivate(&req)
            .await
            .unwrap();
    }
}
