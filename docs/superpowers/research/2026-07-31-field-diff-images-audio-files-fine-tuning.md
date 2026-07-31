# Field-level diff: Images, Audio, Files, Uploads, Batches, Fine Tuning (2026-07-31)

Companion to `2026-07-31-openai-api-structure.md`. That doc covers category/endpoint-level coverage; this one goes field-by-field on the modules `rs_openai` already implements, comparing the current Rust structs against OpenAI's live docs. No backward-compat constraint per the project owner — items below are framed as additions/removals to make directly, not deprecation paths. Descriptions are in my own words; sources are cited per section.

## Images
Rust source: `rs_openai/src/interfaces/images.rs`. Docs: `resources/images.md`, `resources/images/methods/create_edit.md` (404'd — inferred from the generate page's shared parameter table plus `create_variation.md`), `resources/images/methods/create_variation.md`.

**`CreateImageRequest` (→ `POST /v1/images/generations`)**
- **Add:** `model` — missing entirely today; current values are `gpt-image-1.5`, `gpt-image-1`, `gpt-image-1-mini`, `dall-e-2`, `dall-e-3`, defaulting to `dall-e-2`. `quality`, `background`, `output_format`, `output_compression`, `moderation`, `stream`, `partial_images` — all gpt-image-model-only, all missing. `style` (`vivid`/`natural`) — dall-e-3-only, missing.
- **Changed shape:** `size`/`ImageSize` needs `auto`, `1536x1024`, `1024x1536`, `1792x1024`, `1024x1792` added alongside the existing three (the existing `256x256`/`512x512` are dall-e-2-only legacy values now, not deprecated but a shrinking minority of the option set). `response_format` is now model-conditional: only meaningful for dall-e models — gpt-image models always return base64 and ignore it.
- **Doc drift:** `ImageSize`'s doc comment ("Must be one of 256x256, 512x512, or 1024x1024") describes a subset that's no longer close to complete.

**`CreateImageEditRequest` (→ `POST /v1/images/edits`)**
- **Add:** same gpt-image-model fields as generate (`model`, `quality`, `background`, `output_format`, `output_compression`, `moderation`, `stream`, `partial_images`), plus edit-specific `input_fidelity` (`high`/`low`).
- **Changed shape — the big one:** current `image: File` (singular) should become an array (docs say up to 16 images for gpt-image models; each entry can be a direct upload or a `file_id`/`image_url` reference, not just raw bytes). `mask` similarly can now be a reference, not only a direct upload. `size` for edit accepts `auto`/`1024x1024`/`1536x1024`/`1024x1536` (no `256x256`/`512x512` — those are generate/variation-only, dall-e-2 legacy sizes).

**`CreateImageVariationRequest` (→ `POST /v1/images/variations`)**
- dall-e-2-only endpoint; gpt-image models don't support variations at all, so no new capability fields apply here. `size` in the current docs lists `1024x1024`/`1024x1536`/`1536x1024` (no `256x256`/`512x512`) — **[verify before implementing]**, this could be page drift on OpenAI's side rather than an intentional dall-e-2 change, since dall-e-2 traditionally only did square sizes.

**`ImageResponse`**
- **Add (response-level):** `background`, `output_format`, `quality`, `size`, `usage` (token accounting, gpt-image models only) — currently the response struct only has `created`+`data`.
- **Changed shape:** `ImageData` is currently a `Url(String) | B64Json(String)` enum. Real responses can also include `revised_prompt` (dall-e-3) alongside the image data — this needs to become a struct with optional `url`/`b64_json`/`revised_prompt` fields rather than a two-variant enum, since a single response item can carry more than one of these.

## Audio
Rust source: `rs_openai/src/interfaces/audio.rs`. Docs: `resources/audio/subresources/speech/methods/create.md`, `.../transcriptions/methods/create.md`, `.../voice_consents/methods/create.md`, `.../voices/methods/create.md`.

