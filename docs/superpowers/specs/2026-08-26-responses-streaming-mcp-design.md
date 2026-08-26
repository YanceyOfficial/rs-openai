# Responses: `mcp` Streaming Events — Design

Status: Approved
Date: 2026-08-26

## Context

Fourth slice of Responses' streaming event coverage, continuing the one-tool-at-a-time cadence. `mcp` covers two independent lifecycles -- an individual tool call, and listing a server's available tools -- both included in this slice since they're both part of the same `mcp` tool type.

## Goals

Eight new `ResponseStreamEvent` variants, confirmed via `gh api` against `openai-python`'s `response_mcp_{call,list_tools}_*_event.py`:

- `McpCallInProgress`/`McpCallCompleted`/`McpCallFailed` -- standard `{item_id, output_index, sequence_number}` shape.
- `McpCallArgumentsDelta { delta: String, .. }` / `McpCallArgumentsDone { arguments: String, .. }` -- mirrors the existing `FunctionCallArgumentsDelta`/`Done` shape exactly.
- `McpListToolsInProgress`/`McpListToolsCompleted`/`McpListToolsFailed` -- standard shape.

## Non-goals

code_interpreter, shell, custom-tool events (and the other remaining variants) — future slices. This closes the LAST already-implemented tool type's streaming gap except `code_interpreter`.

## Design notes

Added immediately after `image_generation`'s 4 variants. No `client.rs` changes.

## Testing

One parametrized test for the 6 standard-shape variants, plus a dedicated test for the two arguments delta/done variants (mirroring the existing `FunctionCallArgumentsDelta`/`Done` test pattern).

## Versioning

Purely additive.
