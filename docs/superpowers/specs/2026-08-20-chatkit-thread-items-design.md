# ChatKit: Thread Items + a `ChatKitThread.status` bug fix — Design

Status: Approved
Date: 2026-08-20

## Context

Closes ChatKit's deferred Thread Items sub-resource (originally deferred in the ChatKit slice because Threads/Thread Items stayed only partially documented across 5 doc pages tried). Research via the `gh api` SDK-source technique — the SAME technique that resolved this session's Organization/admin documentation gaps — resolves this one too: `types/beta/chatkit/` has a fully confirmed, generated type set for threads and thread items.

**A genuine, confirmed bug in the already-shipped ChatKit code, found while researching this slice**: `ChatKitThread.status` is currently modeled as a plain `String`. The confirmed source shows it is actually a 3-variant, `type`-discriminated tagged union: `{"type": "active"}`, `{"type": "locked", "reason": Option<String>}`, `{"type": "closed", "reason": Option<String>}`. A real API response for a locked/closed thread would currently fail to deserialize `ChatKitThread` entirely (and even for `active`, the crate loses the tagged-object structure, though the string `"active"` case happens to look plausible on a lossy read of the docs, it's still wrong — the wire value is an object, not a bare string). Fixed in the same change, since it's the same type/file and directly load-bearing for Thread Items (whose union references this same `ChatKitThread` object indirectly via the thread it belongs to, and the fix must land before any new Thread Items test can trust status deserialization).

## Goals

### `ChatKitThread.status` fix

```rust
#[derive(Debug, Deserialize, Clone, Serialize)]
#[serde(tag = "type")]
pub enum ChatKitThreadStatus {
    #[serde(rename = "active")]
    Active,
    #[serde(rename = "locked")]
    Locked { reason: Option<String> },
    #[serde(rename = "closed")]
    Closed { reason: Option<String> },
}
```

`ChatKitThread.status: String` becomes `ChatKitThread.status: ChatKitThreadStatus` — a deliberate, correctness-driven breaking change (the old type could never have deserialized a real `locked`/`closed` response, so there's no non-breaking fix).

### Thread Items (`ChatKitThreadItems::list_items`, `GET /chatkit/threads/{thread_id}/items`)

A 6-variant, `type`-discriminated union (`ChatKitThreadItem`), confirmed via source:

- `ChatKitThreadUserMessageItem` (`chatkit.user_message`): `id`, `attachments: Vec<ChatKitAttachment>`, `content: Vec<ChatKitUserMessageContent>` (2-variant union: `InputText { text }` / `QuotedText { text }`), `created_at: i64`, `inference_options: Option<ChatKitInferenceOptions>` (`model: Option<String>`, `tool_choice: Option<ChatKitToolChoice>` where `ChatKitToolChoice { id: String }`), `thread_id: String`.
- `ChatKitThreadAssistantMessageItem` (`chatkit.assistant_message`): `id`, `content: Vec<ChatKitResponseOutputText>` (`annotations: Vec<ChatKitAnnotation>` — 2-variant union: `File { source: { filename } }` / `Url { url }` —, `text: String`), `created_at: i64`, `thread_id: String`.
- `ChatKitWidgetItem` (`chatkit.widget`): `id`, `created_at: i64`, `thread_id: String`, `widget: String` (serialized widget payload, kept as a raw string per the source — not further parsed).
- `ChatKitClientToolCallItem` (`chatkit.client_tool_call`): `id`, `arguments: String` (JSON-encoded, raw), `call_id: String`, `created_at: i64`, `name: String`, `output: Option<String>` (JSON-encoded, raw), `status: String` (`"in_progress"`/`"completed"`, plain string per this crate's convention), `thread_id: String`.
- `ChatKitTaskItem` (`chatkit.task`): `id`, `created_at: i64`, `heading: Option<String>`, `summary: Option<String>`, `task_type: String` (`"custom"`/`"thought"`), `thread_id: String`.
- `ChatKitTaskGroupItem` (`chatkit.task_group`): `id`, `created_at: i64`, `tasks: Vec<ChatKitTaskGroupTask>` (`heading: Option<String>`, `summary: Option<String>`, `task_type: String`), `thread_id: String`.

`ListChatKitThreadItemsRequest`: `after: Option<String>`, `before: Option<String>`, `limit: Option<u32>`, `order: Option<SortOrder>`.
`ListChatKitThreadItemsResponse`: universal list shape, `data: Vec<ChatKitThreadItem>`.

`ChatKitThreadItems::list_items()` only — confirmed no create/retrieve/delete for individual items (matches the existing scaffold's own accurate doc comment).

## Non-goals

None — this fully resolves ChatKit's only remaining deferred piece.

## Design notes

- All new types appended to `interfaces/chatkit.rs` (matching the existing file's flat organization — ChatKit is a small enough module that it was never split into sub-files).
- `arguments`/`output`/`widget` stay raw `String` (JSON-encoded per source, not further typed) — matches this crate's established "don't over-model an opaque payload" convention (same reasoning as `EvalGraderMessage.content`).
- No `client.rs` changes — reuses the existing `chatkit_get`/`chatkit_post`/`chatkit_delete` beta-header-injecting methods.
- Every internally-tagged-union payload struct must NOT redeclare its own `type`/`object` field (matching the established Usage `UsageResult` precedent) — verified per variant.

## Testing

`wiremock` tests: `list_items` deserializing at least 3 different variant kinds in one response (proving the tagged union dispatches correctly, not just "deserializes something"), plus a dedicated test proving the `ChatKitThread.status` fix — deserializing a `locked` status with a `reason` populated (the exact case the old `String` type could never have handled).

## Versioning

One deliberate, correctness-driven breaking change (`ChatKitThread.status`); Thread Items itself is purely additive. No `client.rs` changes.
