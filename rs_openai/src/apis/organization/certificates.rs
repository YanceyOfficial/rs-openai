//! Organization certificates (`/organization/certificates`): upload, retrieve, update, list,
//! delete, activate, deactivate -- see
//! docs/superpowers/specs/2026-08-25-org-billing-compliance-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::certificates;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationCertificates<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationCertificates<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Uploads a certificate to the organization. Does NOT automatically activate it.
    pub async fn create(
        &self,
        req: &certificates::CreateCertificateRequest,
    ) -> OpenAIResponse<certificates::Certificate> {
        self.openai.post("/organization/certificates", req).await
    }

    /// Lists uploaded certificates for the organization.
    pub async fn list(
        &self,
        req: &certificates::ListCertificatesRequest,
    ) -> OpenAIResponse<certificates::ListCertificatesResponse> {
        self.openai.get("/organization/certificates", req).await
    }

    /// Activates certificates at the organization level (up to 10 at a time).
    pub async fn activate(
        &self,
        req: &certificates::ActivateCertificatesRequest,
    ) -> OpenAIResponse<certificates::CertificatesPage> {
        self.openai
            .post("/organization/certificates/activate", req)
            .await
    }

    /// Deactivates certificates at the organization level (up to 10 at a time).
    pub async fn deactivate(
        &self,
        req: &certificates::DeactivateCertificatesRequest,
    ) -> OpenAIResponse<certificates::CertificatesPage> {
        self.openai
            .post("/organization/certificates/deactivate", req)
            .await
    }
}

pub struct OrganizationCertificate<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationCertificate<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves a certificate. `include: ["content"]` also fetches its PEM content.
    pub async fn retrieve(
        &self,
        req: &certificates::RetrieveCertificateRequest,
    ) -> OpenAIResponse<certificates::Certificate> {
        self.openai
            .get_with_pairs(
                &format!("/organization/certificates/{}", self.id),
                &req.to_query_pairs(),
            )
            .await
    }

    /// Modifies a certificate. Only the name can be changed.
    pub async fn update(
        &self,
        req: &certificates::UpdateCertificateRequest,
    ) -> OpenAIResponse<certificates::Certificate> {
        self.openai
            .post(&format!("/organization/certificates/{}", self.id), req)
            .await
    }

    /// Deletes a certificate. It must be inactive for the organization and all projects.
    pub async fn delete(&self) -> OpenAIResponse<certificates::DeleteCertificateResponse> {
        self.openai
            .delete(&format!("/organization/certificates/{}", self.id), &())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::certificates::{
        ActivateCertificatesRequest, CreateCertificateRequest, DeactivateCertificatesRequest,
        ListCertificatesRequest, RetrieveCertificateRequest, UpdateCertificateRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_certificate_json(active: bool) -> serde_json::Value {
        serde_json::json!({
            "id": "cert_1",
            "certificate_details": {"expires_at": 1800000000_i64, "valid_at": 1700000000_i64},
            "created_at": 1700000000_i64,
            "name": "My Cert",
            "object": "organization.certificate",
            "active": active
        })
    }

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/certificates"))
            .and(body_json(serde_json::json!({
                "certificate": "-----BEGIN CERTIFICATE-----...",
                "name": "My Cert"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cert_1",
                "certificate_details": {"content": null, "expires_at": null, "valid_at": null},
                "created_at": 1700000000_i64,
                "name": "My Cert",
                "object": "certificate",
                "active": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateCertificateRequest::builder()
            .certificate("-----BEGIN CERTIFICATE-----...")
            .name("My Cert")
            .build();

        let res = client
            .organization()
            .certificates()
            .create(&req)
            .await
            .unwrap();
        assert_eq!(res.id, "cert_1");
        assert_eq!(res.active, None);
    }

    #[tokio::test]
    async fn list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/certificates"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [sample_certificate_json(true)],
                "first_id": "cert_1",
                "last_id": "cert_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListCertificatesRequest::builder().limit(10_u32).build();
        let res = client
            .organization()
            .certificates()
            .list(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].active, Some(true));
    }

    #[tokio::test]
    async fn activate_serializes_ids_and_deserializes_page() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/certificates/activate"))
            .and(body_json(
                serde_json::json!({"certificate_ids": ["cert_1", "cert_2"]}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [sample_certificate_json(true)]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ActivateCertificatesRequest::builder()
            .certificate_ids(vec!["cert_1".to_string(), "cert_2".to_string()])
            .build();
        let res = client
            .organization()
            .certificates()
            .activate(&req)
            .await
            .unwrap();
        assert_eq!(res.data.len(), 1);
    }

    #[tokio::test]
    async fn deactivate_serializes_ids_and_deserializes_page() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/certificates/deactivate"))
            .and(body_json(
                serde_json::json!({"certificate_ids": ["cert_1"]}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [sample_certificate_json(false)]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = DeactivateCertificatesRequest::builder()
            .certificate_ids(vec!["cert_1".to_string()])
            .build();
        let res = client
            .organization()
            .certificates()
            .deactivate(&req)
            .await
            .unwrap();
        assert_eq!(res.data[0].active, Some(false));
    }

    #[tokio::test]
    async fn retrieve_sends_include_content_query_param() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/certificates/cert_1"))
            .and(query_param("include", "content"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cert_1",
                "certificate_details": {
                    "content": "-----BEGIN CERTIFICATE-----...",
                    "expires_at": 1800000000_i64,
                    "valid_at": 1700000000_i64
                },
                "created_at": 1700000000_i64,
                "name": "My Cert",
                "object": "certificate",
                "active": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = RetrieveCertificateRequest::builder()
            .include(vec!["content".to_string()])
            .build();
        let res = client
            .organization()
            .certificate("cert_1")
            .retrieve(&req)
            .await
            .unwrap();
        assert_eq!(
            res.certificate_details.content.as_deref(),
            Some("-----BEGIN CERTIFICATE-----...")
        );
    }

    #[tokio::test]
    async fn update_serializes_name_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/certificates/cert_1"))
            .and(body_json(serde_json::json!({"name": "Renamed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "cert_1",
                "certificate_details": {"content": null, "expires_at": null, "valid_at": null},
                "created_at": 1700000000_i64,
                "name": "Renamed",
                "object": "certificate",
                "active": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateCertificateRequest::builder().name("Renamed").build();
        let res = client
            .organization()
            .certificate("cert_1")
            .update(&req)
            .await
            .unwrap();
        assert_eq!(res.name.as_deref(), Some("Renamed"));
    }

    #[tokio::test]
    async fn delete_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/certificates/does-not-exist"))
            .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
                "error": {
                    "message": "No certificate found",
                    "type": "invalid_request_error",
                    "param": null,
                    "code": "not_found"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let err = client
            .organization()
            .certificate("does-not-exist")
            .delete()
            .await
            .unwrap_err();
        assert!(matches!(
            err,
            crate::shared::response_wrapper::OpenAIError::ApiError(_)
        ));
    }
}
