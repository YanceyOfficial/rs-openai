# Organization/Admin: Admin API Keys + Audit Logs (sub-slice 1 of 4) — Design

Status: Approved
Date: 2026-08-19

## Context

Ninth and final Phase 2a scaffold to go from skeleton to working implementation — but large enough (11 sub-resources: admin_api_keys, audit_logs, usage, users, invites, groups, roles, and a projects sub-tree with 5 more nested resources) that it needs its own decomposition, confirmed via `AskUserQuestion` into 4 sub-slices: (1) Admin API Keys + Audit Logs, (2) Users + Invites + Groups, (3) Usage, (4) Projects + its 5 nested sub-resources. This spec covers sub-slice 1.

**Authentication note, confirmed by research**: Organization/admin endpoints use the exact same Bearer-token mechanism as every other endpoint in this crate — just with a different key VALUE (an "Admin API Key", format `sk-admin-...`, created by an Organization Owner) instead of a regular project-scoped key. No `client.rs`/`OpenAI` struct changes are needed for authentication; a caller simply constructs `OpenAI::new("sk-admin-...")` the same way as any other key.

**Research method for this slice, worth recording**: `developers.openai.com`'s reference pages for this resource tree consistently 404'd via WebFetch (unlike every other resource researched this session), even though they appear in search results — likely a client-side-rendered docs site that WebFetch's simple HTML fetch can't execute. Pivoted to fetching OpenAI's own official Python SDK source directly from GitHub via `gh api repos/openai/openai-python/contents/...` — the SDK is machine-generated from OpenAI's actual OpenAPI spec ("File generated from our OpenAPI spec by Castiron"), making it a more authoritative, more reliably fetchable source than the docs site for this resource tree. This confirmed exact field names/types/optionality with higher confidence than any prose-guide fetch this session, including field details (e.g. `expires_in_seconds` on create, the exact `Owner` sub-object shape) that hadn't surfaced from doc-page attempts.

**A major, evidence-backed scoping decision**: Audit Logs' event objects are NOT a small tagged union — OpenAI's own generated types show a `type` field with **140+ possible string values** (including a large, actively-growing `tenant.*` enterprise-tier namespace) and **~90 separate per-event-type "detail" structs**, each event's detail payload appearing as its own optional top-level field on the SAME response object (not nested under a single `data` key), aliased via a dotted name (e.g. a field internally named `api_key_created` serializes to the JSON key `"api_key.created"`). Modeling this exhaustively would be, by a wide margin, the largest single-slice type-modeling effort in this entire project — for a union that's still actively growing with enterprise/tenant features likely irrelevant to most callers of this crate. Per this project's established and repeatedly-validated pattern (defer genuinely large/complex/actively-growing unions, fully type the common envelope), this slice fully types the confirmed common envelope (`id`, `effective_at`, `type`, `actor`, `project`) and captures the event-type-specific detail fields via `#[serde(flatten)] detail: HashMap<String, serde_json::Value>` — **the first use of `#[serde(flatten)]` in this crate**, chosen specifically because it's the correct tool for "a sparse set of optional top-level keys, exactly one of which is populated depending on a discriminator field elsewhere in the same object" — a shape this project hasn't encountered before (every prior tagged union nested its variant payload under the object itself via `#[serde(tag = "type")]`, not as sibling flat keys).

**Audit log list filtering is also scoped down**: the confirmed query parameters include `actor_emails`/`actor_ids`/`event_types`/`project_ids`/`resource_ids` (all `Vec<String>`) and `effective_at` (a nested `{gt, gte, lt, lte}` range-filter object). This crate has never sent a `Vec<String>` or a nested-object query parameter before — every existing `List*Request` uses only flat scalar fields (`after`/`limit`/`order`), which `reqwest`'s `.query()` (backed by `serde_urlencoded`) is confirmed to handle correctly. Whether `serde_urlencoded` correctly repeats keys for `Vec<String>` fields, and how it would even attempt to serialize a nested object, are both genuinely unconfirmed for this crate's query-serialization pipeline. This slice models only the confirmed-safe pagination params (`after`, `before`, `limit`) — matching every List*Request's already-proven pattern — and defers the array/range filters to a future enhancement once query-serialization for compound types is verified.

## Goals

