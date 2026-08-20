//! Organization usage reporting (`/organization/usage/*`, `/organization/costs`) -- see
//! docs/superpowers/specs/2026-08-20-org-usage-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::usage;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationUsage<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsage<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn completions(
        &self,
        req: &usage::UsageCompletionsRequest,
    ) -> OpenAIResponse<usage::UsageResponse> {
        self.openai
            .get_with_pairs("/organization/usage/completions", &req.to_query_pairs())
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::usage::UsageCompletionsRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn completions_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .and(query_param("start_time", "1700000000"))
            .and(query_param("end_time", "1700086400"))
            .and(query_param("bucket_width", "1d"))
            .and(query_param("limit", "7"))
            .and(query_param("group_by", "model"))
            .and(query_param("group_by", "project_id"))
            .and(query_param("project_ids", "proj_1"))
            .and(query_param("project_ids", "proj_2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [{
                        "object": "organization.usage.completions.result",
                        "input_tokens": 100_i64,
                        "output_tokens": 50_i64,
                        "num_model_requests": 2_i64,
                        "model": "gpt-4o"
                    }]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder()
            .start_time(1700000000)
            .end_time(1700086400)
            .bucket_width("1d")
            .limit(7)
            .group_by(vec!["model".to_string(), "project_id".to_string()])
            .project_ids(vec!["proj_1".to_string(), "proj_2".to_string()])
            .build();
        let result = client
            .organization()
            .usage()
            .completions(&req)
            .await
            .unwrap();

        assert!(!result.has_more);
        assert_eq!(result.data.len(), 1);
        assert_eq!(result.data[0].results.len(), 1);
    }

    #[tokio::test]
    async fn completions_with_only_required_field_omits_optionals() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .and(query_param("start_time", "1700000000"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder()
            .start_time(1700000000)
            .build();
        let result = client
            .organization()
            .usage()
            .completions(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn usage_result_deserializes_into_correct_variant_by_object_tag() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/usage/completions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "page",
                "has_more": false,
                "next_page": null,
                "data": [{
                    "object": "bucket",
                    "start_time": 1700000000_i64,
                    "end_time": 1700086400_i64,
                    "results": [
                        {
                            "object": "organization.usage.completions.result",
                            "input_tokens": 100_i64,
                            "output_tokens": 50_i64,
                            "num_model_requests": 2_i64
                        },
                        {
                            "object": "organization.usage.embeddings.result",
                            "input_tokens": 30_i64,
                            "num_model_requests": 1_i64
                        }
                    ]
                }]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UsageCompletionsRequest::builder()
            .start_time(1700000000)
            .build();
        let result = client
            .organization()
            .usage()
            .completions(&req)
            .await
            .unwrap();

        let results = &result.data[0].results;
        assert_eq!(results.len(), 2);
        match &results[0] {
            crate::interfaces::organization::usage::UsageResult::Completions(r) => {
                assert_eq!(r.input_tokens, 100);
            }
            other => panic!("expected Completions variant, got {other:?}"),
        }
        match &results[1] {
            crate::interfaces::organization::usage::UsageResult::Embeddings(r) => {
                assert_eq!(r.input_tokens, 30);
            }
            other => panic!("expected Embeddings variant, got {other:?}"),
        }
    }
}
