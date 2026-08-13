# ChatKit (Beta) — Sessions + Minimal Threads — Design

Status: Approved
Date: 2026-08-14

## Context

ChatKit is the fourth of the 9 Phase 2a scaffolds to go from skeleton to working implementation — "the exact 3-tier example the project owner cited" when the whole restructuring project was originally motivated (Phase 2a's own research doc, `docs/superpowers/research/2026-07-31-openai-api-structure.md`). It lets a server mint short-lived credentials for OpenAI's embeddable chat UI widget, backed by an Agent Builder "workflow". Phase 2a already scaffolded the accessor structure with zero methods: `apis::chatkit::{mod.rs, sessions.rs, threads.rs}` — `ChatKit<'a>` → `.sessions()`/`.session(id)` and `.threads()`/`.thread(id)` → `.items()`, matching the established plural/singular accessor pattern.

Live research this session (WebFetch against `developers.openai.com`, 5 different reference pages tried) found a real, unusual asymmetry: **Sessions' create/cancel schema is fully and precisely documented** (every request and response field confirmed, including nested `workflow`/`chatkit_configuration`/`expires_after`/`rate_limits` objects) — but **Threads' own object fields and the Thread Items union are only partially documented**, with multiple reference pages explicitly noting fields as "not enumerated" or "not detailed in source." This is the first slice in the whole Phase 2b effort where research hit a genuine, unresolved documentation gap rather than eventually finding a complete schema. Per `AskUserQuestion` ("Sessions full + Threads minimal, defer Items (Recommended)"), this slice ships Sessions at full fidelity, Threads' collection operations (list/retrieve/delete) using only the handful of confirmed `ChatKitThread` fields, and defers the Thread Items sub-resource entirely to a future slice once its item-union shapes can be confirmed against a real live session.

**A new, ChatKit-specific requirement**: every ChatKit endpoint requires an `OpenAI-Beta: chatkit_beta=v1` header in addition to the standard bearer auth — no other module in this crate needs a custom header. This requires a small, deliberately scoped addition to `client.rs`: three new `chatkit_get`/`chatkit_post`/`chatkit_delete` methods that mirror the existing `get`/`post`/`delete` but inject the one extra header, rather than adding header-injection capability to the generic methods (which would risk leaking the beta header into every other module's requests, or complicate every other module's call sites for a need only ChatKit has).

**Two naming collisions, both handled by the crate's established convention**: Phase 2a already named the ID-scoped accessor structs `ChatKitSession<'a>` (in `apis::chatkit::sessions`) and `ChatKitThread<'a>` (in `apis::chatkit::threads`); this slice's new response-body interface types are named `ChatKitSession`/`ChatKitThread` too, matching OpenAI's own object names — the same pattern already resolved for `Response`/`VectorStore`/`Conversation` via ordinary qualified paths (`chatkit::ChatKitSession`, `chatkit::ChatKitThread`) inside `apis::chatkit`'s own method signatures once `pub use crate::interfaces::chatkit::*;` brings both into scope. Not a new pattern, just its fourth and fifth occurrence.

**A genuine type-reuse match**: ChatKit sessions' `expires_after` field (`{anchor: "created_at" (fixed), seconds: number}`) is confirmed to be an exact structural match for the crate's existing `shared::types::ExpiresAfter`/`ExpiresAfterAnchor` (used by Files/Uploads) — unlike Vector Stores' `expires_after`, which looked similar but genuinely differed (different anchor value, different duration unit) and was deliberately NOT reused. This one is reused directly.

## Goals

- **Sessions**: `ChatKitSessions::create()` (`POST /chatkit/sessions`), `ChatKitSession::cancel()` (`POST /chatkit/sessions/{id}/cancel`).
- `CreateChatKitSessionRequest`: `workflow: ChatKitWorkflowParam` (required), `user: String` (required), `chatkit_configuration: Option<ChatKitConfiguration>`, `expires_after: Option<shared::types::ExpiresAfter>` (reused directly), `rate_limits: Option<ChatKitRateLimits>`.
- `ChatKitWorkflowParam`: `id: String` (required), `state_variables: Option<HashMap<String, serde_json::Value>>` (confirmed "primitive-typed key-value pairs" — kept as `serde_json::Value` rather than `String` since the docs don't confirm values are string-only, unlike this crate's usual all-string `metadata` fields), `tracing: Option<ChatKitWorkflowTracing>`, `version: Option<String>`.
- `ChatKitWorkflowTracing`: `enabled: Option<bool>`.
- `ChatKitConfiguration`: `automatic_thread_titling: Option<ChatKitAutomaticThreadTitling>` (`{enabled: Option<bool>}`), `file_upload: Option<ChatKitFileUploadConfig>` (`{enabled, max_file_size: Option<u32>, max_files: Option<u32>}`), `history: Option<ChatKitHistoryConfig>` (`{enabled, recent_threads: Option<u32>}`).
- `ChatKitRateLimits`: `max_requests_per_1_minute: Option<u32>`.
- `ChatKitSession` (response, reused for both `create` and `cancel` since `cancel` returns the same shape per the confirmed method signature `cancel(session_id) -> ChatSession`): `id: String`, `object: String`, `client_secret: String`, `expires_at: i64`, `status: String` (kept a plain string despite a confirmed 3-value set — `active`/`expired`/`cancelled` — for uniformity with every other status field in this crate), `user: String`, `workflow: ChatKitWorkflowParam` (reused bidirectionally — the response's "resolved" workflow config was not documented as a structurally different shape from the request), `chatkit_configuration: ChatKitConfiguration`, `rate_limits: ChatKitRateLimits`, `max_requests_per_1_minute: u32`.
- **Threads (minimal)**: `ChatKitThreads::list()` (`GET /chatkit/threads`), `ChatKitThread::retrieve()` (`GET /chatkit/threads/{id}`), `ChatKitThread::delete()` (`DELETE /chatkit/threads/{id}`).
- `ListChatKitThreadsRequest`: `after: Option<String>`, `before: Option<String>`, `limit: Option<u32>` — only the three confirmed params (one source mentioned "2 additional params (not detailed in source)"; not modeled, matching this project's practice of not guessing at unconfirmed fields).
- `ChatKitThread`: `id: String`, `object: String`, `created_at: i64`, `status: String`, `title: Option<String>` (nullable — a thread may not yet be auto-titled) — only the fields confirmed across research; other fields the object likely has (e.g. `user`) were explicitly flagged as "not enumerated" by every source tried and are NOT guessed at.
- `ListChatKitThreadsResponse`: `object: String`, `data: Vec<ChatKitThread>`, `first_id: Option<String>`, `last_id: Option<String>`, `has_more: bool` — matches the crate's universal list-response shape already used by every other paginated resource (Vector Stores, Conversations, Responses' `list_input_items`).
- `DeleteChatKitThreadResponse`: `id: String`, `object: String`, `deleted: bool` — matches the confirmed shape and the crate's universal delete-response pattern.
- **`client.rs` addition**: `chatkit_get`/`chatkit_post`/`chatkit_delete` (mirroring `get`/`post`/`delete`, adding the `OpenAI-Beta: chatkit_beta=v1` header via a small private `chatkit_request` helper that wraps `openai_request`).

## Non-goals (explicit, deferred to later slices)

- **Thread Items** (`GET /chatkit/threads/{id}/items`) — the entire sub-resource, including the `ChatKitThreadUserMessageItem`/`ChatKitThreadAssistantMessageItem`/`ChatKitWidgetItem`/`ChatKitAttachment` union. None of these types' field lists were confirmed by any of the 5 reference sources tried this session. `apis::chatkit::threads::ChatKitThreadItems` stays a zero-method scaffold, unchanged from Phase 2a.
- **`ChatKitThread`'s unconfirmed fields** — anything beyond `id`/`object`/`created_at`/`status`/`title` is not modeled; if OpenAI's docs improve later (or a real API response reveals more), those can be added non-breakingly.
- **The "2 additional" unconfirmed query params** on the threads list endpoint.
- **`order` on the threads list endpoint** — thread items' list confirmed an `order` param, but threads' own list endpoint's param list (as fetched) didn't include it; not modeled without confirmation.

## Design

### File organization

New file `rs_openai/src/interfaces/chatkit.rs` (registered via `pub mod chatkit;` in `interfaces/mod.rs`) — holds both Sessions and Threads types in one file, matching the established convention that `interfaces::<module>` stays a single flat file even when the corresponding `apis::<module>` is split into sub-files (Vector Stores is the precedent: `apis::vector_stores::{mod.rs, files.rs}` but a single `interfaces::vector_stores`).

`apis::chatkit::sessions::{ChatKitSessions, ChatKitSession}` and `apis::chatkit::threads::{ChatKitThreads, ChatKitThread}` get real methods added to their existing Phase 2a scaffolds (both files already exist with the right struct shapes, just no methods). `apis::chatkit::threads::ChatKitThreadItems` stays untouched (still a zero-method scaffold, per Non-goals).

`client.rs`: add `chatkit_request`/`chatkit_get`/`chatkit_post`/`chatkit_delete` alongside the existing `openai_request`/`get`/`post`/`delete`.

### Sessions types

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowTracing {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitWorkflowParam {
    #[builder(into)]
    pub id: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_variables: Option<std::collections::HashMap<String, serde_json::Value>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracing: Option<ChatKitWorkflowTracing>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitAutomaticThreadTitling {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitFileUploadConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_file_size: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_files: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitHistoryConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub enabled: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub recent_threads: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitConfiguration {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub automatic_thread_titling: Option<ChatKitAutomaticThreadTitling>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_upload: Option<ChatKitFileUploadConfig>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub history: Option<ChatKitHistoryConfig>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ChatKitRateLimits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<u32>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateChatKitSessionRequest {
    pub workflow: ChatKitWorkflowParam,

    #[builder(into)]
    pub user: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub chatkit_configuration: Option<ChatKitConfiguration>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_after: Option<crate::shared::types::ExpiresAfter>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub rate_limits: Option<ChatKitRateLimits>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitSession {
    pub id: String,
    pub object: String,
    pub client_secret: String,
    pub expires_at: i64,
    pub status: String,
    pub user: String,
    pub workflow: ChatKitWorkflowParam,
    pub chatkit_configuration: ChatKitConfiguration,
    pub rate_limits: ChatKitRateLimits,
    pub max_requests_per_1_minute: u32,
}
```

### Threads types (minimal)

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListChatKitThreadsRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ChatKitThread {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub status: String,
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListChatKitThreadsResponse {
    pub object: String,
    pub data: Vec<ChatKitThread>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteChatKitThreadResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}
```

### `client.rs` addition

```rust
/// Same as `openai_request`, but adds the `OpenAI-Beta: chatkit_beta=v1` header every
/// ChatKit endpoint requires in addition to the standard auth/org/project headers.
fn chatkit_request(
    &self,
    method: Method,
    route: &str,
) -> Result<reqwest_middleware::RequestBuilder, OpenAIError> {
    Ok(self
        .openai_request(method, route)?
        .header("OpenAI-Beta", "chatkit_beta=v1"))
}

pub(crate) async fn chatkit_get<T, F>(&self, route: &str, query: &F) -> OpenAIResponse<T>
where
    T: DeserializeOwned + Debug,
    F: Serialize,
{
    let request = self.chatkit_request(Method::GET, route)?.query(query);
    Self::resolve_response(request).await
}

pub(crate) async fn chatkit_post<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
where
    T: DeserializeOwned + Debug,
    F: Serialize,
{
    let request = self.chatkit_request(Method::POST, route)?.json(json);
    Self::resolve_response(request).await
}

pub(crate) async fn chatkit_delete<T, F>(&self, route: &str, json: &F) -> OpenAIResponse<T>
where
    T: DeserializeOwned + Debug,
    F: Serialize,
{
    let request = self.chatkit_request(Method::DELETE, route)?.json(json);
    Self::resolve_response(request).await
}
```

These are additive, private-to-crate (`pub(crate)`) methods — no existing `get`/`post`/`delete` call site anywhere else in the crate changes, so this is zero-risk for every other module.

### `apis::chatkit` method wiring

```rust
// sessions.rs
impl<'a> ChatKitSessions<'a> {
    pub async fn create(&self, req: &chatkit::CreateChatKitSessionRequest) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai.chatkit_post("/chatkit/sessions", req).await
    }
}

impl<'a> ChatKitSession<'a> {
    pub async fn cancel(&self) -> OpenAIResponse<chatkit::ChatKitSession> {
        self.openai.chatkit_post(&format!("/chatkit/sessions/{}/cancel", self.id), &()).await
    }
}

// threads.rs
impl<'a> ChatKitThreads<'a> {
    pub async fn list(&self, req: &chatkit::ListChatKitThreadsRequest) -> OpenAIResponse<chatkit::ListChatKitThreadsResponse> {
        self.openai.chatkit_get("/chatkit/threads", req).await
    }
}

impl<'a> ChatKitThread<'a> {
    pub async fn retrieve(&self) -> OpenAIResponse<chatkit::ChatKitThread> {
        self.openai.chatkit_get(&format!("/chatkit/threads/{}", self.id), &()).await
    }

    pub async fn delete(&self) -> OpenAIResponse<chatkit::DeleteChatKitThreadResponse> {
        self.openai.chatkit_delete(&format!("/chatkit/threads/{}", self.id), &()).await
    }
}
```

Both `sessions.rs` and `threads.rs` currently have `#[allow(dead_code)]` on their collection structs (`ChatKitSessions`, `ChatKitThreads`) — per this project's established scaffold convention ("leaf structs with no accessor method reading their fields need it; structs with a real accessor method don't"), these become stale once real methods are added and must be removed in the same task, matching the recurring pattern already caught in Vector Stores/Conversations/Responses core.

## Testing

`wiremock` tests covering: `create` serializing a full `CreateChatKitSessionRequest` (workflow with `state_variables`/`tracing`/`version`, `chatkit_configuration`, `expires_after` reusing `shared::types::ExpiresAfter`, `rate_limits`) and asserting the `OpenAI-Beta: chatkit_beta=v1` header is actually sent (via `wiremock`'s header matcher, not just the body) and deserializing the full `ChatKitSession` response; a minimal `create` test with only the required `workflow`/`user` fields, proving the optional fields are correctly omitted; `cancel` deserializing a `ChatKitSession` with `status: "cancelled"`; `threads().list()` serializing `after`/`before`/`limit` as query params and deserializing a paginated response; `thread(id).retrieve()`; `thread(id).delete()`.

## Versioning

Purely additive at the crate level — new module, new types, new `client.rs` methods that don't touch any existing method's signature or behavior. Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
