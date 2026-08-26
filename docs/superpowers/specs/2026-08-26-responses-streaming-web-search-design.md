# Responses: `web_search` Streaming Events — Design

Status: Approved
Date: 2026-08-26

## Context

Second slice of Responses' streaming event coverage, following the same one-tool-at-a-time cadence established by the `file_search` slice.

## Goals

Three new `ResponseStreamEvent` variants, confirmed via `gh api` against `openai-python`'s `response_web_search_call_{in_progress,searching,completed}_event.py`, all sharing the identical shape already used for `file_search`'s events (`{item_id: String, output_index: u32, sequence_number: u64}`):

- `WebSearchCallInProgress` (`response.web_search_call.in_progress`)
- `WebSearchCallSearching` (`response.web_search_call.searching`)
- `WebSearchCallCompleted` (`response.web_search_call.completed`)

## Non-goals

image_generation, mcp, code_interpreter (and the other ~50 remaining variants) — future one-tool-at-a-time slices.

## Design notes

Added immediately after `file_search`'s 3 variants in `ResponseStreamEvent`. No `client.rs` changes.

## Testing

One parametrized unit test mirroring `file_search`'s exact test structure.

## Versioning

Purely additive.
