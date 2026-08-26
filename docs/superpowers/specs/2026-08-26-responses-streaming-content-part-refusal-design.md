# Responses: `content_part`/`refusal` Streaming Events (+ logprobs) — Design

Status: Approved
Date: 2026-08-26

## Context

First cross-cutting (non-tool-specific) streaming slice, following the 5 tool-type slices. `content_part`/`refusal` govern how output text/refusal content parts stream, independent of which tool produced the output item.

## Goals

- `ResponseOutputContent` (already used for `ResponseOutputMessage.content`) gains a third variant, `ReasoningText { text: String }` (tag `reasoning_text`) -- confirmed via source that `content_part.added`/`.done`'s `part` field is `Union[ResponseOutputText, ResponseOutputRefusal, ReasoningText]`, the same tagged shape already modeled, extended per this project's established extensible-tagged-union pattern.
- `ContentPartAdded`/`ContentPartDone` (`response.content_part.added`/`.done`): `{content_index, item_id, output_index, part: ResponseOutputContent, sequence_number}`.
- `RefusalDelta`/`RefusalDone` (`response.refusal.delta`/`.done`): mirrors the existing `OutputTextDelta`/`Done` shape with a `content_index` field, confirmed via source.

## A genuine gap found while researching this slice, fixed in the same commit

Confirmed via source that the REAL `response.output_text.delta`/`.done` events (already shipped in this crate's core streaming slice) both carry a required `logprobs: List[Logprob]` field (empty array when not requested) that this crate never modeled -- meaning a caller who requested `include: ["message.output_text.logprobs"]` (shipped in the `include`-param slice earlier this session) had no way to actually read the logprobs back out; they were silently dropped by serde's default unknown-field behavior. Added `ResponseLogprob { token, logprob, top_logprobs: Option<Vec<ResponseLogprobTopLogprob>> }` and a `logprobs: Vec<ResponseLogprob>` field (with `#[serde(default)]`, so this crate's own pre-existing test fixtures that predate this field keep deserializing unchanged) to both `OutputTextDelta` and `OutputTextDone`.

## Non-goals

Reasoning text/summary delta+done events, audio events, shell/custom-tool events (~16 remaining variants) -- future slices if ever pursued; these are lower-value (reasoning summaries are rarely consumed programmatically, audio output is a narrower use case, shell/custom tools aren't implemented by this crate at all).

## Design notes

All changes in `interfaces/responses.rs`. No `client.rs` changes.

## Testing

Five new tests: `logprobs` deserializes on `OutputTextDone`; `content_part.added`/`.done` deserialize with an `OutputText` part; a `content_part` event carries the new `ReasoningText` variant; `refusal.delta`/`.done` deserialize.

## Versioning

Purely additive.
