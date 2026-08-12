# Conversations API: Core (CRUD, No Items Sub-Resource) — Design

Status: Approved
Date: 2026-08-12

## Context

The third of Phase 2a's 9 scaffolded-but-empty categories to go from skeleton to working implementation, picked next per the dependency-ordered sequence established after the user's "一并做完吧" (get it all done, continuously) direction — Conversations unblocks Responses' `conversation` request parameter (still deferred, per the Responses core slice's own Non-goals) and is likely also a ChatKit dependency. Phase 2a already scaffolded `apis/conversations.rs` with zero-method `Conversations<'a>`/`Conversation<'a>` structs; `apis/mod.rs` and `client.rs` already have both fully wired (`client.conversations()`/`client.conversation(id)`).

Live research this session (WebFetch against `developers.openai.com`) confirmed: the `Conversation` response object's exact 4 fields (`id`, `object`, `created_at`, `metadata`); the `update` endpoint accepts exactly `metadata` and nothing else; the `create` endpoint's `items` field accepts a genuinely broad, ~30+-variant polymorphic union (`EasyInputMessage`/`Message`/`ResponseOutputMessage`/`FileSearchCall`/`ComputerCall`/`FunctionCall`/`FunctionCallOutput`/etc.) — the exact same item-type family the Responses core slice already scoped down to `ResponseInputItem` (`Message`/`FunctionCallOutput` only). Per explicit project-owner scoping direction (`AskUserQuestion`), this slice covers core CRUD only (`create`/`retrieve`/`update`/`delete`), deferring the `items` sub-resource (`GET/POST/GET/DELETE .../items`) entirely — its full research wasn't completed this session (the `list` fetch returned an incomplete field list, and the exact per-operation paths for creating/retrieving/deleting individual items weren't confirmed), and the primary use case this slice serves (mint a conversation ID, hand it to Responses' future `conversation` param) doesn't need item introspection.

## Goals

- `Conversations`: `create`.
- `Conversation` (ID-scoped): `retrieve`, `update`, `delete`.
- `CreateConversationRequest.items` reuses `crate::interfaces::responses::ResponseInputItem` directly — genuine shape/purpose identity (Conversations' `items` accepts the same input-item family Responses' `input` does, just documented as a much larger polymorphic union on OpenAI's side; this crate already scoped that family down to `Message`/`FunctionCallOutput` for Responses, and the same scope applies here for the same reason).
- `UpdateConversationRequest` accepts exactly `metadata` — confirmed this session, no other fields.

## Non-goals (explicit, deferred to later slices)

- **The `items` sub-resource** (list/create/retrieve/delete individual conversation items) — full research incomplete this session (the real item-list response shape and the broader ~30-variant item union weren't confirmed in enough detail to model safely), and not needed for this slice's primary purpose (supplying conversation IDs to Responses).
- Any item type beyond `ResponseInputItem`'s existing scope (`Message`/`FunctionCallOutput`) — matches the Responses core slice's own scope, not re-litigated here.

## Design

### File organization

All changes land in `rs_openai/src/apis/conversations.rs` (already scaffolded) and a new `rs_openai/src/interfaces/conversations.rs` (doesn't exist yet). Add `pub mod conversations;` to `interfaces/mod.rs` — sorts alphabetically before `pub mod embeddings;` and after... actually before `completions` (c-o-m-p vs c-o-n-v: `completions` < `conversations`), so it lands between `pub mod completions;` and `pub mod embeddings;`. `apis/mod.rs` and `client.rs` already have `conversations` fully wired (Phase 2a scaffold) — no changes needed to either.

### Core types

```rust
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<crate::interfaces::responses::ResponseInputItem>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Conversation {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteConversationResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### Accessor wiring

```rust
impl<'a> Conversations<'a> {
    pub async fn create(&self, req: &conversations::CreateConversationRequest) -> OpenAIResponse<conversations::Conversation> {
        self.openai.post("/conversations", req).await
    }
}

impl<'a> Conversation<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<conversations::Conversation> {
        self.openai.get(&format!("/conversations/{}", self.id), &()).await
    }

    pub async fn update(&self, req: &conversations::UpdateConversationRequest) -> OpenAIResponse<conversations::Conversation> {
        self.openai.post(&format!("/conversations/{}", self.id), req).await
    }

    pub async fn delete(&self) -> OpenAIResponse<conversations::DeleteConversationResponse> {
        self.openai.delete(&format!("/conversations/{}", self.id), &()).await
    }
}
```

**Naming note (same resolution already established twice for Responses and Vector Stores):** `interfaces::conversations::Conversation` collides with `apis::conversations::Conversation<'a>` (the ID-scoped accessor, fixed by Phase 2a's scaffold) once `apis/conversations.rs` re-exports `interfaces::conversations::*`. Resolved via qualified paths (`conversations::Conversation`) in `apis/conversations.rs`'s own method signatures — no renaming, ordinary Rust namespace disambiguation.

## Testing

`wiremock` tests covering: `create` serializing a request with `items` (reusing `ResponseInputItem::Message`) and deserializing the response; `create` with no `items` (metadata-only conversation, proving `items` is genuinely optional); `retrieve`; `update` proving only `metadata` serializes (no other fields exist to accidentally leak); `delete` mapping an API error.

## Versioning

Purely additive at the crate level (a previously-scaffolded, zero-method category now has real methods; `ResponseInputItem` is reused, not modified). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
