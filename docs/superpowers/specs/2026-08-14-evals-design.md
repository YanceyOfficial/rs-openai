# Evals — Core CRUD (Runs deferred), Graders Documented as Embedded-Only — Design

Status: Approved
Date: 2026-08-14

## Context

Sixth of the 9 Phase 2a scaffolds to go from skeleton to working implementation. Evals let a caller define a repeatable test — a schema for test data (`data_source_config`) plus a set of grading rules (`testing_criteria`) — then (in a later slice) execute it against real data via a Run. Phase 2a scaffolded `apis::evals::{mod.rs, runs.rs}` (three-tier: `Evals`/`Eval`/`EvalRuns`/`EvalRun`) and a separate `apis::graders::Graders` top-level accessor, both zero-method.

Live research this session (WebFetch, `developers.openai.com` reference pages) fully and precisely confirmed the Eval's own definition: `data_source_config` is a 3-variant union (`custom`/`logs`/`stored_completions`, the last explicitly marked deprecated in favor of `logs`), and `testing_criteria` is a 5-variant grader union (`string_check`/`text_similarity`/`python`/`score_model`/`label_model`), every variant's fields confirmed exhaustively including `text_similarity`'s 11-value `evaluation_metric` enum and `score_model`'s nested `sampling_params`.

**Two scope decisions confirmed via `AskUserQuestion`**: (1) Eval Runs are deferred entirely — despite several research attempts, the run's own `data_source` union (how to actually execute an eval: which model, which input template, which data source) and its response shape (`status`, `result_counts`, `per_testing_criteria_results`, `report_url`) stayed genuinely unconfirmed, the same class of research gap ChatKit's Thread Items hit. `apis::evals::runs` stays an untouched, zero-method scaffold. (2) A related, notable finding: research confirms there is **no standalone `/v1/graders` REST endpoint at all** — graders exist exclusively as the embedded `testing_criteria` union above, never as an independently callable resource. `apis::graders::Graders` is left as a documented dead-end stub (doc comment updated to state this plainly) rather than removed or restructured, since Phase 2a scaffolded it as a top-level category and this project's convention is to never remove/restructure without being asked.

## Goals

