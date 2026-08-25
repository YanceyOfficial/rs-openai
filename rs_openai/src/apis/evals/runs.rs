//! An eval's runs (`/evals/{eval_id}/runs`): create, retrieve, update, list, delete,
//! cancel. Includes the nested Output Items sub-resource (retrieve/list only) -- see
//! docs/superpowers/specs/2026-08-20-evals-runs-design.md.
//!
//! `cancel()` is confirmed via source to POST to the SAME route as `retrieve()`
//! (`/evals/{eval_id}/runs/{run_id}`), not a `/cancel`-suffixed path.

use crate::client::OpenAI;
use crate::interfaces::evals::{
    CreateEvalRunRequest, DeleteEvalRunResponse, ListEvalRunOutputItemsRequest,
    ListEvalRunOutputItemsResponse, ListEvalRunsRequest, ListEvalRunsResponse,
};
use crate::shared::response_wrapper::OpenAIResponse;

pub struct EvalRuns<'a> {
    openai: &'a OpenAI,
    eval_id: String,
}

impl<'a> EvalRuns<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String) -> Self {
        Self { openai, eval_id }
    }

    pub async fn create(
        &self,
        req: &CreateEvalRunRequest,
    ) -> OpenAIResponse<crate::interfaces::evals::EvalRun> {
        self.openai
            .post(&format!("/evals/{}/runs", self.eval_id), req)
            .await
    }

    pub async fn list(&self, req: &ListEvalRunsRequest) -> OpenAIResponse<ListEvalRunsResponse> {
        self.openai
            .get(&format!("/evals/{}/runs", self.eval_id), req)
            .await
    }
}

pub struct EvalRun<'a> {
    openai: &'a OpenAI,
    eval_id: String,
    run_id: String,
}

impl<'a> EvalRun<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String, run_id: String) -> Self {
        Self {
            openai,
            eval_id,
            run_id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<crate::interfaces::evals::EvalRun> {
        self.openai
            .get(
                &format!("/evals/{}/runs/{}", self.eval_id, self.run_id),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<DeleteEvalRunResponse> {
        self.openai
            .delete(
                &format!("/evals/{}/runs/{}", self.eval_id, self.run_id),
                &(),
            )
            .await
    }

    pub async fn cancel(&self) -> OpenAIResponse<crate::interfaces::evals::EvalRun> {
        self.openai
            .post(
                &format!("/evals/{}/runs/{}", self.eval_id, self.run_id),
                &(),
            )
            .await
    }

    /// Scoped accessor for this run's output items.
    pub fn output_items(&self) -> EvalRunOutputItems<'a> {
        EvalRunOutputItems::new(self.openai, self.eval_id.clone(), self.run_id.clone())
    }
}

pub struct EvalRunOutputItems<'a> {
    openai: &'a OpenAI,
    eval_id: String,
    run_id: String,
}

impl<'a> EvalRunOutputItems<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String, run_id: String) -> Self {
        Self {
            openai,
            eval_id,
            run_id,
        }
    }

    pub async fn list(
        &self,
        req: &ListEvalRunOutputItemsRequest,
    ) -> OpenAIResponse<ListEvalRunOutputItemsResponse> {
        self.openai
            .get(
                &format!("/evals/{}/runs/{}/output_items", self.eval_id, self.run_id),
                req,
            )
            .await
    }

    pub fn item(&self, id: impl Into<String>) -> EvalRunOutputItem<'a> {
        EvalRunOutputItem::new(
            self.openai,
            self.eval_id.clone(),
            self.run_id.clone(),
            id.into(),
        )
    }
}

pub struct EvalRunOutputItem<'a> {
    openai: &'a OpenAI,
    eval_id: String,
    run_id: String,
    id: String,
}

