# Responses: `image_generation` Streaming Events — Design

Status: Approved
Date: 2026-08-26

## Context

Third slice of Responses' streaming event coverage, continuing the one-tool-at-a-time cadence.

## Goals

Four new `ResponseStreamEvent` variants, confirmed via `gh api` against `openai-python`'s `response_image_gen_call_{in_progress,generating,partial_image,completed}_event.py`:

- `ImageGenerationCallInProgress`, `ImageGenerationCallGenerating`, `ImageGenerationCallCompleted` -- the standard `{item_id, output_index, sequence_number}` shape already used for file_search/web_search.
- `ImageGenerationCallPartialImage` -- a richer shape: `partial_image_b64: String`, `partial_image_index: u32` (both required), plus 4 optional fields (`background`/`output_format`/`quality`/`size`, all `Option<String>`) describing the generation settings used for that partial image.

## Non-goals

mcp, code_interpreter, shell, custom-tool events (and the other remaining variants) — future slices.

## Design notes

Added immediately after `web_search`'s 3 variants. No `client.rs` changes.

## Testing

One parametrized test for the 3 standard-shape variants (mirroring file_search/web_search's test structure) plus a dedicated test for `PartialImage` covering all 4 optional fields.

## Versioning

Purely additive.
