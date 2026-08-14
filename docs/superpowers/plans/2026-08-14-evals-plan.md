# Evals Core CRUD Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Evals' core CRUD (create/list/retrieve/update/delete) with the full, confirmed `data_source_config` union (3 variants) and `testing_criteria` grader union (5 variants) on top of Phase 2a's zero-method scaffold. Document `apis::graders::Graders` as a permanent, honest dead-end stub — research confirmed no standalone `/v1/graders` endpoint exists.

**Architecture:** New file `rs_openai/src/interfaces/evals.rs` holds every Eval type (data source configs, all 5 grader types, request/response types) in one flat file, matching the established convention. Real methods get added to the existing `apis::evals::{Evals, Eval}` scaffold. `apis::evals::runs` and `apis::graders` are explicitly OUT of scope — the former stays untouched (deferred, unconfirmed shape), the latter gets only a doc-comment update (no methods, since none exist to add).

**Tech Stack:** Rust, `serde` (`#[serde(tag = "type", rename_all = "snake_case")]` for the two extensible unions), `bon::Builder`, `wiremock`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-14-evals-design.md`. Two scope decisions confirmed via `AskUserQuestion`: Eval Runs deferred entirely (`apis::evals::runs` stays untouched); Graders documented as embedded-only (no standalone endpoint exists — `apis::graders::Graders` gets a doc-comment fix only, no methods).
- `EvalTextSimilarityMetric`'s 11 variants each need an EXPLICIT `#[serde(rename = "...")]` — do NOT use `rename_all` for this enum. Serde's automatic case conversion has no confirmed, reliable behavior for variant names ending in digits (`Rouge1` → `rouge_1`, NOT the unconfirmed `rouge1`) — explicit renames sidestep the ambiguity entirely for every variant, not just the digit-suffixed ones.
- `EvalGraderMessage.content` stays `serde_json::Value` — the exact multimodal (text/image/audio) content shape was not confirmed by research.
- `EvalLogsDataSourceConfig.metadata`/`EvalStoredCompletionsDataSourceConfig.metadata` are `Option<HashMap<String, serde_json::Value>>` (confirmed "map[unknown]", not all-string) — do NOT confuse these with `CreateEvalRequest.metadata`/`UpdateEvalRequest.metadata`, which ARE `Option<HashMap<String, String>>` (confirmed all-string, the crate's standard metadata convention). These are two genuinely different types on two different structs — do not conflate them.
- `UpdateEvalRequest` has ONLY `name`/`metadata` — matches this crate's universal update-request convention (`data_source_config`/`testing_criteria` are immutable after creation, not exposed on the update type at all).
- Naming collision (`interfaces::evals::Eval` vs `apis::evals::Eval<'a>`) resolved via qualified paths, not renaming — the crate's now seventh instance of this established pattern.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: Type foundation (data_source_config + grader unions) and `create`

**Files:**
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod evals;`)
- Create: `rs_openai/src/interfaces/evals.rs` (all data-source-config and grader types, plus `CreateEvalRequest`/`Eval` — the rest of the CRUD types come in Task 2)
- Modify: `rs_openai/src/apis/evals/mod.rs` (add `create` method to `Evals`, remove stale `#[allow(dead_code)]`, update doc comments)
- Test: `rs_openai/src/apis/evals/mod.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::evals::{Evals, Eval}` struct shapes from Phase 2a (`Evals<'a> { openai: &'a OpenAI }`, `Eval<'a> { openai: &'a OpenAI, id: String }`); existing `crate::interfaces::chat::SortOrder` (used by Task 2's `ListEvalsRequest`, not this task).
- Produces: `interfaces::evals::{EvalCustomDataSourceConfig, EvalLogsDataSourceConfig, EvalStoredCompletionsDataSourceConfig, EvalDataSourceConfig, EvalStringCheckOperation, EvalStringCheckGrader, EvalTextSimilarityMetric, EvalTextSimilarityGrader, EvalPythonGrader, EvalGraderMessage, EvalScoreModelSamplingParams, EvalScoreModelGrader, EvalLabelModelGrader, EvalGrader, CreateEvalRequest, Eval}`, `Evals::create` — Task 2 depends on `EvalDataSourceConfig`/`EvalGrader`/`Eval` (used by `ListEvalsResponse`/the retrieve/update return types).

- [ ] **Step 1: Write the failing test — `create` serializes `Custom` data source + `StringCheck`/`TextSimilarity` graders**

Add a test module to `rs_openai/src/apis/evals/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::evals::{
        CreateEvalRequest, EvalCustomDataSourceConfig, EvalDataSourceConfig, EvalGrader,
        EvalStringCheckGrader, EvalStringCheckOperation, EvalTextSimilarityGrader,
        EvalTextSimilarityMetric,
    };
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_custom_data_source_with_string_check_and_text_similarity_graders()
     {
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
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_custom_data_source_with_string_check_and_text_similarity_graders 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::evals` module doesn't exist yet, `client.evals().create(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/evals.rs` with the full type set**

```rust
//! Evaluation definitions for grading model outputs against a defined criteria set. Runs are
//! not yet implemented (their request/response shape was not confirmed by research) -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalCustomDataSourceConfig {
    pub item_schema: serde_json::Value,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_sample_schema: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalLogsDataSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

/// Deprecated by OpenAI in favor of `EvalLogsDataSourceConfig`; still a live wire type.
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalStoredCompletionsDataSourceConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvalDataSourceConfig {
    Custom(EvalCustomDataSourceConfig),
    Logs(EvalLogsDataSourceConfig),
    StoredCompletions(EvalStoredCompletionsDataSourceConfig),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum EvalStringCheckOperation {
    Eq,
    Ne,
    Like,
    Ilike,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalStringCheckGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub input: String,
    #[builder(into)]
    pub reference: String,
    pub operation: EvalStringCheckOperation,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum EvalTextSimilarityMetric {
    #[serde(rename = "cosine")]
    Cosine,
    #[serde(rename = "fuzzy_match")]
    FuzzyMatch,
    #[serde(rename = "bleu")]
    Bleu,
    #[serde(rename = "gleu")]
    Gleu,
    #[serde(rename = "meteor")]
    Meteor,
    #[serde(rename = "rouge_1")]
    Rouge1,
    #[serde(rename = "rouge_2")]
    Rouge2,
    #[serde(rename = "rouge_3")]
    Rouge3,
    #[serde(rename = "rouge_4")]
    Rouge4,
    #[serde(rename = "rouge_5")]
    Rouge5,
    #[serde(rename = "rouge_l")]
    RougeL,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalTextSimilarityGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub input: String,
    #[builder(into)]
    pub reference: String,
    pub evaluation_metric: EvalTextSimilarityMetric,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalPythonGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub source: String,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_tag: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalGraderMessage {
    pub content: serde_json::Value,
    #[builder(into)]
    pub role: String,
    #[builder(into)]
    #[serde(rename = "type")]
    pub kind: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalScoreModelSamplingParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_completions_tokens: Option<u32>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_effort: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalScoreModelGrader {
    #[builder(into)]
    pub name: String,
    #[builder(into)]
    pub model: String,
    pub input: Vec<EvalGraderMessage>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<Vec<f32>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sampling_params: Option<EvalScoreModelSamplingParams>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct EvalLabelModelGrader {
    #[builder(into)]
    pub name: String,
    pub input: Vec<EvalGraderMessage>,
    pub labels: Vec<String>,
    #[builder(into)]
    pub model: String,
    pub passing_labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EvalGrader {
    StringCheck(EvalStringCheckGrader),
    TextSimilarity(EvalTextSimilarityGrader),
    Python(EvalPythonGrader),
    ScoreModel(EvalScoreModelGrader),
    LabelModel(EvalLabelModelGrader),
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateEvalRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,

    pub data_source_config: EvalDataSourceConfig,
    pub testing_criteria: Vec<EvalGrader>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Eval {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub name: Option<String>,
    pub metadata: Option<std::collections::HashMap<String, String>>,
    pub data_source_config: EvalDataSourceConfig,
    pub testing_criteria: Vec<EvalGrader>,
}
```

- [ ] **Step 4: Register the new module**

In `rs_openai/src/interfaces/mod.rs`, add this new line alphabetically, between the existing `pub mod embeddings;` and `pub mod files;` lines:

```rust
pub mod evals;
```

- [ ] **Step 5: Implement `Evals::create`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/evals/mod.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
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
}

pub struct Eval<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> Eval<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
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
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_custom_data_source_with_string_check_and_text_similarity_graders 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing test — `create` covers the remaining 3 grader types and `Logs` data source**

Add to the same test module:

```rust
    #[tokio::test]
    async fn create_serializes_logs_data_source_with_python_score_model_and_label_model_graders()
     {
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
```

- [ ] **Step 8: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_logs_data_source_with_python_score_model_and_label_model_graders 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 9: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (172 + 2 new = 174).

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/evals.rs rs_openai/src/apis/evals/mod.rs
git commit -m "feat(evals): add eval creation with full data_source_config and grader unions"
```

---

### Task 2: `list`/`retrieve`/`update`/`delete` + Graders doc-comment fix

**Files:**
- Modify: `rs_openai/src/interfaces/evals.rs` (add `ListEvalsRequest`/`ListEvalsResponse`/`UpdateEvalRequest`/`DeleteEvalResponse` to the same file, after `Eval`)
- Modify: `rs_openai/src/apis/evals/mod.rs` (add `list` to `Evals`; add `retrieve`/`update`/`delete` to `Eval`)
- Modify: `rs_openai/src/apis/graders.rs` (doc-comment fix only — no methods)
- Test: `rs_openai/src/apis/evals/mod.rs`

**Interfaces:**
- Consumes: `EvalDataSourceConfig`/`EvalGrader`/`Eval` from Task 1; existing `crate::interfaces::chat::SortOrder`.
- Produces: `interfaces::evals::{ListEvalsRequest, ListEvalsResponse, UpdateEvalRequest, DeleteEvalResponse}`, `Evals::list`, `Eval::retrieve`/`::update`/`::delete` — no later task in this plan depends on these; the final cross-check does.

- [ ] **Step 1: Write the failing test — `list` serializes query params and deserializes a paginated response**

Add to the same test module in `rs_openai/src/apis/evals/mod.rs`:

```rust
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
```

Add `ListEvalsRequest` to the existing `use crate::interfaces::evals::{...}` import block, and add `query_param` to the existing `use wiremock::matchers::{...}` import.

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_query_params_and_deserializes_response 2>&1 | tail -30`
Expected: FAIL to compile — `ListEvalsRequest` doesn't exist, `client.evals().list(...)` doesn't exist.

- [ ] **Step 3: Add the remaining types to `rs_openai/src/interfaces/evals.rs`**

Append to the end of `rs_openai/src/interfaces/evals.rs` (after the `Eval` struct from Task 1):

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct UpdateEvalRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListEvalsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListEvalsResponse {
    pub object: String,
    pub data: Vec<Eval>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteEvalResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

- [ ] **Step 4: Implement `Evals::list` and `Eval::retrieve`/`::update`/`::delete`**

In `rs_openai/src/apis/evals/mod.rs`, modify `impl<'a> Evals<'a>` to add `list` after `create`:

```rust
    /// Lists eval definitions.
    pub async fn list(
        &self,
        req: &evals::ListEvalsRequest,
    ) -> OpenAIResponse<evals::ListEvalsResponse> {
        self.openai.get("/evals", req).await
    }
```

And modify `impl<'a> Eval<'a>` to add `retrieve`/`update`/`delete` before the existing `runs`/`run` methods:

```rust
    /// Retrieves an eval by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<evals::Eval> {
        self.openai.get(&format!("/evals/{}", self.id), &()).await
    }

    /// Updates an eval's name and/or metadata.
    pub async fn update(&self, req: &evals::UpdateEvalRequest) -> OpenAIResponse<evals::Eval> {
        self.openai
            .post(&format!("/evals/{}", self.id), req)
            .await
    }

    /// Deletes an eval.
    pub async fn delete(&self) -> OpenAIResponse<evals::DeleteEvalResponse> {
        self.openai
            .delete(&format!("/evals/{}", self.id), &())
            .await
    }
```

- [ ] **Step 5: Fix `apis::graders`'s doc comment**

Replace the full contents of `rs_openai/src/apis/graders.rs`:

```rust
//! Grading logic used by Evals. There is no standalone `/graders` REST endpoint -- graders
//! only ever exist embedded within an eval's `testing_criteria` array (see
//! `crate::interfaces::evals::EvalGrader` and its 5 variants:
//! `StringCheck`/`TextSimilarity`/`Python`/`ScoreModel`/`LabelModel`). This accessor is kept
//! for API-surface completeness (matching Phase 2a's original scaffold) but intentionally has
//! no methods, since none exist on OpenAI's side to call -- see
//! docs/superpowers/specs/2026-08-14-evals-design.md.

use crate::client::OpenAI;

#[allow(dead_code)]
pub struct Graders<'a> {
    openai: &'a OpenAI,
}

impl<'a> Graders<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_serializes_query_params_and_deserializes_response 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 7: Write the failing tests for `retrieve`, `update`, `delete`**

Add to the same test module:

```rust
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
```

Add `UpdateEvalRequest` to the existing import block.

- [ ] **Step 8: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai retrieve_deserializes_eval update_serializes_only_name_and_metadata delete_deserializes_confirmation 2>&1 | tail -30`
Expected: all PASS.

- [ ] **Step 9: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (174 + 4 new = 178).

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/evals.rs rs_openai/src/apis/evals/mod.rs rs_openai/src/apis/graders.rs
git commit -m "feat(evals): add list/retrieve/update/delete; document graders as embedded-only"
```

---

### Task 3: Final cross-check

**Files:**
- No new code changes expected — this task verifies the whole slice, and only touches files if it finds a real gap.

**Interfaces:**
- Consumes: everything from Task 1 and Task 2.
- Produces: nothing new — verification-only, same role as the final task in every prior Phase 2b slice's plan.

- [ ] **Step 1: Run the full test suite, clippy, and build**

Run:
```bash
cargo fmt --check
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: fmt clean (no diff), clippy clean, build clean, 178/178 tests passing.

- [ ] **Step 2: Manually verify scope and cleanup, reading the actual files (don't just trust the step count)**

- `rs_openai/src/apis/evals/mod.rs`: `Evals` struct has NO `#[allow(dead_code)]` (has real methods now).
- `rs_openai/src/apis/evals/runs.rs`: completely untouched — still zero methods, `#[allow(dead_code)]` still present on both `EvalRuns` and `EvalRun` (deferred, per scope).
- `rs_openai/src/apis/graders.rs`: still zero methods, `#[allow(dead_code)]` still present, but doc comment now accurately explains why (no standalone endpoint exists).
- `rs_openai/src/interfaces/evals.rs` defines exactly these 20 types: `EvalCustomDataSourceConfig`, `EvalLogsDataSourceConfig`, `EvalStoredCompletionsDataSourceConfig`, `EvalDataSourceConfig`, `EvalStringCheckOperation`, `EvalStringCheckGrader`, `EvalTextSimilarityMetric`, `EvalTextSimilarityGrader`, `EvalPythonGrader`, `EvalGraderMessage`, `EvalScoreModelSamplingParams`, `EvalScoreModelGrader`, `EvalLabelModelGrader`, `EvalGrader`, `CreateEvalRequest`, `Eval`, `UpdateEvalRequest`, `ListEvalsRequest`, `ListEvalsResponse`, `DeleteEvalResponse` — count these against the actual file, not this list (this note itself could drift).
- `EvalTextSimilarityMetric` has no `rename_all` attribute — every variant has its own explicit `#[serde(rename = ...)]` (scope check, since this was a deliberately reasoned decision, not an oversight to "fix" toward `rename_all`).
- `interfaces::graders` module does NOT exist — no such file was created (graders live entirely inside `interfaces::evals::EvalGrader`).

If any of these checks fail, fix it directly and re-run Step 1.

- [ ] **Step 3: No commit needed for this task if Step 2 found nothing to fix** (if it did, commit the fix with an explicit-path `git add` and a message describing what was found).