- **Admin API Keys**: `OrganizationAdminApiKeys::create()` (`POST /organization/admin_api_keys`), `::list()` (`GET /organization/admin_api_keys`), `OrganizationAdminApiKey::retrieve()` (`GET /organization/admin_api_keys/{key_id}`), `::delete()` (`DELETE /organization/admin_api_keys/{key_id}`).
- `CreateAdminApiKeyRequest`: `name: String` (required), `expires_in_seconds: Option<i64>`.
- `AdminApiKeyOwner`: `id: Option<String>`, `created_at: Option<i64>`, `name: Option<String>`, `object: Option<String>` (always `"organization.user"`), `role: Option<String>` (always `"owner"`), `kind: Option<String>` (`#[serde(rename = "type")]`, always `"user"`) — all confirmed `Optional` in the source, kept as such even though every field is presumably always populated in practice, since the source's own optionality is the ground truth here, not a guess.
- `AdminApiKey` (response, used uniformly for create/retrieve/list — the `value` field being create-only is a purely additive difference, not a shape swap, so one struct suffices): `id: String`, `created_at: i64`, `expires_at: Option<i64>`, `object: String` (always `"organization.admin_api_key"`), `owner: AdminApiKeyOwner` (required), `redacted_value: String`, `last_used_at: Option<i64>`, `name: Option<String>`, `value: Option<String>` (populated only in the create response, confirmed via the source's own doc comment: "Only shown on create").
- `ListAdminApiKeysRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>` — matches the confirmed SDK method signature exactly.
- `ListAdminApiKeysResponse`/`DeleteAdminApiKeyResponse`: the crate's universal list-response/delete-response shapes (confirmed: `DeleteAdminApiKeyResponse` is exactly `{id, deleted, object}`).
- **Audit Logs**: `OrganizationAuditLogs::list()` (`GET /organization/audit_logs`) — the only operation; this resource is list-only, per the existing scaffold's own accurate doc comment.
- `ListAuditLogsRequest`: `after: Option<String>`, `before: Option<String>`, `limit: Option<u32>` — the confirmed-safe subset (array/range filters deferred, see Context).
- `AuditLogActorApiKeyServiceAccount`: `id: Option<String>`.
- `AuditLogActorUser` (shared — reused for both `AuditLogActorApiKey.user` and `AuditLogActorSession.user`, which have identical confirmed shapes in the source despite being separate Python classes there): `id: Option<String>`, `email: Option<String>`.
- `AuditLogActorApiKey`: `id: Option<String>`, `kind: Option<String>` (`#[serde(rename = "type")]`, `"user"`/`"service_account"`), `service_account: Option<AuditLogActorApiKeyServiceAccount>`, `user: Option<AuditLogActorUser>`.
- `AuditLogActorSession`: `ip_address: Option<String>`, `user: Option<AuditLogActorUser>`.
- `AuditLogActor`: `kind: Option<String>` (`#[serde(rename = "type")]`, `"session"`/`"api_key"`), `api_key: Option<AuditLogActorApiKey>`, `session: Option<AuditLogActorSession>`.
- `AuditLogProject`: `id: Option<String>`, `name: Option<String>`.
- `AuditLogEvent`: `id: String`, `effective_at: i64`, `kind: String` (`#[serde(rename = "type")]`, plain string NOT a closed enum — 140+ actively-growing values, confirmed), `actor: Option<AuditLogActor>`, `project: Option<AuditLogProject>`, `#[serde(flatten)] detail: std::collections::HashMap<String, serde_json::Value>` (captures whichever single event-type-specific key is present, e.g. `"api_key.created": {"id": ..., "data": {...}}` — and is automatically forward-compatible with any future event type OpenAI adds, since new keys just land in the map).
- `ListAuditLogsResponse`: the crate's universal list-response shape, `data: Vec<AuditLogEvent>`.

## Non-goals (explicit, deferred to later slices)

- **Per-event-type detail structs** (the ~90 types like `APIKeyCreated`/`InviteSent`/`RoleAssignmentCreated`/etc.) — captured generically via the flattened map instead. A caller who needs a specific event's detail fields reads them from `event.detail.get("api_key.created")` and parses as needed.
- **Audit log array/range query filters** (`actor_emails`, `actor_ids`, `event_types`, `project_ids`, `resource_ids`, `effective_at` range, `tenant_only`) — deferred pending confirmation of how this crate's query-serialization pipeline handles `Vec<String>` and nested-object query params (never exercised before).
- **The rest of Organization/admin** (usage, users, invites, groups, roles, projects + its sub-tree) — sub-slices 2-4, not this one.

## Design

### File organization

New file `rs_openai/src/interfaces/organization/admin_api_keys.rs` and `rs_openai/src/interfaces/organization/audit_logs.rs` (new `interfaces/organization/` directory, registered via `pub mod organization;` in `interfaces/mod.rs` and `pub mod admin_api_keys; pub mod audit_logs;` inside `interfaces/organization/mod.rs`) — matching the established convention that `interfaces::<module>` mirrors `apis::<module>`'s directory structure when the apis side is itself split into multiple files (here, `apis::organization` is already split into `admin_api_keys.rs`/`audit_logs.rs`/etc.), same precedent as `apis::organization::projects` being its own sub-directory.

### Admin API Keys types

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateAdminApiKeyRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_in_seconds: Option<i64>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AdminApiKeyOwner {
    pub id: Option<String>,
    pub created_at: Option<i64>,
    pub name: Option<String>,
    pub object: Option<String>,
    pub role: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AdminApiKey {
    pub id: String,
    pub created_at: i64,
    pub expires_at: Option<i64>,
    pub object: String,
    pub owner: AdminApiKeyOwner,
    pub redacted_value: String,
    pub last_used_at: Option<i64>,
    pub name: Option<String>,
    pub value: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListAdminApiKeysRequest {
    #[builder(into)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListAdminApiKeysResponse {
    pub object: String,
    pub data: Vec<AdminApiKey>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteAdminApiKeyResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

### Audit Logs types

```rust
#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct ListAuditLogsRequest {
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
pub struct AuditLogActorApiKeyServiceAccount {
    pub id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorUser {
    pub id: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorApiKey {
    pub id: Option<String>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub service_account: Option<AuditLogActorApiKeyServiceAccount>,
    pub user: Option<AuditLogActorUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActorSession {
    pub ip_address: Option<String>,
    pub user: Option<AuditLogActorUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogActor {
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub api_key: Option<AuditLogActorApiKey>,
    pub session: Option<AuditLogActorSession>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogProject {
    pub id: Option<String>,
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct AuditLogEvent {
    pub id: String,
    pub effective_at: i64,
    #[serde(rename = "type")]
    pub kind: String,
    pub actor: Option<AuditLogActor>,
    pub project: Option<AuditLogProject>,
    #[serde(flatten)]
    pub detail: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListAuditLogsResponse {
    pub object: String,
    pub data: Vec<AuditLogEvent>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
```

### Naming

No naming collision this slice — `OrganizationAdminApiKey<'a>` (apis accessor) and `AdminApiKey` (interfaces response type) don't share a bare name (unlike `Response`/`Eval`/`Video`/etc., which matched OpenAI's own object name exactly), so no qualified-path resolution is needed here.

## Testing

`wiremock` tests covering: `create` serializing `name`+`expires_in_seconds` and deserializing an `AdminApiKey` with `value` populated; a minimal `create` test with only `name`; `list` serializing `after`/`limit`/`order` and deserializing a paginated response; `retrieve` deserializing an `AdminApiKey` with `value` absent (confirming the create-only field is correctly `None` elsewhere); `delete`; audit logs `list` serializing `after`/`before`/`limit` and deserializing a response containing an `AuditLogEvent` with `actor`/`project` populated and a `#[serde(flatten)]`-captured event-specific detail key (e.g. `"api_key.created": {"id": "key_1", "data": {"scopes": ["api.model.request"]}}`), asserting the detail is retrievable from the `detail` HashMap by its dotted key; a second audit log test with a DIFFERENT event type's detail key present, proving the flatten approach genuinely generalizes across event types rather than being hardcoded to one.

## Versioning

Purely additive at the crate level — new module, new types, no client method changes (existing Bearer-auth `get`/`post`/`delete` are reused as-is; an Admin API Key works through the exact same auth path as any other key). Ships as a pre-1.0 minor bump alongside whatever else accumulates before the next release, per established practice.
