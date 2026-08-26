# Responses: `include` Param — Design

Status: Approved
Date: 2026-08-26

## Context

Closes Responses' `include` gap: additional output data (file/web search results, image URLs, reasoning content, logprobs) a caller can opt into. Confirmed via `gh api` against `openai-python`'s `types/responses/response_includable.py`, `response_create_params.py`, `response_retrieve_params.py`, `input_item_list_params.py`.

## Goals

- `ResponseIncludable`: an 8-variant enum with dotted wire values (`file_search_call.results`, `web_search_call.results`, `web_search_call.action.sources`, `message.input_image.image_url`, `computer_call_output.output.image_url`, `code_interpreter_call.outputs`, `reasoning.encrypted_content`, `message.output_text.logprobs`) — each variant spelled out with explicit `#[serde(rename)]`/`#[strum(serialize)]` since the values aren't valid Rust-identifier snake_case.
- `CreateResponseRequest.include: Option<Vec<ResponseIncludable>>` — a **JSON body** field, so it serializes as a normal array with no query-string workaround needed.
- `retrieve()` gains a new `RetrieveResponseRequest { include: Option<Vec<ResponseIncludable>> }` param (previously `retrieve()` took none) — `include` here IS a query param, so it uses the established `to_query_pairs()`/`get_with_pairs` workaround. `include_obfuscation`/`starting_after` (confirmed via source to be streaming-resume params) are deliberately out of scope, since this crate's `retrieve()` always does a plain non-streaming fetch.
- `ListInputItemsRequest` gains the same `include` field, converted from plain `get()` query serialization to `to_query_pairs()` (needed the moment any field becomes a `Vec<T>`, matching the Conversations/Certificates precedent).

## Non-goals

- `include_obfuscation`, `starting_after` (streaming-resume) — a materially different feature (resuming an in-flight SSE stream), not part of "which fields does the response include."

## Design notes

- `retrieve()`'s signature change (`&self` → `&self, req: &RetrieveResponseRequest`) is a deliberate breaking change; only one pre-existing call site needed updating (test code).
- All new types added to `interfaces/responses.rs`, next to `CreateResponseRequest`/`ListInputItemsRequest`.

## Testing

Three new tests: `CreateResponseRequest.include` serializes as a JSON array in the body; `retrieve()`'s `include` sends the correct query param; `list_input_items()`'s `include` sends the correct query param.

## Versioning

One deliberate breaking change (`retrieve()`'s signature), otherwise purely additive.
