//! A project's rate limits (`/organization/projects/{project_id}/rate_limits`): list,
//! update -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::rate_limits;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectRateLimits<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectRateLimits<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn list_rate_limits(
        &self,
        req: &rate_limits::ListProjectRateLimitsRequest,
    ) -> OpenAIResponse<rate_limits::ListProjectRateLimitsResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/rate_limits", self.project_id),
                req,
            )
            .await
    }

    pub fn rate_limit(&self, id: impl Into<String>) -> OrganizationProjectRateLimit<'a> {
        OrganizationProjectRateLimit::new(self.openai, self.project_id.clone(), id.into())
    }
}

pub struct OrganizationProjectRateLimit<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectRateLimit<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn update(
        &self,
        req: &rate_limits::UpdateProjectRateLimitRequest,
    ) -> OpenAIResponse<rate_limits::ProjectRateLimit> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/rate_limits/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::rate_limits::{
        ListProjectRateLimitsRequest, UpdateProjectRateLimitRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_rate_limits_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/rate_limits"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "rl_1",
                    "max_requests_per_1_minute": 500,
                    "max_tokens_per_1_minute": 10000,
                    "model": "gpt-4",
                    "object": "project.rate_limit"
                }],
                "first_id": "rl_1",
                "last_id": "rl_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectRateLimitsRequest::builder().limit(5).build();
        let result = client
            .organization()
            .project("proj_1")
            .rate_limits()
            .list_rate_limits(&req)
            .await
            .unwrap();

        assert_eq!(result.data[0].model, "gpt-4");
        assert_eq!(result.data[0].batch_1_day_max_input_tokens, None);
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/rate_limits/rl_1"))
            .and(body_json(
                serde_json::json!({"max_requests_per_1_minute": 1000}),
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "rl_1",
                "max_requests_per_1_minute": 1000,
                "max_tokens_per_1_minute": 10000,
                "model": "gpt-4",
                "object": "project.rate_limit"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectRateLimitRequest::builder()
            .max_requests_per_1_minute(1000)
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .rate_limits()
            .rate_limit("rl_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.max_requests_per_1_minute, 1000);
    }
}
