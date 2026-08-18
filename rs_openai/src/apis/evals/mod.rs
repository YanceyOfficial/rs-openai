//! Evaluation definitions for grading model outputs against a defined criteria set -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md.

pub mod runs;

use crate::client::OpenAI;
use crate::interfaces::evals;
pub use crate::interfaces::evals::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct Evals<'a> {
    openai: &'a OpenAI,
}

impl<'a> Evals<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an eval definition.
    pub async fn create(&self, req: &evals::CreateEvalRequest) -> OpenAIResponse<evals::Eval> {
        self.openai.post("/evals", req).await
    }

    /// Lists eval definitions.
    pub async fn list(
        &self,
        req: &evals::ListEvalsRequest,
    ) -> OpenAIResponse<evals::ListEvalsResponse> {
        self.openai.get("/evals", req).await
    }
}

pub struct Eval<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Eval<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves an eval by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<evals::Eval> {
        self.openai.get(&format!("/evals/{}", self.id), &()).await
    }

    /// Updates an eval's name and/or metadata.
    pub async fn update(&self, req: &evals::UpdateEvalRequest) -> OpenAIResponse<evals::Eval> {
        self.openai.post(&format!("/evals/{}", self.id), req).await
    }

    /// Deletes an eval.
    pub async fn delete(&self) -> OpenAIResponse<evals::DeleteEvalResponse> {
        self.openai
            .delete(&format!("/evals/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this eval's runs. Not yet implemented -- the run's request/response
    /// shape was not confirmed by research; see the design spec's Non-goals.
    pub fn runs(&self) -> runs::EvalRuns<'a> {
        runs::EvalRuns::new(self.openai, self.id.clone())
    }

    /// Scoped accessor for a specific run of this eval. Not yet implemented -- see `runs()`.
    pub fn run(&self, run_id: impl Into<String>) -> runs::EvalRun<'a> {
        runs::EvalRun::new(self.openai, self.id.clone(), run_id.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::evals::{
        CreateEvalRequest, EvalCustomDataSourceConfig, EvalDataSourceConfig, EvalGrader,
        EvalStringCheckGrader, EvalStringCheckOperation, EvalTextSimilarityGrader,
        EvalTextSimilarityMetric, ListEvalsRequest, UpdateEvalRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_custom_data_source_with_string_check_and_text_similarity_graders() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/evals"))
            .and(body_json(serde_json::json!({
                "data_source_config": {
                    "type": "custom",
                    "item_schema": {
                        "type": "object",
                        "properties": {"ticket_text": {"type": "string"}, "correct_label": {"type": "string"}},
                        "required": ["ticket_text", "correct_label"]
                    },
                    "include_sample_schema": true
                },
                "testing_criteria": [
                    {
                        "type": "string_check",
                        "name": "Exact match",
                        "input": "{{ sample.output_text }}",
                        "reference": "{{ item.correct_label }}",
                        "operation": "eq"
                    },
                    {
                        "type": "text_similarity",
                        "name": "Rouge check",
                        "input": "{{ sample.output_text }}",
                        "reference": "{{ item.correct_label }}",
                        "evaluation_metric": "rouge_1"
                    }
                ]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "eval_1",
                "object": "eval",
                "created_at": 1700000000_i64,
                "name": null,
                "metadata": null,
                "data_source_config": {
                    "type": "custom",
                    "item_schema": {"type": "object"},
                    "include_sample_schema": true
                },
                "testing_criteria": [
                    {
                        "type": "string_check",
                        "name": "Exact match",
                        "input": "{{ sample.output_text }}",
                        "reference": "{{ item.correct_label }}",
                        "operation": "eq"
                    }
                ]
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEvalRequest::builder()
            .data_source_config(EvalDataSourceConfig::Custom(
                EvalCustomDataSourceConfig::builder()
                    .item_schema(serde_json::json!({
                        "type": "object",
                        "properties": {"ticket_text": {"type": "string"}, "correct_label": {"type": "string"}},
                        "required": ["ticket_text", "correct_label"]
                    }))
                    .include_sample_schema(true)
                    .build(),
            ))
            .testing_criteria(vec![
                EvalGrader::StringCheck(
                    EvalStringCheckGrader::builder()
                        .name("Exact match")
                        .input("{{ sample.output_text }}")
                        .reference("{{ item.correct_label }}")
                        .operation(EvalStringCheckOperation::Eq)
                        .build(),
                ),
                EvalGrader::TextSimilarity(
                    EvalTextSimilarityGrader::builder()
                        .name("Rouge check")
                        .input("{{ sample.output_text }}")
                        .reference("{{ item.correct_label }}")
                        .evaluation_metric(EvalTextSimilarityMetric::Rouge1)
                        .build(),
                ),
            ])
            .build();

        let res = client.evals().create(&req).await.unwrap();
        assert_eq!(res.id, "eval_1");
        assert_eq!(res.testing_criteria.len(), 1);
    }

    #[tokio::test]
    async fn create_serializes_logs_data_source_with_python_score_model_and_label_model_graders() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/evals"))
            .and(body_json(serde_json::json!({
                "data_source_config": {"type": "logs"},
                "testing_criteria": [
                    {
                        "type": "python",
                        "name": "Custom check",
                        "source": "def grade(sample, item):\n    return 1.0"
                    },
                    {
                        "type": "score_model",
                        "name": "LLM judge",
                        "model": "gpt-5-mini",
                        "input": [{"role": "user", "type": "message", "content": "Grade this."}],
                        "range": [0.0, 1.0]
                    },
                    {
                        "type": "label_model",
                        "name": "Label check",
                        "model": "gpt-5-mini",
                        "input": [{"role": "user", "type": "message", "content": "Label this."}],
                        "labels": ["good", "bad"],
                        "passing_labels": ["good"]
                    }
                ]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "eval_2",
                "object": "eval",
                "created_at": 1700000000_i64,
                "name": "Multi-grader eval",
                "metadata": null,
                "data_source_config": {"type": "logs", "metadata": null},
                "testing_criteria": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateEvalRequest::builder()
            .data_source_config(EvalDataSourceConfig::Logs(
                crate::interfaces::evals::EvalLogsDataSourceConfig::builder().build(),
            ))
            .testing_criteria(vec![
                EvalGrader::Python(
                    crate::interfaces::evals::EvalPythonGrader::builder()
                        .name("Custom check")
                        .source("def grade(sample, item):\n    return 1.0")
                        .build(),
                ),
                EvalGrader::ScoreModel(
                    crate::interfaces::evals::EvalScoreModelGrader::builder()
                        .name("LLM judge")
                        .model("gpt-5-mini")
                        .input(vec![
                            crate::interfaces::evals::EvalGraderMessage::builder()
                                .role("user")
                                .kind("message")
                                .content(serde_json::json!("Grade this."))
                                .build(),
                        ])
                        .range(vec![0.0, 1.0])
                        .build(),
                ),
                EvalGrader::LabelModel(
                    crate::interfaces::evals::EvalLabelModelGrader::builder()
                        .name("Label check")
                        .model("gpt-5-mini")
                        .input(vec![
                            crate::interfaces::evals::EvalGraderMessage::builder()
                                .role("user")
                                .kind("message")
                                .content(serde_json::json!("Label this."))
                                .build(),
                        ])
                        .labels(vec!["good".to_string(), "bad".to_string()])
                        .passing_labels(vec!["good".to_string()])
                        .build(),
                ),
            ])
            .build();

        let res = client.evals().create(&req).await.unwrap();
        assert_eq!(res.id, "eval_2");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "eval_3",
                    "object": "eval",
                    "created_at": 1700000000_i64,
                    "name": "My eval",
                    "metadata": null,
                    "data_source_config": {"type": "custom", "item_schema": {"type": "object"}, "include_sample_schema": null},
                    "testing_criteria": []
                }],
                "first_id": "eval_3",
                "last_id": "eval_3",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = ListEvalsRequest::builder().limit(5).build();
        let res = client.evals().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert_eq!(res.data[0].id, "eval_3");
    }

    #[tokio::test]
    async fn retrieve_deserializes_eval() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/evals/eval_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "eval_4",
                "object": "eval",
                "created_at": 1700000000_i64,
                "name": "Retrieved eval",
                "metadata": null,
                "data_source_config": {"type": "custom", "item_schema": {"type": "object"}, "include_sample_schema": null},
                "testing_criteria": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.eval("eval_4").retrieve().await.unwrap();
        assert_eq!(res.id, "eval_4");
    }

    #[tokio::test]
    async fn update_serializes_only_name_and_metadata() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/evals/eval_4"))
            .and(body_json(serde_json::json!({"name": "Renamed eval"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "eval_4",
                "object": "eval",
                "created_at": 1700000000_i64,
                "name": "Renamed eval",
                "metadata": null,
                "data_source_config": {"type": "custom", "item_schema": {"type": "object"}, "include_sample_schema": null},
                "testing_criteria": []
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = UpdateEvalRequest::builder().name("Renamed eval").build();
        let res = client.eval("eval_4").update(&req).await.unwrap();
        assert_eq!(res.name.as_deref(), Some("Renamed eval"));
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/evals/eval_4"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "eval_4",
                "object": "eval.deleted",
                "deleted": true
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let res = client.eval("eval_4").delete().await.unwrap();
        assert!(res.deleted);
    }
}
