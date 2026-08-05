# Existing Modules: Field-Level Fixes — Design

Status: Approved
Date: 2026-08-05

## Context

Phase 2b (filling in Phase 2a's scaffolds + applying the field-diff research) has so far shipped two narrow slices: Chat tool-calling (2026-08-04) and Images' `model` field + gpt-image capability set (2026-08-05). The project owner then gave explicit direction: **"你把所有 existing modules with issues 都一次搞定吧, 否则太分散了, 把这些都搞定后, 下一步我们就开始做未实现的部分"** — resolve every remaining gap in already-implemented modules in one consolidated pass, rather than one narrow slice at a time, and only then move on to implementing the 9 categories Phase 2a scaffolded but left empty.

This document covers every remaining gap identified in `docs/superpowers/research/2026-07-31-field-diff-chat-embeddings-moderations-models.md` and `docs/superpowers/research/2026-07-31-field-diff-images-audio-files-fine-tuning.md`, across 8 modules: Fine Tuning, Audio, Embeddings, Files, Uploads, Batches, Completions (legacy), Chat. Every field shape, enum value set, and endpoint below was live-verified against current OpenAI docs during design (not carried over unverified from the original research, which had several items marked `[verify]`/`[training]`).

**Flagship finding, not from the original research — a real, previously-undiscovered bug**: `Upload::upload()` (`rs_openai/src/apis/uploads.rs`) currently builds a multipart form (`file` + `purpose`) and posts it to `POST /v1/uploads`. The real endpoint is JSON-only and expects upload *metadata* (`bytes`, `filename`, `mime_type`, `purpose`) to open an upload session — actual file bytes are sent afterward via the separate, already-correctly-implemented `POST /uploads/{id}/parts` call (`add_upload_part`). Every call to today's `upload()` fails against the real API — either an unexpected-content-type rejection or a missing-required-field error. This is the same class of bug as Phase 1's `Files::add_upload` mislabeling and Images' `create_edit` mask-duplication bug: found by actually reading the real API against the code, not by field-diffing.

## Goals

Bring every already-implemented module's request/response types and operation set in line with OpenAI's current API, per module:

- **Fine Tuning**: add the `method` field (supervised/DPO/reinforcement training configuration, superseding the flat `hyperparameters`-only shape) including the full Grader union reinforcement fine-tuning needs; add response-side `error`/`method`/`metadata`.
- **Audio**: fix the pre-identified `TtsModel` naming bug (`Whisper1`/`Whisper1Hd` variant names describe TTS models, not Whisper); add newer TTS/STT models; expand `Voice`; add `SttResponseFormat::DiarizedJson`; add new request fields to speech/transcription.
- **Embeddings**: add `dimensions`, `encoding_format`.
- **Files**: `purpose` becomes a closed enum; add `expires_after`/`expires_at`; remove the two confirmed-deprecated `status`/`status_details` fields.
- **Uploads**: fix the real bug above — `UploadFileRequest` becomes JSON metadata, not a multipart file.
- **Batches**: `endpoint` becomes a closed enum (now 8 values, was 3 in the crate's docs); add `output_expires_after`.
- **Completions (legacy)**: add `seed`, `stream_options`; response gains `system_fingerprint`.
- **Chat**: add the remaining request fields the field-diff research found (`response_format`, `reasoning_effort`, `service_tier`, `stream_options`, `seed`, `logprobs`/`top_logprobs`, `store`, `metadata`, `max_completion_tokens`, `safety_identifier`, `prompt_cache_key`); add response-side `refusal`, `usage.completion_tokens_details`/`.prompt_tokens_details`; add three new operations that fill out this already-partially-implemented resource — `retrieve`/`list`/`delete` for stored chat completions (gated by `store: true` on creation).

New shared types, in `rs_openai/src/shared/types.rs` (alongside the existing `File`/`Stop`), since they're genuinely reused across multiple modules:
- `ExpiresAfter { anchor: ExpiresAfterAnchor, seconds: u32 }` — Files, Uploads, and Batches (`output_expires_after`) all use the identical shape.
- `FilePurpose` (8 variants) — Files uses the full set; Uploads uses a documented subset (`assistants`/`batch`/`fine-tune`/`vision`) of the same type, same pattern already established for `ImageSize` being shared across Images' three endpoints with per-endpoint subset validity noted in prose rather than forked into per-endpoint types.

## Non-goals (explicit, deferred to later work)

- **Audio's `voice_consents`/`voices` resources** (Phase 2a scaffolds, currently zero endpoint methods) — these are unimplemented categories, not gaps in an implemented one; per the project owner's own ordering, they belong with the *next* round (implementing Phase 2a's scaffolds), not this one.
- **Fine Tuning Checkpoint Permissions** (also a Phase 2a scaffold, zero methods) — same reasoning.
- **Audio transcription streaming** (`stream: true` on `CreateTranscriptionRequest` plus SSE response parsing) — adding the field alone without a way to consume a streamed response would be a half-finished capability; real streaming support needs the same kind of dedicated plumbing Chat's `create_with_stream` has, which is out of scope for a field-level fixes pass.
- **Chat's multimodal message content** (`content: Option<String>` → allowing `Vec<ContentPart>` for image/audio/file input parts) and a `Developer` role variant — both already deferred once, in the Chat tool-calling slice's own Non-goals, for the same reason: substantial, independent modeling work, not a field-level fix.
- **`CreateSpeechRequest.voice`/`CreateTranscriptionRequest`'s custom-voice-object support** (`{"id": "voice_xxx"}` referencing a Voices-resource entry) — depends on the Voices resource existing, which is itself deferred (see above). This round only expands `Voice`'s built-in named values.
- Realtime API (any form) — permanently out of scope, per every prior phase's decision.
- The Assistants cluster (`assistants`/`threads`/`messages`/`runs`) — frozen, deprecated in place ahead of its 2026-08-26 sunset, per Phase 2a.
- `Chat::create`/`create_with_stream`'s multipart-adjacent internals, and every other module's existing endpoint *methods* not named above — this pass only touches request/response *types* plus the three explicitly-named new Chat operations; no other new endpoints.

## Design

### Shared types (`shared/types.rs`)

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum ExpiresAfterAnchor {
    CreatedAt,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ExpiresAfter {
    pub anchor: ExpiresAfterAnchor,
    pub seconds: u32, // valid range 3600-2592000 per OpenAI's docs; not enforced client-side
}

#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum FilePurpose {
    Assistants,
    AssistantsOutput,
    Batch,
    BatchOutput,
    #[strum(serialize = "fine-tune")]
    #[serde(rename = "fine-tune")]
    FineTune,
    #[strum(serialize = "fine-tune-results")]
    #[serde(rename = "fine-tune-results")]
    FineTuneResults,
    Vision,
    UserData,
}
```

(`FineTune`/`FineTuneResults` need explicit renames since `snake_case` would otherwise produce `fine_tune`/`fine_tune_results`, not the real hyphenated wire values.)

### Fine Tuning (`interfaces/fine_tuning.rs`)

`CreateFineTuningRequest` gains `method: Option<FineTuningMethod>` and `metadata: Option<HashMap<String, String>>`. `hyperparameters` stays (OpenAI's docs mark it "deprecated; use `method` instead" but it's still live — no removal).

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningMethodType { Supervised, Dpo, Reinforcement }

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct FineTuningMethod {
    pub r#type: FineTuningMethodType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supervised: Option<SupervisedMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dpo: Option<DpoMethod>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reinforcement: Option<ReinforcementMethod>,
}
```

A flat struct with three optional sibling objects (not a Rust `enum`) — this mirrors the real wire shape exactly (`{"type": "dpo", "dpo": {...}}`, with `type` as a plain sibling field, not content nested under a `content` key), and needs no custom serde tag/content handling. `SupervisedMethod`/`DpoMethod`/`ReinforcementMethod` each wrap an `Option<...Hyperparameters>` struct reusing the crate's existing `AutoOrInteger` untagged enum (`"auto"` string or a number) for every numeric hyperparameter — `DpoHyperparameters` adds `beta`; `ReinforcementHyperparameters` adds `compute_multiplier`, `eval_interval`, `eval_samples`, and `reasoning_effort: Option<FineTuningReasoningEffort>` (`Default`/`Low`/`Medium`/`High` — a distinct, smaller value set from Chat's own reasoning-effort enum below; do not merge the two types, their wire value sets differ). `ReinforcementMethod` also has a required (non-`Option`) `grader: Grader`.

**Grader union** — six variants, internally tagged, `Multi` recursively holds other graders via `Vec<Grader>` (no `Box` needed — `Vec` already provides the indirection):

```rust
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Grader {
    StringCheck(StringCheckGrader),
    TextSimilarity(TextSimilarityGrader),
    Python(PythonGrader),
    ScoreModel(ScoreModelGrader),
    LabelModel(LabelModelGrader),
    Multi(MultiGrader),
}
```

- `StringCheckGrader { name, input, reference, operation: StringCheckOperation }` — `StringCheckOperation { Eq, Ne, Like, Ilike }`.
- `TextSimilarityGrader { name, input, reference, evaluation_metric: TextSimilarityMetric }` — `TextSimilarityMetric { Cosine, FuzzyMatch, Bleu, Gleu, Meteor, Rouge1, Rouge2, Rouge3, Rouge4, Rouge5, RougeL }` (the `Rouge1`..`Rouge5` variants need explicit `#[serde(rename = "rouge_1")]` etc. — `snake_case` alone would produce `rouge1`, not `rouge_1`).
- `PythonGrader { name, source, image_tag: Option<String> }`.
- `ScoreModelGrader { name, model, input: Vec<GraderMessage>, range: Option<[f32; 2]>, sampling_params: Option<GraderSamplingParams> }` — `GraderMessage { role: GraderMessageRole, content: String, r#type: Option<String> }`, `GraderMessageRole { User, Assistant, System, Developer }`, `GraderSamplingParams { max_completions_tokens: Option<u32>, temperature: Option<f32>, top_p: Option<f32>, seed: Option<i64>, reasoning_effort: Option<GraderReasoningEffort> }` — `GraderReasoningEffort { None, Minimal, Low, Medium, High, Xhigh, Max }` (yet another distinct, larger value set — three different "reasoning effort" enums exist across this one design: Fine Tuning's own, Grader's, and Chat's; each has a genuinely different confirmed value set, so each stays its own type).
- `LabelModelGrader { name, model, input: Vec<GraderMessage>, labels: Vec<String>, passing_labels: Vec<String> }`.
- `MultiGrader { name, graders: Vec<Grader>, calculate_output: String }`.

`FineTuningResponse` gains `method: Option<FineTuningMethod>`, `metadata: Option<HashMap<String, String>>`, `error: Option<FineTuningError>` where `FineTuningError { code: String, message: String, param: Option<String> }`.

### Audio (`interfaces/audio.rs`)

`TtsModel::Whisper1`/`Whisper1Hd` renamed to `Tts1`/`Tts1Hd` (wire values `#[strum(serialize = "tts-1")]`/`"tts-1-hd"` unchanged — only the misleading Rust identifiers change, fixing the naming bug the original field-diff research flagged). Add a new variant `Gpt4oMiniTts` (`"gpt-4o-mini-tts"`) alongside `Tts1`/`Tts1Hd`.

`SttModel` gains `Gpt4oTranscribe`, `Gpt4oMiniTranscribe`, `Gpt4oTranscribeDiarize` (existing `Whisper1` variant is correctly named already, no change).

`Voice` gains `Ash`, `Ballad`, `Coral`, `Sage`, `Verse`, `Marin`, `Cedar` (built-in named voices only — custom voice-object references are out of scope, see Non-goals).

`SttResponseFormat` gains `DiarizedJson` (`"diarized_json"`).

`CreateSpeechRequest` gains `instructions: Option<String>` (doc-noted: doesn't work with `tts-1`/`tts-1-hd`) and `stream_format: Option<AudioStreamFormat>` where `AudioStreamFormat { Sse, Audio }` (also doc-noted as unsupported on the two legacy models).

`CreateTranscriptionRequest` gains `include: Option<Vec<TranscriptionInclude>>` (`TranscriptionInclude { Logprobs }` — single confirmed value, modeled as an enum for forward-compatibility, matching the crate's established convention of enum-over-string for closed sets even at one value, e.g. `ExpiresAfterAnchor` above), `chunking_strategy: Option<String>` (kept as a plain string — the exact object shape isn't fully confirmed and `"auto"` is the only documented literal value seen, not worth a premature enum), `known_speaker_names: Option<Vec<String>>`, `known_speaker_references: Option<Vec<String>>` (base64-encoded audio references).

### Embeddings (`interfaces/embeddings.rs`)

`CreateEmbeddingRequest` gains `dimensions: Option<u32>` and `encoding_format: Option<EncodingFormat>` where `EncodingFormat { Float, Base64 }`.

### Files (`interfaces/files.rs`)

`UploadFileRequest.purpose: String` → `purpose: FilePurpose` (shared type, breaking change — justified as modeling a now-confirmed closed set, same class of fix as `ImageSize`/`ImageData` in the Images slice). Add `expires_after: Option<ExpiresAfter>` (shared type). `FileResponse` gains `expires_at: Option<i64>`; **removes** `status`/`status_details` entirely (both confirmed deprecated in OpenAI's current schema — deletion, not deprecation-in-place, matching this crate's established no-compat-shim convention for confirmed-dead fields).

### Uploads (`interfaces/uploads.rs`, `apis/uploads.rs`) — real bug fix

`UploadFileRequest` replaced entirely:

```rust
#[derive(Builder, Clone, Debug, Serialize)]
pub struct UploadFileRequest {
    pub bytes: u64,
    #[builder(into)]
    pub filename: String,
    #[builder(into)]
    pub mime_type: String,
    pub purpose: FilePurpose, // shared type; Uploads only accepts assistants/batch/fine-tune/vision, a subset of the full 8
}
```

No `File`/byte-buffer field at all — this struct now only carries the metadata needed to *open* an upload session; actual bytes go through the already-correct `add_upload_part`. `apis/uploads.rs`'s `upload()` method changes from building a multipart `Form` to a plain JSON POST: `self.openai.post("/uploads", req).await` (mirroring how `complete_upload` already correctly does a JSON POST two lines below it in the same file). `UploadFileResponse.status: String` becomes `status: UploadStatus` where `UploadStatus { Pending, Completed, Cancelled, Expired }`. `CompleteUploadResponse.file: CompleteUploadFile` becomes `file: Option<CompleteUploadFile>` (confirmed: null until the upload actually completes — the struct's own name already implies "this exists once complete," but the response object it's embedded in is returned at multiple lifecycle points, including before completion).

### Batches (`interfaces/batch.rs`)

`CreateBatchRequest.endpoint: String` → closed enum:

```rust
#[derive(Debug, Serialize, Deserialize, Clone, strum::Display)]
pub enum BatchEndpoint {
    #[strum(serialize = "/v1/responses")]
    #[serde(rename = "/v1/responses")]
    Responses,
    #[strum(serialize = "/v1/chat/completions")]
    #[serde(rename = "/v1/chat/completions")]
    ChatCompletions,
    #[strum(serialize = "/v1/embeddings")]
    #[serde(rename = "/v1/embeddings")]
    Embeddings,
    #[strum(serialize = "/v1/completions")]
    #[serde(rename = "/v1/completions")]
    Completions,
    #[strum(serialize = "/v1/moderations")]
    #[serde(rename = "/v1/moderations")]
    Moderations,
    #[strum(serialize = "/v1/images/generations")]
    #[serde(rename = "/v1/images/generations")]
    ImagesGenerations,
    #[strum(serialize = "/v1/images/edits")]
    #[serde(rename = "/v1/images/edits")]
    ImagesEdits,
    #[strum(serialize = "/v1/videos")]
    #[serde(rename = "/v1/videos")]
    Videos,
}
```

(Every variant needs an explicit rename — these wire values are literal URL paths with slashes, which no `rename_all` case-conversion mode produces.) `CreateBatchRequest` gains `output_expires_after: Option<ExpiresAfter>` (shared type).

### Completions, legacy (`interfaces/completions.rs`)

`CreateCompletionRequest` gains `seed: Option<i64>` and `stream_options: Option<StreamOptions>` where `StreamOptions { include_usage: Option<bool> }` (a small shared type — Chat needs the identical shape, see below; defined once, reused by both). `CompletionResponse` gains `system_fingerprint: Option<String>`.

### Chat (`interfaces/chat.rs`, `apis/chat.rs`)

`CreateChatRequest` gains:
- `response_format: Option<ChatResponseFormat>` — internally tagged: `ChatResponseFormat { Text, JsonObject, JsonSchema { json_schema: JsonSchemaFormat } }` where `JsonSchemaFormat { name, description: Option<String>, schema: serde_json::Value, strict: Option<bool> }`. Named `ChatResponseFormat` (not `ResponseFormat`) to stay unambiguous alongside Images' unrelated `ResponseFormat` type, even though the two live in different module namespaces.
- `reasoning_effort: Option<ChatReasoningEffort>` — `ChatReasoningEffort { None, Minimal, Low, Medium, High, Xhigh, Max }` (7 values — distinct from Fine Tuning's and Grader's reasoning-effort types, see Fine Tuning section above).
- `service_tier: Option<ServiceTier>` — `ServiceTier { Auto, Default, Flex, Scale, Priority, Fast }` (6 values).
- `stream_options: Option<StreamOptions>` (shared type, defined in Completions section above — only the well-confirmed `include_usage` field; a second field surfaced during research as "include_obfusion" was not corroborated by any other source and is excluded as likely fetch noise rather than risk shipping a fabricated field name).
- `seed: Option<i64>`, `logprobs: Option<bool>`, `top_logprobs: Option<u8>` (0-20).
- `store: Option<bool>`, `metadata: Option<HashMap<String, String>>`.
- `max_completion_tokens: Option<u32>` (newer, reasoning-model-compatible name; existing `max_tokens` stays, no removal).
- `safety_identifier: Option<String>`, `prompt_cache_key: Option<String>` (newer replacements for the now-secondary `user` field; `user` stays, no removal — this round adds, it doesn't deprecate-in-place).

`ChatCompletionMessage` gains `refusal: Option<String>`, following this struct's own established always-serialize convention (no `skip_serializing_if`, matching `content`/`tool_calls`/`tool_call_id`/`name`) even though it's realistically response-only, for internal consistency with the struct's existing fields rather than introducing a second convention on the same type.

`ChatUsage` gains `completion_tokens_details: Option<CompletionTokensDetails>` (`reasoning_tokens`, `audio_tokens`, `accepted_prediction_tokens`, `rejected_prediction_tokens`, all `u32`) and `prompt_tokens_details: Option<PromptTokensDetails>` (`audio_tokens`, `cached_tokens`, `cache_write_tokens`, all `u32`).

**Three new operations** on `Chat<'a>`, filling out gaps in this already-implemented resource (gated by `store: true` at creation time — not a new top-level category, so in scope for this pass per the Context section above):

```rust
pub async fn retrieve(&self, completion_id: &str) -> OpenAIResponse<ChatResponse> {
    self.openai.get(&format!("/chat/completions/{completion_id}"), &()).await
}
pub async fn list(&self, req: &ListChatCompletionsRequest) -> OpenAIResponse<ListChatCompletionsResponse> {
    self.openai.get("/chat/completions", req).await
}
pub async fn delete(&self, completion_id: &str) -> OpenAIResponse<DeleteChatCompletionResponse> {
    self.openai.delete(&format!("/chat/completions/{completion_id}"), &()).await
}
```

`ListChatCompletionsRequest { after: Option<String>, limit: Option<u32>, metadata: Option<HashMap<String, String>>, model: Option<String>, order: Option<SortOrder> }` — `SortOrder { Asc, Desc }` (kept Chat-local rather than shared/generalized — this pass fixes gaps in specific modules, not a broader pagination refactor). `ListChatCompletionsResponse { data: Vec<ChatResponse>, first_id: String, last_id: String, has_more: bool, object: String }`. `DeleteChatCompletionResponse { id: String, deleted: bool, object: String }`.

## Testing

Each module gets targeted `wiremock` tests covering: the new request fields serializing correctly (including the Grader union's six variants each tag-discriminating correctly, and `ChatResponseFormat`'s three variants), the new response fields deserializing correctly, and — for Uploads specifically — a regression test proving the fixed `upload()` sends a JSON body (not multipart) with the four metadata fields, since that's the flagship bug this pass fixes. Existing tests are edited only where a field/type they directly touch changed shape (Files' `purpose` enum, Uploads' whole request, Batches' `endpoint` enum) — these are direct, unavoidable consequences of the same field's own mandated shape change, not scope creep, matching the precedent already established across every prior Phase 2b slice.

## Versioning

Mixed: most additions are purely additive (new optional fields, new enum variants, new operations). Real breaking changes: `Files.purpose`/`Uploads.purpose` (String → enum), `Uploads`' entire request shape (the bug fix), `Batches.endpoint` (String → enum), `Audio::TtsModel`'s renamed variants, `Files.status`/`.status_details` removed. Each is justified individually above as a correctness fix (closed-set modeling, a real bug, a naming fix, or removing confirmed-dead fields) — not compatibility-optional choices. Ships as a pre-1.0 minor bump, exact number decided at implementation/release time, consistent with how the two prior Phase 2b slices handled versioning (deferred to release aggregation rather than bumped per-slice).
