# Responses: Multimodal Input — Design

Status: Approved
Date: 2026-08-26

## Context

Closes Responses' highest-value remaining gap: `ResponseInputItem::Message.content` was a plain `String`, unable to represent image or file input. Confirmed via `gh api` against `openai-python`'s `types/responses/` (`easy_input_message_param.py`, `response_input_content_param.py`, `response_input_text_param.py`, `response_input_image_param.py`, `response_input_file_param.py`).

## Goals

- `ResponseInputMessageContent` (untagged 2-variant enum): `Text(String)` | `Items(Vec<ResponseInputContent>)` — confirmed via source (`EasyInputMessageParam.content: Union[str, ResponseInputMessageContentListParam]`). The two shapes are JSON-type-disjoint (bare string vs. array), so untagged deserialization can't mis-route, matching the same reasoning already applied to `mcp`'s `require_approval` union.
- `ResponseInputContent` (`type`-tagged 3-variant enum, mirroring the established `ResponseTool` pattern): `InputText(ResponseInputText { text })`, `InputImage(ResponseInputImage { detail: ImageDetail, file_id: Option<String>, image_url: Option<String> })`, `InputFile(ResponseInputFile { detail: Option<String>, file_data: Option<String>, file_id: Option<String>, file_url: Option<String>, filename: Option<String> })`.
- `ImageDetail` (4-variant `strum`+`serde` snake_case enum: `Low`/`High`/`Auto`/`Original`).
- `ResponseInputItem::Message.content` changes from `String` to `ResponseInputMessageContent` — a deliberate breaking change. `From<String>`/`From<&str>`/`From<Vec<ResponseInputContent>>` impls let existing plain-string call sites migrate with `.into()`.

## Non-goals

- Audio input (`ResponseInputAudioParam`) — deliberately deferred, matching this crate's standing pattern of scoping large unions down to the well-documented common (text/image/file) cases.
- The other ~30 `ResponseInputItemParam` variants (`Program`, `ShellCall`, `ApplyPatchCall`, etc.) — already deliberately out of scope per `ResponseInputItem`'s existing doc comment; unrelated to this slice, which only touches `Message.content`'s shape.
- `prompt_cache_breakpoint` on each content part — a niche prompt-caching knob, not part of "can the model see an image/file."

## Design notes

- All new types added to `interfaces/responses.rs`, immediately before `ResponseInputItem` (their only consumer).
- `ResponseInputContent`'s tagged-enum-of-newtype-variants pattern mirrors `ResponseTool` exactly (already proven to work with `#[serde(tag = "type")]` in this codebase).
- 4 pre-existing call sites (2 in `apis/responses.rs`, 2 in `apis/conversations.rs`, all test code) updated to `"...".into()`.

## Testing

Two new tests: a multimodal 3-part content list serializes with correct per-variant `type` tags (text/image/file), and a mocked `list_input_items` response deserializes a multimodal message item, verified field-by-field including reaching into the second content part.

## Versioning

One deliberate breaking change (`Message.content`'s type), otherwise purely additive.