**`CreateSpeechRequest` (→ `POST /v1/audio/speech`)**
- **Add:** `instructions` (style-control text, incompatible with `tts-1`/`tts-1-hd`), `stream_format` (`sse`/`audio`, also unavailable on the two legacy models).
- **Changed shape:** `TtsModel` needs `gpt-4o-mini-tts` (and its dated snapshot). `Voice` needs `ash`, `ballad`, `coral`, `sage`, `verse`, `marin`, `cedar` added to the existing six, **and** needs to support a custom-voice reference (`{"id": "voice_xxx"}`, tying into the new Voices resource) — this pushes `Voice` from a plain enum toward an untagged enum of (named voice | custom voice object).
- **Doc drift / naming bug, independent of the API update:** `TtsModel`'s two variants are named `Whisper1`/`Whisper1Hd` — Whisper is OpenAI's *speech-to-text* model family; these are text-to-speech models (`tts-1`/`tts-1-hd`). The `#[strum(serialize)]` wire values are correct, only the Rust variant names are misleading.

**`CreateTranscriptionRequest` (→ `POST /v1/audio/transcriptions`)**
- **Add:** `stream` (streaming transcript events), `include[]` (currently just `logprobs`), `chunking_strategy`, `known_speaker_names[]`/`known_speaker_references[]` (speaker diarization).
- **Changed shape:** `SttModel` needs `gpt-4o-transcribe`, `gpt-4o-mini-transcribe`, `gpt-4o-transcribe-diarize` added alongside `whisper-1`. `SttResponseFormat` needs `diarized_json` added.

**`CreateTranslationRequest` (→ `POST /v1/audio/translations`)** — not separately re-checked this pass; this is the Whisper-only English-translation endpoint and the fetched pages gave no indication it gained new models. Low priority relative to the transcription gap.

**Net-new resources (zero existing coverage):**
- **Audio Voice Consents** (`POST /v1/audio/voice_consents` + presumably list/retrieve/update/delete per the structure doc) — records a user's consent to have their voice used/cloned. Create fields: `language` (BCP 47 code), `name` (label), `recording` (audio file). Response: `id`, `created_at`, `language`, `name`, `object: "audio.voice_consent"`.
- **Audio Voices** (`POST /v1/audio/voices` + list/etc.) — registers a custom voice built from a sample + a consent record. Create fields: `audio_sample` (file), `consent` (a Voice Consent id), `name`. Response: `id`, `created_at`, `name`, `object: "audio.voice"`. This `id` is exactly what `CreateSpeechRequest.voice`'s new custom-voice-object form (above) references.

## Files
Rust source: `rs_openai/src/interfaces/files.rs`. Docs: `resources/files/methods/create.md`.

- **Add:** `expires_after` (object: `anchor` + `seconds`) on the create request.
- **Changed shape:** `purpose` is currently a bare `String` field with no enum and a doc comment mentioning only `"fine-tune"`. The real, current allowed set is `assistants`, `assistants_output`, `batch`, `batch_output`, `fine-tune`, `fine-tune-results`, `vision`, `user_data` — worth a real enum instead of a free string, both for `UploadFileRequest.purpose` here and the identically-shaped field in Uploads below.
- **Remove:** `FileResponse.status` and `.status_details` are both explicitly marked deprecated in OpenAI's current docs. Per the no-compat directive, drop both rather than keep dead fields (missing keys in the real JSON response are simply ignored by `#[derive(Deserialize)]` either way, so this is safe either direction, but the fields serve no purpose going forward).
- **Add (response):** `expires_at`, mirroring the new request-side `expires_after`.

## Uploads
Rust source: `rs_openai/src/interfaces/uploads.rs`. Docs: `resources/uploads/methods/create.md`.

