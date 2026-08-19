# Organization Admin API Keys + Audit Logs Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement Organization Admin API Keys (full CRUD) and Audit Logs (list-only) — sub-slice 1 of 4 for the Organization/admin scaffold, the ninth and final Phase 2a category.

**Architecture:** New nested directory `rs_openai/src/interfaces/organization/` (mirroring `apis::organization`'s existing multi-file split) holds `admin_api_keys.rs` and `audit_logs.rs`. Unlike smaller single-file modules elsewhere in this crate, `interfaces::organization` does NOT get a flat glob re-export at its own top level — with 11 total sub-resources planned across all 4 sub-slices, nested module paths (`rs_openai::organization::admin_api_keys::AdminApiKey`) are kept deliberately, avoiding a single crowded flat namespace. Real methods get added to the existing `apis::organization::{admin_api_keys, audit_logs}` scaffold files, each importing its OWN interfaces submodule directly (`use crate::interfaces::organization::admin_api_keys;`) and referencing types via qualified paths — the same pattern already used by every multi-file `apis` module in this crate (e.g. `apis::vector_stores::files.rs`).

**Tech Stack:** Rust, `serde` (including `#[serde(flatten)]` on a `HashMap<String, serde_json::Value>` field — the first use of this pattern in the crate, empirically verified in a scratch project before this plan was written), `bon::Builder`, `wiremock`.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md`. This is sub-slice 1 of 4 for Organization/admin (confirmed decomposition via `AskUserQuestion`); sub-slices 2-4 (Users+Invites+Groups, Usage, Projects+sub-tree) are explicitly out of scope for this plan.
- No `client.rs` changes — Admin API Keys use the exact same Bearer-auth `get`/`post`/`delete` methods as every other resource; only the KEY VALUE differs (an `sk-admin-...` key vs. a regular key), which is entirely the caller's concern at `OpenAI::new(...)` construction time, not this crate's.
- `AdminApiKey` is ONE struct used uniformly for create/retrieve/list responses (not a separate `AdminApiKeyCreateResponse` type) — `value: Option<String>` is populated only on create and `None` elsewhere, a purely additive difference confirmed via OpenAI's own Python SDK source, not a shape swap requiring separate types (contrast with the Evals slice's `EvalDataSourceConfig`, which DID need a request/response split because that was a genuine shape difference).
- `AuditLogEvent.kind` (`#[serde(rename = "type")]`) stays a plain `String`, NOT a closed enum — confirmed 140+ actively-growing values from OpenAI's own generated types, do not attempt to enumerate them.
- `AuditLogEvent`'s event-type-specific detail fields (the ~90 per-event-type structs like `APIKeyCreated`/`InviteSent`) are NOT modeled individually — captured via `#[serde(flatten)] detail: HashMap<String, serde_json::Value>`, which lets a caller look up `event.detail.get("api_key.created")` themselves. Do not add individual typed fields for specific event types.
- `ListAuditLogsRequest` gets ONLY `after`/`before`/`limit` — the array-valued filters (`actor_emails`, `actor_ids`, `event_types`, `project_ids`, `resource_ids`) and the nested `effective_at` range object are explicitly deferred (this crate's query-serialization pipeline has never been exercised with `Vec<String>` or nested-object query params, and correctness there is unconfirmed). Do not add them.
- `AuditLogActorUser` is shared between `AuditLogActorApiKey.user` and `AuditLogActorSession.user` (both confirmed to have the identical `{id, email}` shape in OpenAI's source, even though the source itself defines them as two separate Python classes) — do not duplicate this type.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.

---

### Task 1: Admin API Keys (create/list/retrieve/delete)

**Files:**
- Modify: `rs_openai/src/interfaces/mod.rs` (add `pub mod organization;`, alphabetically between `moderations` and `responses`)
- Create: `rs_openai/src/interfaces/organization/mod.rs` (`pub mod admin_api_keys;` — `audit_logs` added in Task 2)
- Create: `rs_openai/src/interfaces/organization/admin_api_keys.rs`
- Modify: `rs_openai/src/apis/organization/admin_api_keys.rs` (add methods, remove stale `#[allow(dead_code)]`, update doc comments)
- Test: `rs_openai/src/apis/organization/admin_api_keys.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::{OrganizationAdminApiKeys, OrganizationAdminApiKey}` struct shapes from Phase 2a (`OrganizationAdminApiKeys<'a> { openai: &'a OpenAI }`, `OrganizationAdminApiKey<'a> { openai: &'a OpenAI, id: String }`, unchanged); existing `crate::interfaces::chat::SortOrder`.
- Produces: `interfaces::organization::admin_api_keys::{CreateAdminApiKeyRequest, AdminApiKeyOwner, AdminApiKey, ListAdminApiKeysRequest, ListAdminApiKeysResponse, DeleteAdminApiKeyResponse}`, `OrganizationAdminApiKeys::create`/`::list`, `OrganizationAdminApiKey::retrieve`/`::delete` — Task 2 (Audit Logs) is structurally independent and doesn't consume anything from this task, but the final cross-check does.

- [ ] **Step 1: Write the failing test — `create` serializes the full request and deserializes the response with `value` populated**

Add a test module to `rs_openai/src/apis/organization/admin_api_keys.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::admin_api_keys::{
        CreateAdminApiKeyRequest, ListAdminApiKeysRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response_with_value() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/admin_api_keys"))
            .and(body_json(serde_json::json!({
                "name": "CI deploy key",
                "expires_in_seconds": 3600
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1711471533_i64,
                "expires_at": 1711475133_i64,
                "object": "organization.admin_api_key",
                "owner": {
                    "id": "user_1",
                    "created_at": 1700000000_i64,
                    "name": "Jane Admin",
                    "object": "organization.user",
                    "role": "owner",
                    "type": "user"
                },
                "redacted_value": "sk-admin...abcd",
                "last_used_at": null,
                "name": "CI deploy key",
                "value": "sk-admin-1234abcd"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = CreateAdminApiKeyRequest::builder()
            .name("CI deploy key")
            .expires_in_seconds(3600)
            .build();

        let res = client.organization().admin_api_keys().create(&req).await.unwrap();
        assert_eq!(res.id, "key_1");
        assert_eq!(res.value.as_deref(), Some("sk-admin-1234abcd"));
        assert_eq!(res.owner.id.as_deref(), Some("user_1"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response_with_value 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::organization` module doesn't exist yet, `client.organization().admin_api_keys().create(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/organization/mod.rs`**

```rust
//! Organization-level administration types -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

pub mod admin_api_keys;
```

- [ ] **Step 4: Create `rs_openai/src/interfaces/organization/admin_api_keys.rs`**

```rust
//! Organization admin API keys -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

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

- [ ] **Step 5: Register `interfaces::organization`**

In `rs_openai/src/interfaces/mod.rs`, add this new line alphabetically, between the existing `pub mod moderations;` and `pub mod responses;` lines:

```rust
pub mod organization;
```

- [ ] **Step 6: Implement `OrganizationAdminApiKeys::create`/`::list` and `OrganizationAdminApiKey::retrieve`/`::delete`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/organization/admin_api_keys.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
//! Organization admin API keys (`/organization/admin_api_keys`) -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::admin_api_keys;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationAdminApiKeys<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAdminApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Creates an organization admin API key.
    pub async fn create(
        &self,
        req: &admin_api_keys::CreateAdminApiKeyRequest,
    ) -> OpenAIResponse<admin_api_keys::AdminApiKey> {
        self.openai.post("/organization/admin_api_keys", req).await
    }

    /// Lists organization admin API keys.
    pub async fn list(
        &self,
        req: &admin_api_keys::ListAdminApiKeysRequest,
    ) -> OpenAIResponse<admin_api_keys::ListAdminApiKeysResponse> {
        self.openai.get("/organization/admin_api_keys", req).await
    }
}

pub struct OrganizationAdminApiKey<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationAdminApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    /// Retrieves an organization admin API key by ID.
    pub async fn retrieve(&self) -> OpenAIResponse<admin_api_keys::AdminApiKey> {
        self.openai
            .get(&format!("/organization/admin_api_keys/{}", self.id), &())
            .await
    }

    /// Deletes an organization admin API key.
    pub async fn delete(&self) -> OpenAIResponse<admin_api_keys::DeleteAdminApiKeyResponse> {
        self.openai
            .delete(&format!("/organization/admin_api_keys/{}", self.id), &())
            .await
    }
}
```

Also update `rs_openai/src/apis/organization/mod.rs`'s doc comments for `admin_api_keys()`/`admin_api_key()` (leave every other accessor's "Not yet implemented" comment untouched — those belong to later sub-slices):

```rust
    /// Scoped accessor for organization admin API keys.
    pub fn admin_api_keys(&self) -> admin_api_keys::OrganizationAdminApiKeys<'a> {
        admin_api_keys::OrganizationAdminApiKeys::new(self.openai)
    }

    /// Scoped accessor for a specific admin API key.
    pub fn admin_api_key(
        &self,
        id: impl Into<String>,
    ) -> admin_api_keys::OrganizationAdminApiKey<'a> {
        admin_api_keys::OrganizationAdminApiKey::new(self.openai, id.into())
    }
```

- [ ] **Step 7: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai create_serializes_full_request_and_deserializes_response_with_value 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 8: Write the failing tests for `list`, `retrieve` (value absent), and `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/admin_api_keys"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "key_2",
                    "created_at": 1711471533_i64,
                    "expires_at": null,
                    "object": "organization.admin_api_key",
                    "owner": {
                        "id": "user_2",
                        "created_at": 1700000000_i64,
                        "name": "Service Bot",
                        "object": "organization.user",
                        "role": "owner",
                        "type": "user"
                    },
                    "redacted_value": "sk-admin...wxyz",
                    "last_used_at": 1711471600_i64,
                    "name": "Bot key",
                    "value": null
                }],
                "first_id": "key_2",
                "last_id": "key_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAdminApiKeysRequest::builder().limit(10).build();
        let res = client.organization().admin_api_keys().list(&req).await.unwrap();
        assert_eq!(res.data.len(), 1);
        assert!(res.data[0].value.is_none());
    }

    #[tokio::test]
    async fn retrieve_deserializes_key_with_value_absent() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/admin_api_keys/key_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_3",
                "created_at": 1711471533_i64,
                "expires_at": null,
                "object": "organization.admin_api_key",
                "owner": {
                    "id": "user_3",
                    "created_at": 1700000000_i64,
                    "name": "Jane Admin",
                    "object": "organization.user",
                    "role": "owner",
                    "type": "user"
                },
                "redacted_value": "sk-admin...efgh",
                "last_used_at": null,
                "name": "Retrieved key",
                "value": null
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .admin_api_key("key_3")
            .retrieve()
            .await
            .unwrap();
        assert_eq!(res.id, "key_3");
        assert!(res.value.is_none());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/admin_api_keys/key_3"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_3",
                "deleted": true,
                "object": "organization.admin_api_key.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let res = client
            .organization()
            .admin_api_key("key_3")
            .delete()
            .await
            .unwrap();
        assert!(res.deleted);
    }
```

- [ ] **Step 9: Run tests to verify they pass**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai admin_api_keys:: 2>&1 | tail -30`
Expected: all 4 PASS.

- [ ] **Step 10: Format, lint, full test run**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, all tests pass (193 + 4 new = 197).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/mod.rs rs_openai/src/interfaces/organization/mod.rs rs_openai/src/interfaces/organization/admin_api_keys.rs rs_openai/src/apis/organization/mod.rs rs_openai/src/apis/organization/admin_api_keys.rs
git commit -m "feat(organization): add admin API keys create/list/retrieve/delete"
```

---

### Task 2: Audit Logs (list-only) + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod audit_logs;`)
- Create: `rs_openai/src/interfaces/organization/audit_logs.rs`
- Modify: `rs_openai/src/apis/organization/audit_logs.rs` (add `list` method, remove stale `#[allow(dead_code)]`, update doc comments)
- Test: `rs_openai/src/apis/organization/audit_logs.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::OrganizationAuditLogs` struct shape from Phase 2a (`OrganizationAuditLogs<'a> { openai: &'a OpenAI }`, unchanged) — structurally independent of Task 1's Admin API Keys.
- Produces: `interfaces::organization::audit_logs::{ListAuditLogsRequest, AuditLogActorApiKeyServiceAccount, AuditLogActorUser, AuditLogActorApiKey, AuditLogActorSession, AuditLogActor, AuditLogProject, AuditLogEvent, ListAuditLogsResponse}`, `OrganizationAuditLogs::list` — the final cross-check depends on this.

- [ ] **Step 1: Write the failing test — `list` deserializes an event with `#[serde(flatten)]`-captured detail**

Add a test module to `rs_openai/src/apis/organization/audit_logs.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::audit_logs::ListAuditLogsRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_deserializes_event_with_flattened_api_key_created_detail() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/audit_logs"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "audit_log_1",
                    "effective_at": 1700000000_i64,
                    "type": "api_key.created",
                    "actor": {
                        "type": "session",
                        "api_key": null,
                        "session": {
                            "ip_address": "203.0.113.1",
                            "user": {"id": "user_1", "email": "admin@example.com"}
                        }
                    },
                    "project": {"id": "proj_1", "name": "Default Project"},
                    "api_key.created": {
                        "id": "key_1",
                        "data": {"scopes": ["api.model.request"]}
                    }
                }],
                "first_id": "audit_log_1",
                "last_id": "audit_log_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAuditLogsRequest::builder().limit(5).build();
        let res = client.organization().audit_logs().list(&req).await.unwrap();

        assert_eq!(res.data.len(), 1);
        let event = &res.data[0];
        assert_eq!(event.id, "audit_log_1");
        assert_eq!(event.kind, "api_key.created");
        assert_eq!(
            event.actor.as_ref().unwrap().session.as_ref().unwrap().ip_address.as_deref(),
            Some("203.0.113.1")
        );
        assert_eq!(event.project.as_ref().unwrap().id.as_deref(), Some("proj_1"));

        let detail = event.detail.get("api_key.created").expect("detail present");
        assert_eq!(detail["id"], "key_1");
        assert_eq!(detail["data"]["scopes"][0], "api.model.request");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_deserializes_event_with_flattened_api_key_created_detail 2>&1 | tail -40`
Expected: FAIL to compile — `crate::interfaces::organization::audit_logs` doesn't exist yet, `client.organization().audit_logs().list(...)` doesn't exist.

- [ ] **Step 3: Create `rs_openai/src/interfaces/organization/audit_logs.rs`**

```rust
//! Organization audit logs -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

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

- [ ] **Step 4: Register `audit_logs` in `interfaces::organization`**

In `rs_openai/src/interfaces/organization/mod.rs`, add this new line after the existing `pub mod admin_api_keys;`:

```rust
pub mod audit_logs;
```

- [ ] **Step 5: Implement `OrganizationAuditLogs::list`, remove stale `#[allow(dead_code)]`**

Replace the full contents of `rs_openai/src/apis/organization/audit_logs.rs` (everything above the `#[cfg(test)]` block from Step 1 — leave that test module in place) with:

```rust
//! Organization audit logs (`/organization/audit_logs`, list-only) -- see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::audit_logs;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationAuditLogs<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationAuditLogs<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    /// Lists organization audit log events.
    pub async fn list(
        &self,
        req: &audit_logs::ListAuditLogsRequest,
    ) -> OpenAIResponse<audit_logs::ListAuditLogsResponse> {
        self.openai.get("/organization/audit_logs", req).await
    }
}
```

Also update `rs_openai/src/apis/organization/mod.rs`'s doc comment for `audit_logs()`:

```rust
    /// Scoped accessor for organization audit logs.
    pub fn audit_logs(&self) -> audit_logs::OrganizationAuditLogs<'a> {
        audit_logs::OrganizationAuditLogs::new(self.openai)
    }
```

- [ ] **Step 6: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_deserializes_event_with_flattened_api_key_created_detail 2>&1 | tail -30`
Expected: PASS.

- [ ] **Step 7: Write the failing test — a second event with a DIFFERENT event type's detail key, proving the flatten approach generalizes**

Add to the same test module:

```rust
    #[tokio::test]
    async fn list_deserializes_event_with_different_flattened_detail_key() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/audit_logs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "audit_log_2",
                    "effective_at": 1700000100_i64,
                    "type": "invite.sent",
                    "actor": null,
                    "project": null,
                    "invite.sent": {
                        "id": "invite_1",
                        "data": {"email": "newmember@example.com", "role": "member"}
                    }
                }],
                "first_id": "audit_log_2",
                "last_id": "audit_log_2",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("sk-admin-test").with_base_url(mock_server.uri());
        let req = ListAuditLogsRequest::builder().build();
        let res = client.organization().audit_logs().list(&req).await.unwrap();

        let event = &res.data[0];
        assert_eq!(event.kind, "invite.sent");
        assert!(event.actor.is_none());
        let detail = event.detail.get("invite.sent").expect("detail present");
        assert_eq!(detail["data"]["email"], "newmember@example.com");
        // Confirm the OTHER event's detail key is genuinely absent from this one, not
        // just unchecked -- proves the flatten map only contains what was actually sent.
        assert!(event.detail.get("api_key.created").is_none());
    }
