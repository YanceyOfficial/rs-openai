# Responses: `reasoning`/`audio` Streaming Events — Design

Status: Approved
Date: 2026-08-28

## Context

Closes the last item in the README's "Known gaps" list. Reasoning and audio events are cross-cutting (not tied to a specific tool type), unlike the 5 tool-type streaming slices shipped earlier this session. Confirmed via `gh api` against `openai-python`'s `types/responses/response_reasoning_*_event.py` and `response_audio_*_event.py`.

## Goals

### Reasoning (6 new `ResponseStreamEvent` variants)

- `ReasoningTextDelta` (`response.reasoning_text.delta`): `{content_index, delta, item_id, output_index, sequence_number}`.
- `ReasoningTextDone` (`response.reasoning_text.done`): `{content_index, item_id, output_index, sequence_number, text}`.
- `ReasoningSummaryPartAdded` / `ReasoningSummaryPartDone` (`response.reasoning_summary_part.added`/`.done`): `{item_id, output_index, part: ReasoningSummaryPart, sequence_number, summary_index}` -- `PartDone` additionally carries `status: Option<String>` (`"incomplete"` when generation was interrupted, otherwise omitted).
- `ReasoningSummaryTextDelta` / `ReasoningSummaryTextDone` (`response.reasoning_summary_text.delta`/`.done`): standard delta/done shape plus `summary_index`.
- New type `ReasoningSummaryPart` -- a `type`-tagged extensible enum (matching this crate's established pattern) with one confirmed variant, `SummaryText { text: String }` (tag `summary_text`).

### Audio (4 new `ResponseStreamEvent` variants)

- `AudioDelta` (`response.audio.delta`): `{delta, sequence_number}` -- `delta` is base64-encoded audio bytes.
- `AudioDone` (`response.audio.done`): `{sequence_number}`.
- `AudioTranscriptDelta` (`response.audio.transcript.delta`): `{delta, sequence_number}`.
- `AudioTranscriptDone` (`response.audio.transcript.done`): `{sequence_number}`.

## Non-goals

Shell and custom-tool streaming events -- this crate implements neither tool type (no `ShellCall`/`CustomToolCall` request-side support exists anywhere), so their events would be permanently unreachable. This closes the LAST item in the README's "Known gaps" list; no further Responses streaming work is anticipated after this slice.

## Design notes

All new types added to `interfaces/responses.rs`, appended to the existing `ResponseStreamEvent` enum. No `client.rs` changes.

## Testing

3 new tests: reasoning text delta/done deserialize; reasoning summary part added/done deserialize (including the `SummaryText` part variant and the optional `status` field); reasoning summary text delta/done deserialize; audio delta/done/transcript-delta/transcript-done deserialize (parametrized, since all 4 share a trivial shape).

## Versioning

Purely additive.
