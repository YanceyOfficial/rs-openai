# OpenAI API Structure Map (as of 2026-07-31)

Research artifact for Phase 2 of the `rs_openai` modernization: a full structural map of OpenAI's current public API, to inform a redesign of the crate's module layout around the same tiers OpenAI itself uses.

## Methodology

- **Primary source:** `https://developers.openai.com/api/reference/llms.txt`, OpenAI's own machine-readable index of every page in its API reference. This is complete and current — it enumerates every resource and every CRUD-style operation under it, matching the same 3-tier structure (category → resource → operation) the project owner pointed to.
- **Cross-check:** `https://raw.githubusercontent.com/openai/openai-openapi/master/openapi.yaml` (OpenAI's official OpenAPI 3.1 spec, maintained in `openai/openai-openapi`). This file is large enough that a single fetch only returned paths through the start of `/chat/completions` before truncating — not usable for a full path inventory in one pass. The portion retrieved (`/assistants`, `/audio/speech`, `/audio/transcriptions`, `/audio/translations`, `/audio/voice_consents`, `/audio/voices`, `/batches`, `/chat/completions`, with their HTTP methods) **confirms** OpenAI's path conventions are exactly what training-era familiarity with this API would predict: collection paths like `/v1/{resource}` for list/create, `/v1/{resource}/{id}` for retrieve/update/delete, and `/v1/{resource}/{id}/{action}` for verb-actions like `cancel`.
- **Path precision:** every path below is one of:
  - **[confirmed]** — directly observed in the openapi.yaml fragment actually retrieved.
  - *(no marker)* — not directly observed this session, but a stable, long-documented OpenAI path I have high confidence in from extensive prior exposure (e.g. `/v1/chat/completions`, `/v1/embeddings`, `/v1/models`).
  - **[verify]** — a newer or less-familiar resource (ChatKit, Realtime Calls, Videos, Voice Consents/Voices, Conversations, Containers, Evals/Graders, Responses `compact`) where the path is inferred from OpenAI's own consistent naming convention plus the resource name in the reference index, but was not directly confirmed against the spec this session. **Confirm exact paths against the openapi.yaml or live docs before implementing**, not before designing module structure.
- Descriptions below are one-line summaries in my own words, not copied from OpenAI's docs.
- `rs_openai`'s current modules (read from `rs_openai/src/apis/mod.rs`): `assistants`, `audio`, `batch`, `chat`, `completions`, `edits`, `embeddings`, `engines`, `files`, `fine_tuning`, `images`, `messages`, `models`, `moderations`, `runs`, `threads`, `uploads` (17 modules).

## Coverage summary (category level)

| OpenAI category | rs_openai status | Notes |
|---|---|---|
| Responses | **Missing** | The flagship, OpenAI-recommended primary API. Highest-priority gap. |
| Conversations | **Missing** | Responses API's stateful-context companion; replaces Assistants' "thread" concept. |
| Vector Stores | **Missing** | Needed for Responses' `file_search` tool. |
| ChatKit (Beta) | **Missing** | The exact 3-tier example the project owner cited (ChatKit → Sessions/Threads → ops). |
| Containers | **Missing** | Backs the Code Interpreter tool. |
| Evals | **Missing** | Includes Graders as a sub-resource. |
| Realtime | **Missing** | Explicitly deferred in Phase 1 planning — WebSocket/WebRTC/SIP, needs its own connection-oriented design. Still deferred here; flagged only for completeness. |
| Videos | **Missing** | Sora video generation. |
| Webhooks | **Missing** | Signature verification / event unwrapping, not a CRUD resource. |
| Organization / Projects (admin) | **Missing** | Large, entirely separate admin-tier surface (audit logs, API keys, users, roles, rate limits). |
| Chat (Completions) | **Partial** | Create + streaming exist; missing "retrieve a stored completion" (GET by id). |
| Images | **Partial** | Create/edit/variations exist; missing streaming partial-image events (`gpt-image-1`-era feature). |
| Audio | **Partial** | Speech/transcription/translation exist; missing Voice Consents and Voices resources. |
| Fine Tuning | **Partial** | Jobs create/list/retrieve/cancel exist; missing Checkpoint Permissions endpoints. |
| Assistants (Beta) | **Implemented, deprecated on OpenAI's side** | Sunsets 2026-08-26 per OpenAI's deprecations page. Threads/Messages/Runs also present; Runs is missing "Submit Tool Outputs" and there's no "Run Steps" sub-resource at all. |
| Batches | Implemented | Operation set matches (create/list/retrieve/cancel). |
| Embeddings | Implemented | Field-level check needed later (e.g. `dimensions` param on newer models). |
| Files | Implemented | Operation set matches. |
| Models | Implemented | Operation set matches. |
| Moderations | Implemented | Operation set matches. |
| Uploads | Implemented | Operation set matches (create/cancel + Parts create). |
| Completions (legacy) | Implemented | Legacy on OpenAI's side but still live; low priority to touch further. |
| Edits | **Drop** | `rs_openai` still has this module; the endpoint doesn't exist anywhere in OpenAI's current reference (shut down 2024-01-04). |
| Engines | **Drop** | Same — `rs_openai` still has this module; endpoint shut down 2022-12-03. |

## Full structure tree

### Responses — *Missing entirely, top priority*
- `POST /v1/responses` — Create a model response (the primary, agentic-by-default entry point OpenAI now recommends over Chat Completions).
- `GET /v1/responses/{response_id}` — Retrieve a stored response.
- `DELETE /v1/responses/{response_id}` — Delete a stored response.
- `POST /v1/responses/{response_id}/cancel` — Cancel a background response.
- `POST /v1/responses/{response_id}/compact` **[verify]** — Compact a response's context (newer feature).
- `GET /v1/responses/{response_id}/input_items` — List a response's input items.
- Responses Input Tokens **[verify]** — token-counting helper for response input.
- Responses streaming events / WebSocket events — SSE and WebSocket event schemas for streaming responses; not a CRUD endpoint, but a large set of typed event variants worth modeling explicitly.

### Conversations — *Missing entirely*
- `POST /v1/conversations` **[verify]** — Create a conversation (stateful context object).
- `GET /v1/conversations/{conversation_id}` **[verify]** — Retrieve a conversation.
- `POST /v1/conversations/{conversation_id}` **[verify]** — Update a conversation (e.g. metadata).
- `DELETE /v1/conversations/{conversation_id}` **[verify]** — Delete a conversation.
- `GET /v1/conversations/{conversation_id}/items` **[verify]** — List items in a conversation.

### Vector Stores — *Missing entirely*
- `POST /v1/vector_stores` — Create a vector store.
- `GET /v1/vector_stores` — List vector stores.
- `GET /v1/vector_stores/{vector_store_id}` — Retrieve a vector store.
- `POST /v1/vector_stores/{vector_store_id}` — Update a vector store.
- `DELETE /v1/vector_stores/{vector_store_id}` — Delete a vector store.
- `POST /v1/vector_stores/{vector_store_id}/search` — Search a vector store.
- **Vector Stores Files** (sub-resource): `POST` create, `GET` list, `GET /{file_id}` retrieve, `POST /{file_id}` update, `DELETE /{file_id}` delete, `GET /{file_id}/content` — content retrieve.
- **Vector Stores File Batches** (sub-resource): `POST` create, `GET /{batch_id}` retrieve, `POST /{batch_id}/cancel` cancel, `GET /{batch_id}/files` list files.

### ChatKit (Beta) — *Missing entirely — the exact 3-tier example cited*
- **ChatKit Sessions**: `POST /v1/chatkit/sessions` **[verify]** create, `POST /v1/chatkit/sessions/{session_id}/cancel` **[verify]** cancel.
- **ChatKit Threads**: `GET /v1/chatkit/threads` **[verify]** list, `GET /v1/chatkit/threads/{thread_id}` **[verify]** retrieve, `DELETE /v1/chatkit/threads/{thread_id}` **[verify]** delete, `GET /v1/chatkit/threads/{thread_id}/items` **[verify]** list items.

### Containers — *Missing entirely*
- `POST /v1/containers` **[verify]** create, `GET /v1/containers` **[verify]** list, `GET /v1/containers/{container_id}` **[verify]** retrieve, `DELETE /v1/containers/{container_id}` **[verify]** delete.
- **Containers Files** (sub-resource): create, list, retrieve, delete **[verify]**, plus `GET .../content` **[verify]** for file content retrieve.

### Evals — *Missing entirely*
- `POST /v1/evals` create, `GET /v1/evals` list, `GET /v1/evals/{eval_id}` retrieve, `POST /v1/evals/{eval_id}` update, `DELETE /v1/evals/{eval_id}` delete.
- `POST /v1/evals/{eval_id}/runs/{run_id}/cancel` **[verify]** — cancel an eval run.
- **Graders** — standalone resource for defining custom grading logic used by evals; operation set not fully enumerated by the index, needs its own lookup during implementation.

### Realtime — *Missing entirely, explicitly deferred (not this phase either)*
- `POST /v1/realtime/client_secrets` **[verify]** — issue an ephemeral client secret for a Realtime session.
- **Realtime Calls**: accept, create, hangup, refer, reject **[verify]** — SIP/telephony-oriented call control.
- Realtime client events / server events / translation client events / translation server events — large sets of typed WebSocket event schemas, not simple CRUD.
- Kept out of scope per the existing decision: needs its own connection-oriented architecture separate from this crate's request/response HTTP client.

### Videos — *Missing entirely*
- `POST /v1/videos` **[verify]** create, `GET /v1/videos` **[verify]** list, `GET /v1/videos/{video_id}` **[verify]** retrieve, `DELETE /v1/videos/{video_id}` **[verify]** delete. (Sora video generation.)

### Webhooks — *Missing entirely*
- Not a CRUD resource — signature verification and typed event payloads for OpenAI's outbound webhooks (e.g. batch completion, eval run completion). Needs an "unwrap"-style helper (verify + deserialize a webhook payload) rather than an HTTP-calling module.

### Organization / Projects (admin surface) — *Missing entirely, large*
All under an implied `/v1/organization/...` and `/v1/organization/projects/{project_id}/...` path family **[verify exact prefixes]**:
- **Audit Logs**: list; **Admin API Keys** (create/delete/list/retrieve) and **Usage** as sub-resources.
- **Groups**: create/delete/list/update; **Groups Users**: create/delete/list.
- **Invites**: create/delete/list/retrieve.
- **Projects**: create/list/retrieve/update; **Projects API Keys**: delete/list/retrieve; **Projects Groups**: delete/list; **Projects Rate Limits**: (read/update); **Projects Service Accounts**: create/delete/list/retrieve; **Projects Users**: create/delete/list/retrieve/update.
- **Roles**: create/delete/list/update.
- **Users**: delete/list/retrieve/update; **Users Roles**: create/delete/list.
- A parallel, slightly differently-scoped **Projects** family (Groups Roles, Roles, Users Roles — create/delete/list/update) also appears in the index, likely a project-scoped-API-key view of the same underlying resources.
- This whole category is a distinct "admin tier" — worth its own top-level module group, separate from the "call the model" tier, when the crate's structure is redesigned.

### Chat — *Partial*
- `POST /v1/chat/completions` **[confirmed]** — create a chat completion. *(rs_openai: implemented, `chat` module.)*
- `GET /v1/chat/completions` **[confirmed]** — retrieve a stored chat completion (only works when the original request set `store: true`). *(rs_openai: missing.)*
- Chat Completions streaming events — SSE delta event schema. *(rs_openai: implemented for the create path.)*

### Completions (legacy) — *Implemented*
- `POST /v1/completions` — create a legacy text completion. *(rs_openai: implemented, `completions` module. Legacy on OpenAI's side; low priority beyond what already exists.)*

### Audio — *Partial*
- `POST /v1/audio/speech` **[confirmed]** — text to speech. *(rs_openai: implemented.)*
- `POST /v1/audio/transcriptions` **[confirmed]** — speech to text. *(rs_openai: implemented.)*
- `POST /v1/audio/translations` **[confirmed]** — speech to English text. *(rs_openai: implemented.)*
- **Audio Voice Consents** *(rs_openai: missing entirely)*: `POST /v1/audio/voice_consents` **[confirmed path prefix]** create, `GET` list, `GET /{consent_id}` retrieve, `POST /{consent_id}` update, `DELETE /{consent_id}` delete — consent records for voice cloning/custom-voice features.
- **Audio Voices** *(rs_openai: missing)*: `POST /v1/audio/voices` **[confirmed path prefix]** create — custom voice registration.

### Embeddings — *Implemented*
- `POST /v1/embeddings` — create embeddings. *(rs_openai: implemented. Check field coverage — e.g. `dimensions` — during implementation.)*

### Images — *Partial*
- `POST /v1/images/generations` create, `POST /v1/images/edits` edit, `POST /v1/images/variations` create variation. *(rs_openai: all three implemented.)*
- Image edit streaming events / Image generation streaming events *(rs_openai: missing)* — partial-image SSE events for `gpt-image-1`-style progressive generation.

### Files — *Implemented*
- `POST /v1/files` create, `GET /v1/files` list, `GET /v1/files/{file_id}` retrieve, `DELETE /v1/files/{file_id}` delete. *(rs_openai: implemented, `files` module — note `retrieve_content` also exists in the crate today though not separately listed in the reference index as its own page.)*

### Uploads — *Implemented*
- `POST /v1/uploads` create, `POST /v1/uploads/{upload_id}/cancel` cancel, `POST /v1/uploads/{upload_id}/parts` parts create. *(rs_openai: implemented, `uploads` module.)*

### Batches — *Implemented*
- `POST /v1/batches` **[confirmed]** create, `GET /v1/batches` **[confirmed]** list, `GET /v1/batches/{batch_id}` **[confirmed]** retrieve, `POST /v1/batches/{batch_id}/cancel` **[confirmed]** cancel. *(rs_openai: implemented, `batch` module.)*

### Fine Tuning — *Partial*
- `GET /v1/fine_tuning/jobs` list (create/retrieve/cancel also exist per the crate today, though the index page only separately calls out "list"). *(rs_openai: implemented, `fine_tuning` module.)*
- `GET /v1/fine_tuning/jobs/{job_id}/checkpoints` — list checkpoints. *(rs_openai: implemented.)*
- **Fine Tuning Checkpoints Permissions** *(rs_openai: missing)*: create/delete — manage which organizations can use a fine-tuned checkpoint.

### Models — *Implemented*
- `GET /v1/models` list, `GET /v1/models/{model}` retrieve, `DELETE /v1/models/{model}` delete. *(rs_openai: implemented.)*

### Moderations — *Implemented*
- `POST /v1/moderations` create. *(rs_openai: implemented.)*

### Assistants (Beta) — *Implemented, deprecated on OpenAI's side (sunsets 2026-08-26)*
- **Assistants**: `POST /v1/assistants` **[confirmed]** create, `GET /v1/assistants` **[confirmed]** list, `GET /v1/assistants/{assistant_id}` **[confirmed]** retrieve, `POST /v1/assistants/{assistant_id}` **[confirmed]** update, `DELETE /v1/assistants/{assistant_id}` **[confirmed]** delete. *(rs_openai: implemented, `assistants` module.)*
- **Threads**: create/delete/retrieve/update. *(rs_openai: implemented, `threads` module.)*
- **Threads Messages**: create/delete/list/retrieve/update. *(rs_openai: implemented, `messages` module.)*
- **Threads Runs**: cancel/create/list/retrieve/update, plus **Submit Tool Outputs**. *(rs_openai: `runs` module has create/retrieve/update/delete but no `delete` in OpenAI's real API for runs and no **submit tool outputs** endpoint — real gap, since submitting tool outputs is required for any run that uses function calling.)*
- **Threads Runs Steps**: list/retrieve — inspecting the individual steps a run took. *(rs_openai: missing entirely — no "run steps" concept in the crate today.)*
- Assistants streaming events — SSE event schema for assistant/run streaming. *(rs_openai: missing.)*
- **Recommendation carried over from Phase 1 planning:** don't invest further in filling these gaps — deprecate in place, point users at Responses + Conversations instead, per the sunset date.

## Biggest gap, in one sentence

`rs_openai` currently has zero coverage of Responses, Conversations, Vector Stores, ChatKit, Containers, Evals, Realtime, Videos, Webhooks, or the entire Organization/admin tier — together roughly half of OpenAI's current documented API surface, including the one API (Responses) OpenAI now says to use for all new projects.