impl<'a> EvalRunOutputItem<'a> {
    pub(crate) fn new(openai: &'a OpenAI, eval_id: String, run_id: String, id: String) -> Self {
        Self {
            openai,
            eval_id,
            run_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<crate::interfaces::evals::EvalRunOutputItem> {
        self.openai
            .get(
                &format!(
                    "/evals/{}/runs/{}/output_items/{}",
                    self.eval_id, self.run_id, self.id
                ),
                &(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::evals::{
        CreateEvalRunRequest, ListEvalRunOutputItemsRequest, ListEvalRunsRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn sample_eval_run_json(id: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id,
            "created_at": 1700000000_i64,
            "data_source": {"type": "jsonl", "source": {"type": "file_id", "id": "file_1"}},
            "error": {"code": "", "message": ""},
            "eval_id": "eval_1",
            "metadata": null,
            "model": "gpt-4o-mini",
            "name": "Run 1",
            "object": "eval.run",
            "per_model_usage": [],
            "per_testing_criteria_results": [],
            "report_url": "https://platform.openai.com/evals/eval_1/runs/run_1",
            "result_counts": {"errored": 0, "failed": 0, "passed": 0, "total": 0},
            "status": "queued"
        })
    }

    #[tokio::test]
    async fn create_round_trips_raw_data_source_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/evals/eval_1/runs"))
            .and(body_json(serde_json::json!({
                "data_source": {"type": "jsonl", "source": {"type": "file_id", "id": "file_1"}},
                "name": "Run 1"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_eval_run_json("run_1")))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEvalRunRequest::builder()
            .data_source(
                serde_json::json!({"type": "jsonl", "source": {"type": "file_id", "id": "file_1"}}),
            )
            .name("Run 1")
            .build();
        let result = client.eval("eval_1").runs().create(&req).await.unwrap();

        assert_eq!(result.id, "run_1");
        assert_eq!(result.status, "queued");
        assert_eq!(result.data_source["type"], "jsonl");
    }

    #[tokio::test]
    async fn retrieve_deserializes_run() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals/eval_1/runs/run_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_eval_run_json("run_1")))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let result = client.eval("eval_1").run("run_1").retrieve().await.unwrap();

        assert_eq!(result.id, "run_1");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals/eval_1/runs"))
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

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListEvalRunsRequest::builder().limit(5).build();
        let result = client.eval("eval_1").runs().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation_with_run_id_not_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/evals/eval_1/runs/run_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "eval.run.deleted",
                "run_id": "run_1"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let result = client.eval("eval_1").run("run_1").delete().await.unwrap();

        assert_eq!(result.run_id.as_deref(), Some("run_1"));
        assert_eq!(result.deleted, Some(true));
    }

    #[tokio::test]
    async fn cancel_posts_to_same_route_as_retrieve() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/evals/eval_1/runs/run_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(sample_eval_run_json("run_1")))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let result = client.eval("eval_1").run("run_1").cancel().await.unwrap();

        assert_eq!(result.id, "run_1");
    }

    #[tokio::test]
    async fn output_items_list_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals/eval_1/runs/run_1/output_items"))
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

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListEvalRunOutputItemsRequest::builder().limit(5).build();
        let result = client
            .eval("eval_1")
            .run("run_1")
            .output_items()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn output_item_retrieve_deserializes_and_flattens_extra_grader_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals/eval_1/runs/run_1/output_items/item_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "item_1",
                "created_at": 1700000000_i64,
                "datasource_item": {"question": "What is 2+2?"},
                "datasource_item_id": 0,
                "eval_id": "eval_1",
                "object": "eval.run.output_item",
                "results": [{
                    "name": "string-check-grader",
                    "passed": true,
                    "score": 1.0,
                    "type": "string-check-grader",
                    "custom_grader_field": "unmodeled"
                }],
                "run_id": "run_1",
                "sample": {
                    "error": {"code": "", "message": ""},
                    "finish_reason": "stop",
                    "input": [{"content": "What is 2+2?", "role": "user"}],
                    "max_completion_tokens": 100,
                    "model": "gpt-4o-mini",
                    "output": [{"content": "4", "role": "assistant"}],
                    "seed": 42,
                    "temperature": 1.0,
                    "top_p": 1.0,
                    "usage": {
                        "cached_tokens": 0,
                        "completion_tokens": 1,
                        "prompt_tokens": 10,
                        "total_tokens": 11
                    }
                },
                "status": "completed"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let result = client
            .eval("eval_1")
            .run("run_1")
            .output_items()
            .item("item_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.results[0].name, "string-check-grader");
        assert_eq!(
            result.results[0].extra.get("custom_grader_field"),
            Some(&serde_json::json!("unmodeled"))
        );
        assert_eq!(result.sample.output[0].content.as_deref(), Some("4"));
    }
}
