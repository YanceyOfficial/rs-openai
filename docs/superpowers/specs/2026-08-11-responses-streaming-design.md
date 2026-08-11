# Responses API: Streaming (Text + Function-Calling Events) — Design

Status: Approved
Date: 2026-08-11

## Context

The Responses core slice (`docs/superpowers/specs/2026-08-07-responses-core-design.md`, merged `54f8114`) shipped `create`/`retrieve`/`delete`/`cancel`/`list_input_items`, non-streaming only, text + function-calling only. Its own Non-goals explicitly deferred streaming to a dedicated future slice, reasoning that exposing a `stream` field with no `create_with_stream` sibling to redirect a rejected `stream: true` request to would be confusing partial surface. This slice is that dedicated follow-up, chosen by explicit project-owner direction over switching to a different Phase 2a scaffold (Vector Stores, ChatKit, etc.) — the project-owner wants Responses itself developed out further before moving to other categories.

Live research this session confirmed Responses' streaming model is structurally different from Chat's: Chat streams a single uniform "delta chunk" shape repeatedly; Responses streams a large taxonomy of distinctly-shaped, `type`-discriminated SSE events (dozens total across lifecycle, per-output-item, per-content-part, and per-tool-type deltas). What was live-confirmed this session: the event-type naming convention is dot-separated (`response.output_item.added`, `response.content_part.done`, `response.function_call_arguments.done` — confirmed via the Python SDK class names shown in OpenAI's streaming guide, correcting an initial fetch that paraphrased some names with underscores); the existence of `response.created`, `response.in_progress`, `response.completed`, `response.failed`, `response.output_text.delta`, `response.output_item.added`/`.done`, `response.content_part.added`/`.done`, `response.function_call_arguments.done`, and a top-level `error` event. The full streaming-events reference page itself exceeded the fetch tool's hard size limit (over 10MB, apparently inlining every nested schema in the whole API) and could not be fetched at all, at any prompt. Per explicit project-owner scoping direction (`AskUserQuestion`, "Text + function-calling events only"), this slice covers only the event types relevant to the core slice's existing type surface — full field-by-field shapes for those events were NOT independently re-confirmed this session and rely on stable prior knowledge of OpenAI's streaming conventions (see the confidence note under Design below).

`response.incomplete` is not confirmed to exist as its own SSE event (unlike `.failed`, which is explicitly confirmed) — OpenAI may instead always terminate via `response.completed` with the final `Response.status` field set to `incomplete`. Per project-owner direction to develop Responses out more completely rather than leave a narrow gap, it is included in this design as a best-effort, symmetric variant alongside `.completed`/`.failed`. If wrong, this fails loudly (an `Err(OpenAIError::JSONDeserialize(_))` on an actual incomplete-terminated stream, since the tagged enum won't match any variant) rather than silently misparsing — an acceptable risk for a variant this cheap to add.

## Goals

- `CreateResponseRequest` gains `stream: Option<bool>`.
- `Responses::create()` starts rejecting `stream: true` (redirects the caller to `create_with_stream`, mirroring Chat's/Completions' existing `is_stream`-based rejection).
- New `Responses::create_with_stream()`: rejects `stream: false`/`None`, returns `Pin<Box<dyn Stream<Item = OpenAIResponse<ResponseStreamEvent>> + Send>>` via the crate's existing `OpenAI::post_stream`/`stream_sse` machinery (already fully generic over the deserialized event type — built for Chat, reused unchanged here, no `client.rs` changes needed).
- `ResponseStreamEvent`: one internally-tagged enum (`#[serde(tag = "type")]`) covering exactly the events below, using the crate's existing precedent for literal (non-case-converted) per-variant renames (`BatchEndpoint` in `interfaces/batch.rs` already does this for values like `/v1/responses`) to express the dotted type strings:
  - `response.created`, `response.in_progress`, `response.completed`, `response.failed`, `response.incomplete` — lifecycle events, each wrapping a snapshot of the existing non-streaming `Response` type (direct reuse, no duplication).
  - `response.output_item.added`, `response.output_item.done` — each wrapping the existing `ResponseOutputItem` (direct reuse).
  - `response.output_text.delta`, `response.output_text.done` — text streaming deltas for a message's `OutputText` content part.
  - `response.function_call_arguments.delta`, `response.function_call_arguments.done` — function-call argument streaming deltas.
  - `error` — top-level stream-level error (distinct from a `Response.error` carried inside a completed/failed response body), reusing the existing `ApiErrorDetail` shape (`shared/response_wrapper.rs`) rather than inventing a new error shape — same `message`/`param`/`code` fields and types, minus `r#type`, which is subsumed by the enum's own `"error"` discriminator.

## Non-goals (explicit, deferred to later slices)

- **`response.content_part.added`/`.done`** — excluded per explicit project-owner scope choice this round (`AskUserQuestion`, "Text + function-calling events only" over the broader "include content_part events too" alternative).
- **`response.output_text_annotation_added`** (annotation streaming) — the core slice already keeps `ResponseOutputContent::OutputText.annotations` as raw `Vec<serde_json::Value>`; no streaming support for them follows from that until annotations get a typed union.
- **`response.refusal.delta`/`.done`** — `Refusal` exists as a `ResponseOutputContent` variant in the core slice, but no streaming deltas for it this round.
- **Every event tied to a tool type this crate hasn't implemented**: `response.file_search_call.*`, `response.code_interpreter_call.*`, and the web_search/image_generation/mcp/computer_use equivalents — these arrive alongside their respective tool types in future slices, not before.
- **`response.reasoning_summary_part.*`/`response.reasoning_text.*`** — no `Reasoning` output item type exists in the crate yet.
- **Background-mode stream resumption** (`starting_after`/cursor-based reconnect) — the crate has no background-mode support at all yet (the core slice's own explicit non-goal); resumption is meaningless without it.

## Design

### File organization

All changes land in the two files the core slice already created: `rs_openai/src/interfaces/responses.rs` (new types) and `rs_openai/src/apis/responses.rs` (new `create_with_stream` method, `stream` validation in `create`). No new files, no `client.rs` changes — `post_stream`/`stream_sse<T>` are already fully generic.

### `CreateResponseRequest.stream` and the create/create_with_stream split

```rust
// Added to CreateResponseRequest, alongside its other Option fields:
#[serde(skip_serializing_if = "Option::is_none")]
pub stream: Option<bool>,
```

```rust
impl<'a> Responses<'a> {
    pub async fn create(&self, req: &responses::CreateResponseRequest) -> OpenAIResponse<responses::Response> {
        if is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is true, use Responses::create_with_stream".into(),
            ));
        }
        self.openai.post("/responses", req).await
    }

    pub async fn create_with_stream(
        &self,
        req: &responses::CreateResponseRequest,
    ) -> OpenAIResponse<Pin<Box<dyn Stream<Item = OpenAIResponse<responses::ResponseStreamEvent>> + Send>>> {
        if !is_stream(req.stream) {
            return Err(OpenAIError::InvalidArgument(
                "When stream is false, use Responses::create".into(),
            ));
        }
        self.openai.post_stream("/responses", req).await
    }
}
```

This is a byte-for-byte structural match of `Chat::create`/`create_with_stream` and `Completions::create`/`create_with_stream` (both in this crate already) — same `is_stream` utility (`shared/utils.rs`), same rejection wording pattern, same `post_stream` call.

### `ResponseStreamEvent`

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ResponseStreamEvent {
    #[serde(rename = "response.created")]
    Created {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.in_progress")]
    InProgress {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.completed")]
    Completed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.failed")]
    Failed {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.incomplete")]
    Incomplete {
        response: Response,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.added")]
    OutputItemAdded {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_item.done")]
    OutputItemDone {
        output_index: u32,
        item: ResponseOutputItem,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.delta")]
    OutputTextDelta {
        item_id: String,
        output_index: u32,
        content_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.output_text.done")]
    OutputTextDone {
        item_id: String,
        output_index: u32,
        content_index: u32,
        text: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.delta")]
    FunctionCallArgumentsDelta {
        item_id: String,
        output_index: u32,
        delta: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "response.function_call_arguments.done")]
    FunctionCallArgumentsDone {
        item_id: String,
        output_index: u32,
        arguments: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
    #[serde(rename = "error")]
    Error {
        message: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        param: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<serde_json::Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        sequence_number: Option<u64>,
    },
}
```

**Confidence note:** event *existence* and the dotted `type`-string naming convention were live-confirmed this session (see Context). The exact field lists on each variant (`item_id`/`output_index`/`content_index`/`sequence_number`, and whether lifecycle events wrap the full `Response` object) are stable prior knowledge of OpenAI's streaming conventions, not independently re-verified this session — the streaming-events reference page could not be fetched at all (see Context). Every `sequence_number` field is `Option<u64>`, not required, applying the same `ChatUsage`/`ResponseUsageTokenDetails` deserialization-fragility lesson from prior slices: if a real event omits this field, deserialization of that event must not fail outright. `Error`'s `message`/`param`/`code` field shapes are reused directly from `ApiErrorDetail` (`shared/response_wrapper.rs`) rather than invented — `param`/`code` are `serde_json::Value` there too, since OpenAI's error codes/params aren't always strings.

The enum derives `Serialize` in addition to `Deserialize`, matching every other response-shaped type already in this file (`Response`, `ResponseOutputItem`, etc., which all derive both) — it has no serialization call site this slice, since events only ever flow client-side (deserialized off the wire), never constructed and sent by this crate.

### Testing

`wiremock` tests covering, following the exact pattern already established by `Chat::create_with_stream`'s tests (`ResponseTemplate::new(200).set_body_raw(sse_body.into_bytes(), "text/event-stream")`, consumed via `StreamExt::next` to completion):

- `create` rejects `stream: true` (mirrors `create_rejects_stream_true` in `chat.rs`/`completions.rs`).
- `create_with_stream` rejects `stream: false`/absent (mirrors `create_with_stream_rejects_stream_false`).
- An end-to-end happy-path stream: a realistic multi-event SSE body covering `response.created` → `response.in_progress` → `response.output_item.added` → several `response.output_text.delta` chunks → `response.output_text.done` → `response.output_item.done` → `response.completed`, asserting the full ordered sequence of decoded variants and that the stream terminates cleanly (no trailing item) — the direct Responses-API analog of Chat's `create_with_stream_yields_deltas_and_terminates_after_done` test.
- A function-call-argument streaming test: `response.output_item.added` (a `FunctionCall` item) → several `response.function_call_arguments.delta` chunks → `response.function_call_arguments.done` → `response.output_item.done`, asserting the accumulated `arguments` string and that the tagged union correctly discriminates this event family from the text-delta family within the same stream.
- A top-level `error` event mid-stream, asserting it decodes to `ResponseStreamEvent::Error` with the expected `message`.
- A `response.failed` (or `.incomplete`) terminal event, asserting the wrapped `Response.status` is read correctly off the embedded snapshot.

## Versioning

Purely additive at the crate level (`CreateResponseRequest` gains one new optional field; `Responses` gains one new method; `ResponseStreamEvent` is a brand-new type with no existing consumers). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per this project's established practice of deferring version bumps to release time rather than per-slice.