- **Evals**: `Evals::create()` (`POST /evals`), `Evals::list()` (`GET /evals`), `Eval::retrieve()` (`GET /evals/{id}`), `Eval::update()` (`POST /evals/{id}`), `Eval::delete()` (`DELETE /evals/{id}`).
- `EvalDataSourceConfig` (3-variant tagged union, `#[serde(tag = "type", rename_all = "snake_case")]`):
  - `Custom(EvalCustomDataSourceConfig)`: `item_schema: serde_json::Value` (a JSON Schema object — same modeling choice already used for `ResponseFunctionTool.parameters`), `include_sample_schema: Option<bool>`.
  - `Logs(EvalLogsDataSourceConfig)`: `metadata: Option<HashMap<String, serde_json::Value>>` (confirmed as "map[unknown]", not all-string — same reasoning as ChatKit's `state_variables`).
  - `StoredCompletions(EvalStoredCompletionsDataSourceConfig)`: same shape as `Logs`, doc-commented as deprecated per OpenAI's own docs.
- `EvalGrader` (5-variant tagged union, `#[serde(tag = "type", rename_all = "snake_case")]`):
  - `StringCheck(EvalStringCheckGrader)`: `name: String`, `input: String`, `reference: String`, `operation: EvalStringCheckOperation` (4-value enum: `Eq`/`Ne`/`Like`/`Ilike`).
  - `TextSimilarity(EvalTextSimilarityGrader)`: `name: String`, `input: String`, `reference: String`, `evaluation_metric: EvalTextSimilarityMetric` (11-value enum with explicit per-variant renames — `cosine`/`fuzzy_match`/`bleu`/`gleu`/`meteor`/`rouge_1`/`rouge_2`/`rouge_3`/`rouge_4`/`rouge_5`/`rouge_l` — the `rouge_N` values deliberately do NOT use `rename_all` since serde's automatic snake_case conversion has no reliable, confirmed behavior for trailing digits like `Rouge1`; every variant gets an explicit `#[serde(rename = "...")]` instead, avoiding that ambiguity entirely).
  - `Python(EvalPythonGrader)`: `name: String`, `source: String` (the Python script body), `image_tag: Option<String>`.
  - `ScoreModel(EvalScoreModelGrader)`: `name: String`, `model: String`, `input: Vec<EvalGraderMessage>`, `range: Option<Vec<f32>>` (defaults to `[0, 1]` per docs), `sampling_params: Option<EvalScoreModelSamplingParams>` (`max_completions_tokens: Option<u32>`, `reasoning_effort: Option<String>`, `seed: Option<i64>`, `temperature: Option<f32>`, `top_p: Option<f32>`).
  - `LabelModel(EvalLabelModelGrader)`: `name: String`, `input: Vec<EvalGraderMessage>`, `labels: Vec<String>`, `model: String`, `passing_labels: Vec<String>`.
- `EvalGraderMessage`: `content: serde_json::Value` (confirmed to support text/image/audio, but the exact multimodal-content shape wasn't confirmed — kept as raw JSON, same under-typing precedent as `file_search`'s `filters`), `role: String`, `kind: String` (`#[serde(rename = "type")]`, the established convention for a wire-`"type"`-colliding field).
- `CreateEvalRequest`: `name: Option<String>`, `metadata: Option<HashMap<String, String>>` (confirmed all-string, unlike `data_source_config`'s metadata — "keys max 64 chars, values max 512 chars", matching this crate's standard metadata convention), `data_source_config: EvalDataSourceConfig` (required), `testing_criteria: Vec<EvalGrader>` (required).
- `Eval` (response): `id: String`, `object: String`, `created_at: i64`, `name: Option<String>`, `metadata: Option<HashMap<String, String>>`, `data_source_config: EvalDataSourceConfig`, `testing_criteria: Vec<EvalGrader>`.
- `UpdateEvalRequest`: `name: Option<String>`, `metadata: Option<HashMap<String, String>>` — matches this crate's universal update-request convention (only name/metadata mutable after creation; `data_source_config`/`testing_criteria` are immutable, same pattern as every other `Update*Request` in this crate).
- `ListEvalsRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>` — the crate's standard list-params trio; not independently confirmed for this specific endpoint (the fetched reference didn't enumerate query params), but consistent with every other list endpoint in this crate.
- `ListEvalsResponse`/`DeleteEvalResponse`: the crate's universal list-response/delete-response shapes.
- **Graders**: `apis::graders::Graders`'s doc comment updated to state plainly that no standalone API exists — graders are modeled entirely within `EvalGrader` above. No methods added; the struct remains functionally a permanent stub.

## Non-goals (explicit, deferred to later slices)

- **Eval Runs** (`apis::evals::runs`) — the run's own `data_source` union and response shape (`status`/`result_counts`/`per_testing_criteria_results`/`report_url`) were never confirmed despite several research attempts. Stays a zero-method scaffold.
- **`EvalGraderMessage.content`'s exact multimodal shape** — kept as raw JSON.
- **`data_source_config`'s alternate object-filter forms, if any exist beyond what's confirmed** — only the 3 confirmed variants are modeled.

## Design

### File organization

New file `rs_openai/src/interfaces/evals.rs` (registered via `pub mod evals;` in `interfaces/mod.rs`) — holds every Eval type in one file, matching the established single-flat-file convention. No `interfaces/graders.rs` — graders are modeled as `EvalGrader` and friends inside `interfaces/evals.rs`, since they only ever exist embedded within an eval (not a standalone resource with its own request/response types).

### Types

```rust
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

### Naming collision

`interfaces::evals::Eval` (response type) vs `apis::evals::Eval<'a>` (ID-scoped accessor) — the crate's now seventh instance of this exact, already-established pattern, resolved the same way (qualified paths inside `apis::evals`'s own method signatures).

## Testing

`wiremock` tests covering: `create` serializing a full `CreateEvalRequest` with `data_source_config: Custom` and two `testing_criteria` entries (`StringCheck` and `TextSimilarity`, proving the 5-variant grader union discriminates correctly and the `rouge_1`-style explicit renames serialize to the exact confirmed wire value) and deserializing the `Eval` response; a second `create` test with `data_source_config: Logs`; a `create` test covering `Python`/`ScoreModel`/`LabelModel` graders (proving all 5 variants round-trip, including `ScoreModel`'s nested `sampling_params`); `list` serializing query params and deserializing a paginated response; `retrieve`; `update` serializing only `name`/`metadata`; `delete`.

## Versioning

Purely additive at the crate level — new module, new types, no client method changes. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
