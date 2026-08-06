//! Manage fine-tuning jobs to tailor a model to your specific training data. Related guide: [Fine-tune models](https://platform.openai.com/docs/guides/fine-tuning)

pub mod checkpoints;

use crate::client::OpenAI;
use crate::interfaces::fine_tuning;
pub use crate::interfaces::fine_tuning::*;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct FineTuning<'a> {
    openai: &'a OpenAI,
}

impl<'a> FineTuning<'a> {
    pub fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates a job that fine-tunes a specified model from a given dataset.
    ///
    /// OpenAIResponse includes details of the enqueued job including job status and the name of the fine-tuned models once complete.
    ///
    /// [Learn more about Fine-tuning](https://platform.openai.com/docs/guides/fine-tuning)
    pub async fn create(
        &self,
        req: &fine_tuning::CreateFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai.post("/fine-tuning/jobs", req).await
    }

    /// List your organization's fine-tuning jobs
    pub async fn list(
        &self,
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningEventResponse> {
        self.openai.get("/fine-tuning/jobs", req).await
    }

    /// Get status updates for a fine-tuning job.
    pub async fn list_events(
        &self,
        fine_tuning_job_id: &str, // The ID of the fine-tuning job to get events for.
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningEventResponse> {
        self.openai
            .get(
                &format!("/fine-tuning/jobs/{fine_tuning_job_id}/events"),
                req,
            )
            .await
    }

    /// Get status updates for a fine-tuning job.
    pub async fn list_checkpoints(
        &self,
        fine_tuning_job_id: &str, // The ID of the fine-tuning job to get checkpoints for.
        req: &fine_tuning::ListFineTuningRequest,
    ) -> OpenAIResponse<fine_tuning::FineTuningCheckpointResponse> {
        self.openai
            .get(
                &format!("/fine-tuning/jobs/{fine_tuning_job_id}/checkpoints"),
                req,
            )
            .await
    }

    /// Gets info about the fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_id` - The ID of the fine-tune job
    ///
    /// [Learn more about Fine-tuning](https://platform.openai.com/docs/guides/fine-tuning)
    pub async fn retrieve(
        &self,
        fine_tuning_job_id: &str,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai
            .get(&format!("/fine-tuning/jobs/{fine_tuning_job_id}"), &())
            .await
    }

    /// Immediately cancel a fine-tune job.
    ///
    /// # Path parameters
    ///
    /// - `fine_tune_job_id` - The ID of the fine-tune job to cancel
    pub async fn cancel(
        &self,
        fine_tune_job_id: &str,
    ) -> OpenAIResponse<fine_tuning::FineTuningResponse> {
        self.openai
            .post(&format!("/fine-tuning/jobs/{fine_tune_job_id}/cancel"), &())
            .await
    }

    /// Scoped accessor for a specific fine-tuning checkpoint's operations (e.g. sharing
    /// permissions). Not yet implemented — see
    /// docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md.
    pub fn checkpoint(&self, id: impl Into<String>) -> checkpoints::FineTuningCheckpoint<'a> {
        checkpoints::FineTuningCheckpoint::new(self.openai, id.into())
    }
}

#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::fine_tuning::{
        CreateFineTuningRequest, DpoHyperparameters, DpoMethod, FineTuningMethod,
        FineTuningMethodType, Grader, GraderMessageRole, MultiGrader, PythonGrader,
        ReinforcementMethod, ScoreModelGrader, StringCheckGrader, StringCheckOperation,
        TextSimilarityGrader, TextSimilarityMetric,
    };
    use crate::shared::response_wrapper::OpenAIError;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "davinci",
                "training_file": "file-abc"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-123",
                "model": "davinci",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "queued",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("davinci")
            .training_file("file-abc")
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        assert_eq!(res.id, "ftjob-123");
    }

    #[tokio::test]
    async fn create_maps_api_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": {
                    "message": "training_file is required",
                    "type": "invalid_request_error",
                    "param": "training_file",
                    "code": null
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("davinci")
            .training_file("")
            .build();

        let err = client.fine_tuning().create(&req).await.unwrap_err();
        match err {
            OpenAIError::ApiError(e) => {
                assert_eq!(e.error.param, Some(serde_json::json!("training_file")));
            }
            other => panic!("expected ApiError, got {other:?}"),
        }
    }

    #[test]
    fn grader_string_check_serializes_with_type_tag() {
        let grader = Grader::StringCheck(StringCheckGrader {
            name: "exact_match".to_string(),
            input: "{{sample.output}}".to_string(),
            reference: "{{item.expected}}".to_string(),
            operation: StringCheckOperation::Eq,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "string_check",
                "name": "exact_match",
                "input": "{{sample.output}}",
                "reference": "{{item.expected}}",
                "operation": "eq"
            })
        );
    }

    #[test]
    fn grader_text_similarity_serializes_numbered_rouge_variant_correctly() {
        let grader = Grader::TextSimilarity(TextSimilarityGrader {
            name: "similarity".to_string(),
            input: "{{sample.output}}".to_string(),
            reference: "{{item.expected}}".to_string(),
            evaluation_metric: TextSimilarityMetric::Rouge1,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(json["evaluation_metric"], serde_json::json!("rouge_1"));
    }

    #[test]
    fn grader_python_serializes_optional_image_tag_as_absent() {
        let grader = Grader::Python(PythonGrader {
            name: "custom".to_string(),
            source: "def grade(sample, item): return 1.0".to_string(),
            image_tag: None,
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "type": "python",
                "name": "custom",
                "source": "def grade(sample, item): return 1.0"
            })
        );
    }

    #[test]
    fn grader_score_model_deserializes_with_sampling_params() {
        let json = serde_json::json!({
            "type": "score_model",
            "name": "scorer",
            "model": "gpt-4o-mini",
            "input": [{"role": "user", "content": "grade this"}],
            "sampling_params": {"temperature": 0.0}
        });

        let grader: Grader = serde_json::from_value(json).unwrap();
        match grader {
            Grader::ScoreModel(ScoreModelGrader {
                model,
                input,
                sampling_params,
                ..
            }) => {
                assert_eq!(model, "gpt-4o-mini");
                assert_eq!(input[0].role, GraderMessageRole::User);
                assert_eq!(sampling_params.unwrap().temperature, Some(0.0));
            }
            other => panic!("expected ScoreModel, got {other:?}"),
        }
    }

    #[test]
    fn grader_multi_recursively_holds_other_graders() {
        let grader = Grader::Multi(MultiGrader {
            name: "combined".to_string(),
            graders: vec![
                Grader::StringCheck(StringCheckGrader {
                    name: "a".to_string(),
                    input: "x".to_string(),
                    reference: "y".to_string(),
                    operation: StringCheckOperation::Eq,
                }),
                Grader::Python(PythonGrader {
                    name: "b".to_string(),
                    source: "...".to_string(),
                    image_tag: None,
                }),
            ],
            calculate_output: "0.5 * a + 0.5 * b".to_string(),
        });

        let json = serde_json::to_value(&grader).unwrap();
        assert_eq!(json["graders"].as_array().unwrap().len(), 2);
        assert_eq!(
            json["graders"][0]["type"],
            serde_json::json!("string_check")
        );
        assert_eq!(json["graders"][1]["type"], serde_json::json!("python"));
    }

    #[tokio::test]
    async fn create_serializes_dpo_method_and_metadata() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "training_file": "file-abc",
                "method": {
                    "type": "dpo",
                    "dpo": {"hyperparameters": {"beta": "auto"}}
                },
                "metadata": {"team": "eval"}
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-dpo",
                "model": "gpt-4o-mini",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "queued",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null,
                "method": {"type": "dpo", "dpo": {"hyperparameters": {"beta": "auto"}}},
                "metadata": {"team": "eval"},
                "error": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let mut metadata = std::collections::HashMap::new();
        metadata.insert("team".to_string(), "eval".to_string());

        let req = CreateFineTuningRequest::builder()
            .model("gpt-4o-mini")
            .training_file("file-abc")
            .method(FineTuningMethod {
                r#type: FineTuningMethodType::Dpo,
                supervised: None,
                dpo: Some(DpoMethod {
                    hyperparameters: Some(DpoHyperparameters {
                        batch_size: None,
                        beta: Some(crate::interfaces::fine_tuning::AutoOrInteger::Auto(
                            "auto".to_string(),
                        )),
                        learning_rate_multiplier: None,
                        n_epochs: None,
                    }),
                }),
                reinforcement: None,
            })
            .metadata(metadata)
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        assert_eq!(res.id, "ftjob-dpo");
        assert!(res.error.is_none());
        assert_eq!(
            res.metadata.unwrap().get("team").map(String::as_str),
            Some("eval")
        );
    }

    #[tokio::test]
    async fn create_serializes_reinforcement_method_with_grader() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/fine-tuning/jobs"))
            .and(body_json(serde_json::json!({
                "model": "gpt-4o-mini",
                "training_file": "file-abc",
                "method": {
                    "type": "reinforcement",
                    "reinforcement": {
                        "grader": {
                            "type": "string_check",
                            "name": "exact",
                            "input": "{{sample.output}}",
                            "reference": "{{item.expected}}",
                            "operation": "eq"
                        }
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "fine_tuning.job",
                "id": "ftjob-rl",
                "model": "gpt-4o-mini",
                "created_at": 1700000000_u64,
                "finished_at": null,
                "fine_tuned_model": null,
                "organization_id": "org-123",
                "result_files": [],
                "status": "failed",
                "validation_file": null,
                "training_file": "file-abc",
                "trained_tokens": null,
                "hyperparameters": {"batch_size": "auto", "learning_rate_multiplier": "auto", "n_epochs": "auto"},
                "integrations": null,
                "seed": 42,
                "estimated_finish": null,
                "error": {"code": "invalid_grader", "message": "grader misconfigured", "param": "method.reinforcement.grader"}
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-test").with_base_url(mock_server.uri());
        let req = CreateFineTuningRequest::builder()
            .model("gpt-4o-mini")
            .training_file("file-abc")
            .method(FineTuningMethod {
                r#type: FineTuningMethodType::Reinforcement,
                supervised: None,
                dpo: None,
                reinforcement: Some(ReinforcementMethod {
                    grader: Grader::StringCheck(StringCheckGrader {
                        name: "exact".to_string(),
                        input: "{{sample.output}}".to_string(),
                        reference: "{{item.expected}}".to_string(),
                        operation: StringCheckOperation::Eq,
                    }),
                    hyperparameters: None,
                }),
            })
            .build();

        let res = client.fine_tuning().create(&req).await.unwrap();
        let error = res.error.expect("expected error to be present");
        assert_eq!(error.code, "invalid_grader");
        assert_eq!(error.param.as_deref(), Some("method.reinforcement.grader"));
    }
}
