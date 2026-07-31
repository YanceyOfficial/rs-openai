# Field-level diff: chat/completions, embeddings, moderations, models (2026-07-31)

Companion to `2026-07-31-openai-api-structure.md`. That doc covers endpoint-level coverage; this one goes field-by-field on request/response structs for five already-implemented `rs_openai` modules, against OpenAI's current docs. Per the project owner's direction: no backward-compat constraint — fields OpenAI no longer documents are flagged for removal, not preservation.

**Sourcing:** each section is marked **[fetched]** (pulled live this session from the cited URL) or **[training]** (OpenAI's `POST /chat/completions` create-parameters page returned no usable content via `llms.txt`/`llms-full.txt`/the `.md` export after five attempts — likely not yet published in machine-readable form on `developers.openai.com` — so that one section relies on stable, well-established knowledge of this endpoint instead; treat it as needing a live-docs check before implementation, same as the `[verify]` tags in the companion doc).

## Chat — `interfaces/chat.rs`

**Endpoint-level correction to the companion doc:** `resources/chat.md` **[fetched]** lists **four** operations, not two: `POST /chat/completions` (create), `GET /chat/completions/{id}` (retrieve, only for `store: true` completions), `GET /chat/completions` (**list** stored completions — not previously noted), `DELETE /chat/completions/{id}` (**delete** a stored completion — not previously noted). `rs_openai` has create only.

### `CreateChatRequest` — **[training]**, not live-fetch-confirmed this session

**Add** (all confirmed present in the API as of recent knowledge, not verified against a live page this session):
- `tools` (array) / `tool_choice` (string | object) / `parallel_tool_calls` (bool) — function/tool calling. **This is the single biggest capability gap**: `ChatCompletionMessage` has no `tool_calls` or `tool_call_id` field and `Role` has no `Tool` variant, so the crate cannot represent a tool-calling turn at all, in either direction.
- `response_format` (object: `text` / `json_object` / `json_schema` with `strict` mode) — structured outputs.
- `max_completion_tokens` — newer name; `max_tokens` is deprecated/reasoning-model-incompatible but the crate only has `max_tokens`.
- `reasoning_effort` (`"minimal"`/`"low"`/`"medium"`/`"high"`) — reasoning-model param.
- `service_tier` (`"auto"`/`"default"`/`"flex"`/`"priority"`).
- `store` (bool) — required `true` to use the newly-noted retrieve/list/delete endpoints above.
- `stream_options` (object, e.g. `include_usage`).
- `modalities` (array, e.g. `["text","audio"]`) + `audio` (object, output voice/format config).
- `prediction` (object) — predicted-output latency optimization.
- `metadata` (object, freeform key-value tags).
- `seed`, `logprobs` (bool), `top_logprobs` (int).
- `parallel_tool_calls`.

**Changed shape:**
- `ChatCompletionMessage.content: String` → should be `String | Vec<ContentPart>` (text, image_url, input_audio, file parts) for multimodal input. Currently string-only.
- `Role` enum (System/User/Assistant) is missing `Tool` (required for tool-response messages) and likely `Developer` (the newer system-role naming for reasoning-model contexts).

**Remove:** none identified — existing fields (`temperature`, `top_p`, `n`, `stream`, `stop`, `presence_penalty`, `frequency_penalty`, `logit_bias`, `user`) all still appear to be live params.

**Response side additions:** `choices[].message.refusal`, `choices[].message.tool_calls`, `choices[].message.audio`; `choices[].logprobs`; `usage.completion_tokens_details` (reasoning_tokens, audio_tokens, accepted/rejected_prediction_tokens); `usage.prompt_tokens_details` (cached_tokens, audio_tokens); `service_tier` in the response envelope.

## Completions (legacy) — `interfaces/completions.rs` — **[fetched]** `resources/completions/methods/create.md`

Confirmed **not marked deprecated** by OpenAI, scoped to `gpt-3.5-turbo-instruct`/`davinci-002`/`babbage-002` only.

**Add:** `seed`, `stream_options`. Response gained `system_fingerprint` (current Rust `CompletionResponse` doesn't have it).

**Remove:** none — `best_of`, `echo`, `suffix`, `logprobs` (u8 max-5 count), etc. all still documented as-is.

**No shape changes** identified for `Prompt`/request/response beyond the two additions above.

## Embeddings — `interfaces/embeddings.rs` — **[fetched]** `resources/embeddings/methods/create.md`

**Add:**
- `dimensions` (number) — truncate output vector size, `text-embedding-3-*` and later only. Real gap, was already flagged as a follow-up in Phase 1 research.
- `encoding_format` (`"float"` | `"base64"`) — missing entirely from `CreateEmbeddingRequest`.

**Remove:** none.

**Confirmed unchanged:** `input`, `model`, `user`; response `data[].embedding`/`index`/`object`, `model`, `object`, `usage.prompt_tokens`/`total_tokens`.

## Moderations — `interfaces/moderations.rs` — **[fetched]** `resources/moderations/methods/create.md`

**Confirmed the category set is unchanged** — all 13 categories in `Categories`/`CategoryScores`/`CategoryAppliedInputTypes` (harassment, harassment/threatening, sexual, hate, hate/threatening, illicit, illicit/violent, self-harm/intent, self-harm/instructions, self-harm, sexual/minors, violence, violence/graphic) match OpenAI's current docs exactly. No additions, no removals, no renames.

**Add / Remove / Changed shape:** none identified for `CreateModerationRequest`, `ModerationResponse`, or the category structs.

## Models — `interfaces/models.rs` — **[fetched]** `resources/models/methods/list.md`, `.../delete.md`

**Confirmed unchanged.** Model object: `id`, `object` (always `"model"`), `created`, `owned_by` — exactly matches `ModelResponse`. Delete response: `id`, `deleted`, `object` — exactly matches `DeleteModelResponse`. `ListModelResponse`/`object`+`data` also matches. No additions, no removals, no shape changes for any of the three Models operations.

## Doc-comment drift

No field in these five modules has a doc comment that's actively wrong (wrong default, wrong constraint, references a shut-down model) — the existing prose is accurate as far as it goes, just incomplete relative to the fields listed above as missing.