- `purpose`'s current doc comment (`"assistants"`, `"vision"`, `"batch"`, `"fine-tune"`) matches OpenAI's current docs for this endpoint — no change needed here, though it should share the same enum type as Files' `purpose` once that's introduced (Files' allowed set is a superset).
- **Add:** `expires_after` on the create request (same object shape as Files: `anchor`, `seconds` — anchor is `"created_at"` only, seconds range 3600–2,592,000).
- **Changed shape:** `UploadFileResponse.status` is currently a bare `String`; the real, closed set is `pending`/`completed`/`cancelled`/`expired` — worth an enum. `CompleteUploadResponse.file: CompleteUploadFile` is currently non-optional; OpenAI's docs describe the nested `file` object as present only once the upload completes — should be `Option<CompleteUploadFile>`.

## Batches
Rust source: `rs_openai/src/interfaces/batch.rs`. Docs: `resources/batches/methods/create.md`.

- **Real bug, not just an update — flagging clearly:** `CreateBatchRequest.meta: Option<serde_json::Value>` has no `#[serde(rename = "metadata")]`, but the actual API field is named `metadata`. As written, this field serializes to the wire as `"meta"`, which OpenAI's API doesn't recognize — batch metadata set through this crate is silently dropped today. This should be caught and fixed regardless of the broader Phase 2 work (it's the same class of bug Phase 1 found and fixed repeatedly — worth fixing immediately rather than waiting).
- **Changed shape — significant:** `endpoint`'s doc comment lists only `/v1/chat/completions`, `/v1/embeddings`, `/v1/completions`. The current full set is `/v1/responses`, `/v1/chat/completions`, `/v1/embeddings`, `/v1/completions`, `/v1/moderations`, `/v1/images/generations`, `/v1/images/edits`, `/v1/videos` — more than double. Worth turning `endpoint` into an enum now that the set is large and stable-ish, rather than a free string.
- **Add:** `output_expires_after` (object: `anchor` — `"created_at"` only — + `seconds`, 3600–2,592,000) on the create request.

## Fine Tuning
Rust source: `rs_openai/src/interfaces/fine_tuning.rs`. Docs: `resources/fine_tuning/subresources/jobs/methods/list.md`, `.../checkpoints/subresources/permissions/methods/create.md`.

**Biggest capability gap in this whole batch.** `CreateFineTuningRequest` currently only models the plain supervised-fine-tuning shape (a flat `hyperparameters` object). OpenAI now documents a `method` field selecting between three fine-tuning approaches, each with its own hyperparameter shape:
- **Supervised** — `batch_size`, `learning_rate_multiplier`, `n_epochs` (all `"auto"` or a number) — this is what the crate has today, just not modeled as one arm of a `method` union.
- **DPO** (Direct Preference Optimization) — same three, plus `beta` (`"auto"` or number — controls the penalty weight between the trained and reference policy).
- **Reinforcement** — `batch_size`, `learning_rate_multiplier`, `n_epochs`, plus `compute_multiplier`, `eval_interval`, `eval_samples` (all `"auto"` or number), `reasoning_effort` (`default`/`low`/`medium`/`high`), and a `grader` configuration object. Graders are themselves polymorphic (`StringCheck`, `TextSimilarity`, `Python`, `ScoreModel`, `MultiGrader`, `LabelModel`) — real modeling work, flagged for the implementation phase rather than designed here.

- **Add to `CreateFineTuningRequest`:** `method` (the tagged union above — this effectively supersedes today's flat `hyperparameters` field as the primary way to configure a job; keeping `hyperparameters` as a legacy/implicit-supervised path is an implementation-time call, not a design one).
- **Add to `FineTuningResponse`:** `error` (job failure detail — currently no way to see why a job failed beyond `status: "failed"`), `method`, `metadata`.

**Net-new resource:** Fine Tuning Checkpoint Permissions (`POST/GET/DELETE .../fine_tuning/checkpoints/{checkpoint}/permissions`, admin-key-only) — grants other projects in the org access to a fine-tuned checkpoint. Create: path param `fine_tuned_model_checkpoint`, body `project_ids: [string]`. Response (single): `id`, `created_at`, `object: "checkpoint.permission"`, `project_id`. List response: `data`, `has_more`, `first_id`, `last_id`.

## Summary: single biggest gap

**Fine Tuning's missing `method` field** (no DPO or Reinforcement fine-tuning support at all, only implicit supervised) is the largest capability gap — it's not a missing param, it's an entire configuration axis with three real, actively-used training methodologies behind it. Close second: **Images has no `model` field whatsoever today**, meaning every gpt-image-1/1.5/mini-specific capability (quality tiers, background/transparency, streaming partial images, format control) is unreachable regardless of what else gets added, since there's no way to even select those models.
