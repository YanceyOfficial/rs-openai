# Responses: `file_search` Streaming Events — Design

Status: Approved
Date: 2026-08-26

## Context

First slice of Responses' large remaining streaming gap. `ResponseStreamEvent` is confirmed (via `gh api` against `openai-python`'s `types/responses/response_stream_event.py`) to be a 60-variant union; this crate currently models ~12 (lifecycle + text + function-call deltas). Per `AskUserQuestion`, adding coverage one tool type at a time rather than in bulk — `file_search` chosen first as the smallest, already-implemented tool type without its own streaming events yet.

## Goals

Three new `ResponseStreamEvent` variants, all confirmed via source to share one shape (`{item_id: String, output_index: u32, sequence_number: u64}`, `sequence_number` REQUIRED here — unlike the existing lifecycle events' `Option<u64>`):

- `FileSearchCallInProgress` (`response.file_search_call.in_progress`) — emitted when a file search call is initiated.
- `FileSearchCallSearching` (`response.file_search_call.searching`) — emitted while actively searching.
- `FileSearchCallCompleted` (`response.file_search_call.completed`) — emitted when results are found.

## Non-goals

The other ~57 remaining stream event variants (web_search/image_generation/mcp/code_interpreter/shell/custom-tool events, content_part, refusal, reasoning, audio) — explicitly deferred to future one-tool-at-a-time slices, per the user's chosen scoping approach.

## Design notes

- Added to the existing `ResponseStreamEvent` enum in `interfaces/responses.rs`, immediately after `FunctionCallArgumentsDone`.
- No `client.rs`/streaming-pipeline changes needed -- `create_with_stream` already forwards any tagged event generically; no exhaustive match exists anywhere in the crate over `ResponseStreamEvent`'s variants.
- Updated the enum's own doc comment to state the confirmed total variant count and the one-tool-at-a-time plan, so future slices don't need to re-derive this scope from scratch.

## Testing

One parametrized unit test deserializing all 3 variants (looping over `(json_type, expected_variant_name)` pairs, since the shape and assertions are identical across all three) plus a shared match arm asserting `item_id`/`sequence_number`.

## Versioning

Purely additive.