```

- [ ] **Step 8: Run test to verify it passes**

Run: `NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai list_deserializes_event_with_different_flattened_detail_key 2>&1 | tail -20`
Expected: PASS.

- [ ] **Step 9: Format, lint, full test run (final cross-check for this sub-slice)**

Run:
```bash
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo build -p rs_openai
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai 2>&1 | tail -10
```
Expected: clippy clean, build clean, all tests pass (197 + 2 new = 199).

Also manually verify (read the file, don't just trust the count):
- `rs_openai/src/apis/organization/admin_api_keys.rs`: `OrganizationAdminApiKeys` struct has NO `#[allow(dead_code)]` (has real methods now). `OrganizationAdminApiKey` also has NO `#[allow(dead_code)]`.
- `rs_openai/src/apis/organization/audit_logs.rs`: `OrganizationAuditLogs` struct has NO `#[allow(dead_code)]`.
- No individual event-type detail struct (e.g. `ApiKeyCreatedDetail`) exists anywhere in `interfaces/organization/audit_logs.rs` — only the `#[serde(flatten)]` `HashMap` (scope check).
- `AuditLogEvent.kind` is plain `String`, not an enum (scope check).
- `ListAuditLogsRequest` has exactly `after`/`before`/`limit` — no `actor_emails`/`actor_ids`/`event_types`/`project_ids`/`resource_ids`/`effective_at`/`tenant_only` (scope check).
- Every OTHER `apis::organization::*` accessor (`usage`, `groups`, `invites`, `projects`, `roles`, `users`) in `rs_openai/src/apis/organization/mod.rs` still says "Not yet implemented" in its doc comment — this task must not have touched those (scope check, since this is sub-slice 1 of 4).

If any of these checks fail, fix it directly and re-run the fmt/clippy/build/test sequence.

- [ ] **Step 10: No commit needed for this task if the manual verification found nothing to fix** (if it did, commit the fix with an explicit-path `git add` and a message describing what was found).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs rs_openai/src/interfaces/organization/audit_logs.rs rs_openai/src/apis/organization/mod.rs rs_openai/src/apis/organization/audit_logs.rs
git commit -m "feat(organization): add audit logs list, completing sub-slice 1 of 4"
```
