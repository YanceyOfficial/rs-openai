# Responses: `code_interpreter` Streaming Events — Design

Status: Approved
Date: 2026-08-26

## Context

Fifth and final slice of the one-tool-at-a-time streaming expansion for already-implemented tool types. Closes streaming coverage for file_search, web_search, image_generation, mcp, and now code_interpreter -- every tool type this crate implements except `computer_use`, which is confirmed (by its absence from the full 60-variant union) to have no per-item streaming events of its own beyond the generic `output_item.added`/`.done`.

## Goals

Five new `ResponseStreamEvent` variants, confirmed via `gh api` against `openai-python`'s `response_code_interpreter_call_*_event.py`:

- `CodeInterpreterCallInProgress`/`Interpreting`/`Completed` -- standard `{item_id, output_index, sequence_number}` shape.
- `CodeInterpreterCallCodeDelta { delta: String, .. }` / `CodeInterpreterCallCodeDone { code: String, .. }` -- mirrors the existing `FunctionCallArgumentsDelta`/`Done` and `McpCallArguments*` shape.

## Non-goals

The remaining ~20 variants (content_part, refusal, reasoning, audio, and shell/custom-tool events -- this crate doesn't implement programmatic shell or custom tools) are deliberately out of scope; there is currently no further already-implemented tool type needing its own streaming slice.

## Design notes

Added immediately after `mcp`'s 8 variants. No `client.rs` changes. Updated `ResponseStreamEvent`'s doc comment to state that streaming coverage is now complete for every implemented tool type.

## Testing

One parametrized test for the 3 standard-shape variants, plus a dedicated test for the delta/done pair.

## Versioning

Purely additive.
