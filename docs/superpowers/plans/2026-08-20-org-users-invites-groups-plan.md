# Organization Users + Invites + Groups Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement sub-slice 2 of 4 for the Organization/admin API: Users (retrieve/update/list/delete), Invites (full CRUD), Groups (full CRUD), and Groups.Users (add/list/retrieve/remove members).

**Architecture:** Four independent tasks, one per resource area, each adding an `interfaces/organization/<name>.rs` types file and filling in the matching pre-scaffolded `apis/organization/<name>.rs` accessor file with real HTTP methods + `wiremock` tests. No `client.rs` changes — every method reuses the existing generic Bearer-auth `get`/`post`/`delete`.

**Tech Stack:** Rust, `bon::Builder`, `serde`, `wiremock`, `tokio::test` — same stack as every prior slice.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md`. This is sub-slice 2 of 4 for Organization/admin; sub-slice 1 (Admin API Keys + Audit Logs) is already shipped and merged; sub-slices 3-4 (Usage; Projects + sub-tree) are out of scope here. Roles-related sub-resources (top-level Roles, Users.Roles, Groups.Roles) are explicitly deferred — do not implement them.
- No `client.rs` changes anywhere in this plan.
- `apis::organization` does NOT do a flat `pub use crate::interfaces::organization::<name>::*;` re-export. Each `apis/organization/<name>.rs` file imports its sibling interfaces submodule directly (`use crate::interfaces::organization::<name>;`) and references types via qualified paths (e.g. `users::OrganizationUser`), matching the exact pattern already used in `apis/organization/admin_api_keys.rs` and `apis/organization/audit_logs.rs`.
- `Group` (create/retrieve/list response) and `UpdateGroupResponse` (update response) are two genuinely different types — do not collapse them into one struct.
- `UpdateGroupRequest.name` is required (not `Option<String>`) — the API requires it on every update call.
- `RemoveGroupUserResponse` has only `deleted`/`object` fields — no `id`.
- `GroupUser` (Groups.Users response) is its own smaller type — do not reuse `OrganizationUser` for it.
- `ListOrganizationUsersRequest` has only `after`/`limit` — the real API also has an `emails: Vec<String>` filter, but `serde_urlencoded` (this crate's query backend) cannot serialize a `Vec<String>` field at all (empirically confirmed: `Err(Custom("unsupported value"))` in a standalone scratch check). Do not add `emails` to this type.
- Groups.Users types and methods live in `groups.rs` (both interfaces and apis sides), not a separate file — matching the existing scaffold's own colocation of `OrganizationGroupUsers` inside `apis/organization/groups.rs`.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.
- Baseline: 199/199 tests passing before Task 1 starts.

---

### Task 1: Users (retrieve/update/list/delete)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod users;`, alphabetically — the file currently has `pub mod admin_api_keys;` and `pub mod audit_logs;`; insert `pub mod users;` after `audit_logs`)
- Create: `rs_openai/src/interfaces/organization/users.rs`
- Modify: `rs_openai/src/apis/organization/users.rs` (add methods, remove stale `#[allow(dead_code)]` from `OrganizationUsers`, update doc comments — leave `OrganizationUserRoles` and its `#[allow(dead_code)]` untouched, it's out of scope)
- Test: `rs_openai/src/apis/organization/users.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::{OrganizationUsers, OrganizationUser}` struct shapes from Phase 2a (`OrganizationUsers<'a> { openai: &'a OpenAI }`, `OrganizationUser<'a> { openai: &'a OpenAI, id: String }`, unchanged — `OrganizationUser::roles()` stays as-is, do not touch).
- Produces: `interfaces::organization::users::{OrganizationUserNested, OrganizationUserProjectsData, OrganizationUserProjects, OrganizationUser, UpdateOrganizationUserRequest, ListOrganizationUsersRequest, ListOrganizationUsersResponse, DeleteOrganizationUserResponse}`, `OrganizationUsers::list`, `OrganizationUser::retrieve`/`::update`/`::delete`. Independent of Tasks 2-4.

- [ ] **Step 1: Write the failing test — `retrieve` deserializes a user with nested `user`/`projects` sub-objects**

Add a test module to `rs_openai/src/apis/organization/users.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::users::{ListOrganizationUsersRequest, UpdateOrganizationUserRequest};
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn retrieve_deserializes_nested_user_and_projects() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "added_at": 1700000000_i64,
                "object": "organization.user",
                "email": "dev@example.com",
                "name": "Dev User",
                "role": "owner",
                "is_service_account": false,
                "projects": {
                    "object": "list",
                    "data": [{"id": "proj_1", "name": "Default", "role": "owner"}]
                },
                "user": {
                    "id": "user_abc",
                    "object": "user",
                    "email": "dev@example.com",
                    "banned": false,
                    "enabled": true
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .user("user_abc")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.id, "user_abc");
        assert_eq!(result.role.as_deref(), Some("owner"));
        let projects = result.projects.unwrap();
        assert_eq!(projects.data.len(), 1);
        assert_eq!(projects.data[0].id.as_deref(), Some("proj_1"));
        let nested = result.user.unwrap();
        assert_eq!(nested.id, "user_abc");
        assert_eq!(nested.banned, Some(false));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::users::tests::retrieve_deserializes_nested_user_and_projects`
Expected: FAIL to compile — `interfaces::organization::users` module and `retrieve()` method don't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/users.rs`:

```rust
//! Organization users (`/organization/users`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserNested {
    pub id: String,
    pub object: String,
    pub banned: Option<bool>,
    pub banned_at: Option<i64>,
    pub email: Option<String>,
    pub enabled: Option<bool>,
    pub name: Option<String>,
    pub picture: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserProjectsData {
    pub id: Option<String>,
    pub name: Option<String>,
    pub role: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUserProjects {
    pub object: String,
    pub data: Vec<OrganizationUserProjectsData>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct OrganizationUser {
    pub id: String,
    pub added_at: i64,
    pub object: String,
    pub api_key_last_used_at: Option<i64>,
    pub created: Option<i64>,
    pub developer_persona: Option<String>,
    pub email: Option<String>,
    pub is_default: Option<bool>,
    pub is_scale_tier_authorized_purchaser: Option<bool>,
    pub is_scim_managed: Option<bool>,
    pub is_service_account: Option<bool>,
    pub name: Option<String>,
    pub projects: Option<OrganizationUserProjects>,
    pub role: Option<String>,
    pub technical_level: Option<String>,
    pub user: Option<OrganizationUserNested>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateOrganizationUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub developer_persona: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub technical_level: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListOrganizationUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListOrganizationUsersResponse {
    pub object: String,
    pub data: Vec<OrganizationUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteOrganizationUserResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/mod.rs`, change:

```rust
pub mod admin_api_keys;
pub mod audit_logs;
```

to:

```rust
pub mod admin_api_keys;
pub mod audit_logs;
pub mod users;
```

- [ ] **Step 5: Implement the methods**

Replace the full contents of `rs_openai/src/apis/organization/users.rs` with:

```rust
//! Organization users (`/organization/users`) and their assigned roles -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.
//! Role assignment (`OrganizationUserRoles`) is not yet implemented -- deferred
//! to a future slice.

use crate::client::OpenAI;
use crate::interfaces::organization::users;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationUsers<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn list(
        &self,
        req: &users::ListOrganizationUsersRequest,
    ) -> OpenAIResponse<users::ListOrganizationUsersResponse> {
        self.openai.get("/organization/users", req).await
    }
}

pub struct OrganizationUser<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<users::OrganizationUser> {
        self.openai
            .get(&format!("/organization/users/{}", self.id), &())
            .await
    }

    pub async fn update(
        &self,
        req: &users::UpdateOrganizationUserRequest,
    ) -> OpenAIResponse<users::OrganizationUser> {
        self.openai
            .post(&format!("/organization/users/{}", self.id), req)
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<users::DeleteOrganizationUserResponse> {
        self.openai
            .delete(&format!("/organization/users/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this user's assigned roles. Not yet implemented.
    pub fn roles(&self) -> OrganizationUserRoles<'a> {
        OrganizationUserRoles::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationUserRoles<'a> {
    openai: &'a OpenAI,
    user_id: String,
}

impl<'a> OrganizationUserRoles<'a> {
    pub(crate) fn new(openai: &'a OpenAI, user_id: String) -> Self {
        Self { openai, user_id }
    }
}
```

Check `rs_openai/src/client.rs` for the exact signatures of `get`/`post`/`delete` before writing calls (they take a route `&str` and a serializable reference, matching every other `apis::organization::*` file already in this crate — do not guess, read the actual signatures).

- [ ] **Step 6: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::users::tests::retrieve_deserializes_nested_user_and_projects`
Expected: PASS

- [ ] **Step 7: Write the failing tests — `update` serializes only provided fields, `list` serializes query params and deserializes a paginated response**

Add to the same test module:

```rust
    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/users/user_abc"))
            .and(body_json(serde_json::json!({"role": "reader"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "added_at": 1700000000_i64,
                "object": "organization.user",
                "role": "reader"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateOrganizationUserRequest::builder().role("reader").build();
        let result = client
            .organization()
            .user("user_abc")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role.as_deref(), Some("reader"));
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/users"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListOrganizationUsersRequest::builder().limit(10).build();
        let result = client.organization().users().list(&req).await.unwrap();

        assert!(!result.has_more);
        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "deleted": true,
                "object": "organization.user.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().user("user_abc").delete().await.unwrap();

        assert!(result.deleted);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::users::tests`
Expected: 4 tests PASS (retrieve, update, list, delete)

- [ ] **Step 9: Update doc comments**

In `rs_openai/src/apis/organization/mod.rs`, change:

```rust
    /// Scoped accessor for organization users. Not yet implemented.
    pub fn users(&self) -> users::OrganizationUsers<'a> {
        users::OrganizationUsers::new(self.openai)
    }

    /// Scoped accessor for a specific organization user. Not yet implemented.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationUser<'a> {
        users::OrganizationUser::new(self.openai, id.into())
    }
```

to:

```rust
    /// Scoped accessor for organization users.
    pub fn users(&self) -> users::OrganizationUsers<'a> {
        users::OrganizationUsers::new(self.openai)
    }

    /// Scoped accessor for a specific organization user.
    pub fn user(&self, id: impl Into<String>) -> users::OrganizationUser<'a> {
        users::OrganizationUser::new(self.openai, id.into())
    }
```

- [ ] **Step 10: Run fmt, clippy, and the full test suite**

Run:
```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: fmt clean, clippy clean, 203/203 tests passing (199 baseline + 4 new).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs
git add rs_openai/src/interfaces/organization/users.rs
git add rs_openai/src/apis/organization/users.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add users retrieve/update/list/delete"
```

---

### Task 2: Invites (create/list/retrieve/delete)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod invites;`)
- Create: `rs_openai/src/interfaces/organization/invites.rs`
- Modify: `rs_openai/src/apis/organization/invites.rs` (add methods, remove stale `#[allow(dead_code)]` from both structs, update doc comments)
- Test: `rs_openai/src/apis/organization/invites.rs` (add a `#[cfg(test)] mod tests` block)

**Interfaces:**
- Consumes: existing `apis::organization::{OrganizationInvites, OrganizationInvite}` struct shapes from Phase 2a, unchanged. Structurally independent of Task 1.
- Produces: `interfaces::organization::invites::{InviteProject, CreateInviteRequest, Invite, ListInvitesRequest, ListInvitesResponse, DeleteInviteResponse}`, `OrganizationInvites::create`/`::list`, `OrganizationInvite::retrieve`/`::delete`.

- [ ] **Step 1: Write the failing test — `create` serializes `projects` and deserializes a full `Invite`**

Add a test module to `rs_openai/src/apis/organization/invites.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::invites::{CreateInviteRequest, InviteProject, ListInvitesRequest};
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_projects_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/invites"))
            .and(body_json(serde_json::json!({
                "email": "new@example.com",
                "role": "reader",
                "projects": [{"id": "proj_1", "role": "member"}]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [{"id": "proj_1", "role": "member"}],
                "role": "reader",
                "status": "pending"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateInviteRequest::builder()
            .email("new@example.com")
            .role("reader")
            .projects(vec![InviteProject::builder().id("proj_1").role("member").build()])
            .build();
        let result = client.organization().invites().create(&req).await.unwrap();

        assert_eq!(result.id, "invite_1");
        assert_eq!(result.status, "pending");
        assert_eq!(result.projects.len(), 1);
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_projects() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/invites"))
            .and(body_json(serde_json::json!({
                "email": "new@example.com",
                "role": "owner"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_2",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [],
                "role": "owner",
                "status": "pending"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateInviteRequest::builder().email("new@example.com").role("owner").build();
        let result = client.organization().invites().create(&req).await.unwrap();

        assert!(result.projects.is_empty());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::invites::tests`
Expected: FAIL to compile — `interfaces::organization::invites` module doesn't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/invites.rs`:

```rust
//! Organization invites (`/organization/invites`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct InviteProject {
    #[builder(into)]
    pub id: String,
    #[builder(into)]
    pub role: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateInviteRequest {
    #[builder(into)]
    pub email: String,

    #[builder(into)]
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub projects: Option<Vec<InviteProject>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Invite {
    pub id: String,
    pub created_at: i64,
    pub email: String,
    pub object: String,
    pub projects: Vec<InviteProject>,
    pub role: String,
    pub status: String,
    pub accepted_at: Option<i64>,
    pub expires_at: Option<i64>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListInvitesRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListInvitesResponse {
    pub object: String,
    pub data: Vec<Invite>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteInviteResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/mod.rs`, add `pub mod invites;` alphabetically (after `audit_logs`, before `users` if Task 1 already ran — check current file content first and insert in alphabetical order among whatever `pub mod` lines already exist).

- [ ] **Step 5: Implement the methods**

Replace the full contents of `rs_openai/src/apis/organization/invites.rs` with:

```rust
//! Organization invites (`/organization/invites`) -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::invites;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationInvites<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationInvites<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(
        &self,
        req: &invites::CreateInviteRequest,
    ) -> OpenAIResponse<invites::Invite> {
        self.openai.post("/organization/invites", req).await
    }

    pub async fn list(
        &self,
        req: &invites::ListInvitesRequest,
    ) -> OpenAIResponse<invites::ListInvitesResponse> {
        self.openai.get("/organization/invites", req).await
    }
}

pub struct OrganizationInvite<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationInvite<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<invites::Invite> {
        self.openai
            .get(&format!("/organization/invites/{}", self.id), &())
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<invites::DeleteInviteResponse> {
        self.openai
            .delete(&format!("/organization/invites/{}", self.id), &())
            .await
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::invites::tests`
Expected: 2 tests PASS

- [ ] **Step 7: Write the failing tests — `list`, `retrieve`, `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/invites"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListInvitesRequest::builder().limit(5).build();
        let result = client.organization().invites().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn retrieve_deserializes_invite() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/invites/invite_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "created_at": 1700000000_i64,
                "email": "new@example.com",
                "object": "organization.invite",
                "projects": [],
                "role": "reader",
                "status": "accepted",
                "accepted_at": 1700000100_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().invite("invite_1").retrieve().await.unwrap();

        assert_eq!(result.status, "accepted");
        assert_eq!(result.accepted_at, Some(1700000100));
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/invites/invite_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "invite_1",
                "deleted": true,
                "object": "organization.invite.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().invite("invite_1").delete().await.unwrap();

        assert!(result.deleted);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::invites::tests`
Expected: 5 tests PASS

- [ ] **Step 9: Update doc comments**

In `rs_openai/src/apis/organization/mod.rs`, remove "Not yet implemented." from the `invites()`/`invite()` doc comments (same pattern as Task 1 Step 9).

- [ ] **Step 10: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, 208/208 (203 from Task 1 + 5 new — adjust the expected baseline if Task 1 isn't merged yet by checking the actual current count first).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs
git add rs_openai/src/interfaces/organization/invites.rs
git add rs_openai/src/apis/organization/invites.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add invites create/list/retrieve/delete"
```

---

### Task 3: Groups (create/retrieve/update/list/delete)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod groups;`)
- Create: `rs_openai/src/interfaces/organization/groups.rs` (Groups types only this task — Groups.Users types added in Task 4, same file)
- Modify: `rs_openai/src/apis/organization/groups.rs` (add Groups CRUD methods only; leave `OrganizationGroupUsers` and its `#[allow(dead_code)]` untouched, Task 4's job)
- Test: `rs_openai/src/apis/organization/groups.rs` (add a `#[cfg(test)] mod tests` block)

**Interfaces:**
- Consumes: existing `apis::organization::{OrganizationGroups, OrganizationGroup}` struct shapes from Phase 2a, unchanged. Structurally independent of Tasks 1-2.
- Produces: `interfaces::organization::groups::{CreateGroupRequest, UpdateGroupRequest, Group, UpdateGroupResponse, ListGroupsRequest, ListGroupsResponse, DeleteGroupResponse}`, `OrganizationGroups::create`/`::list`, `OrganizationGroup::retrieve`/`::update`/`::delete`. Task 4 (Groups.Users) depends on this task's `interfaces/organization/groups.rs` file existing (adds to the same file) but not on any specific type in it.

- [ ] **Step 1: Write the failing test — `create` and `update` deserialize into their genuinely different response shapes**

Add a test module to `rs_openai/src/apis/organization/groups.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::groups::{CreateGroupRequest, ListGroupsRequest, UpdateGroupRequest};
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_name_and_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/groups"))
            .and(body_json(serde_json::json!({"name": "Engineering"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "group_type": "group",
                "is_scim_managed": false,
                "name": "Engineering"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateGroupRequest::builder().name("Engineering").build();
        let result = client.organization().groups().create(&req).await.unwrap();

        assert_eq!(result.id, "group_1");
        assert_eq!(result.group_type, "group");
    }

    #[tokio::test]
    async fn update_deserializes_into_distinct_response_shape() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/groups/group_1"))
            .and(body_json(serde_json::json!({"name": "Eng Renamed"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "is_scim_managed": false,
                "name": "Eng Renamed"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateGroupRequest::builder().name("Eng Renamed").build();
        let result = client.organization().group("group_1").update(&req).await.unwrap();

        assert_eq!(result.name, "Eng Renamed");
        // UpdateGroupResponse has no `group_type` field -- this line only compiles
        // because the type is genuinely distinct from `Group`, proving the asymmetry.
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests`
Expected: FAIL to compile.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/groups.rs`:

```rust
//! Organization groups (`/organization/groups`) and their member users -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.

use crate::interfaces::chat::SortOrder;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateGroupRequest {
    #[builder(into)]
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct UpdateGroupRequest {
    #[builder(into)]
    pub name: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Group {
    pub id: String,
    pub created_at: i64,
    pub group_type: String,
    pub is_scim_managed: bool,
    pub name: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct UpdateGroupResponse {
    pub id: String,
    pub created_at: i64,
    pub is_scim_managed: bool,
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListGroupsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListGroupsResponse {
    pub object: String,
    pub data: Vec<Group>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteGroupResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/mod.rs`, add `pub mod groups;` alphabetically among whatever `pub mod` lines already exist.

- [ ] **Step 5: Implement the methods**

In `rs_openai/src/apis/organization/groups.rs`, add imports and fill in `OrganizationGroups`/`OrganizationGroup`'s bodies (leave `OrganizationGroupUsers` at the bottom of the file untouched — Task 4's job):

```rust
//! Organization groups (`/organization/groups`) and their member users -- see
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md.
//! Groups.Users is implemented; Groups.Roles is not yet implemented -- deferred
//! to a future slice.

use crate::client::OpenAI;
use crate::interfaces::organization::groups;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationGroups<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(
        &self,
        req: &groups::CreateGroupRequest,
    ) -> OpenAIResponse<groups::Group> {
        self.openai.post("/organization/groups", req).await
    }

    pub async fn list(
        &self,
        req: &groups::ListGroupsRequest,
    ) -> OpenAIResponse<groups::ListGroupsResponse> {
        self.openai.get("/organization/groups", req).await
    }
}

pub struct OrganizationGroup<'a> {
    openai: &'a OpenAI,
    id: String,
}

impl<'a> OrganizationGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, id: String) -> Self {
        Self { openai, id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<groups::Group> {
        self.openai
            .get(&format!("/organization/groups/{}", self.id), &())
            .await
    }

    pub async fn update(
        &self,
        req: &groups::UpdateGroupRequest,
    ) -> OpenAIResponse<groups::UpdateGroupResponse> {
        self.openai
            .post(&format!("/organization/groups/{}", self.id), req)
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::DeleteGroupResponse> {
        self.openai
            .delete(&format!("/organization/groups/{}", self.id), &())
            .await
    }

    /// Scoped accessor for this group's member users.
    pub fn users(&self) -> OrganizationGroupUsers<'a> {
        OrganizationGroupUsers::new(self.openai, self.id.clone())
    }
}

#[allow(dead_code)]
pub struct OrganizationGroupUsers<'a> {
    openai: &'a OpenAI,
    group_id: String,
}

impl<'a> OrganizationGroupUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String) -> Self {
        Self { openai, group_id }
    }
}
```

Note the `users()` doc comment already drops "Not yet implemented" here since Task 4 (later in this same plan) will implement it before this branch merges — if Task 4 has not yet run when you reach this step, leave the doc comment as "Not yet implemented." instead and let Task 4 update it; check the current file content first rather than blindly pasting.

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests`
Expected: 2 tests PASS

- [ ] **Step 7: Write the failing tests — `retrieve`, `list`, `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "created_at": 1700000000_i64,
                "group_type": "tenant_group",
                "is_scim_managed": true,
                "name": "Tenant Group"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().group("group_1").retrieve().await.unwrap();

        assert_eq!(result.group_type, "tenant_group");
        assert!(result.is_scim_managed);
    }

    #[tokio::test]
    async fn list_serializes_order_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups"))
            .and(query_param("order", "desc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [],
                "first_id": null,
                "last_id": null,
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListGroupsRequest::builder()
            .order(crate::interfaces::chat::SortOrder::Desc)
            .build();
        let result = client.organization().groups().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "group_1",
                "deleted": true,
                "object": "group.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().group("group_1").delete().await.unwrap();

        assert!(result.deleted);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests`
Expected: 5 tests PASS

- [ ] **Step 9: Update doc comments**

In `rs_openai/src/apis/organization/mod.rs`, remove "Not yet implemented." from the `groups()`/`group()` doc comments.

- [ ] **Step 10: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, +5 over whatever the current baseline is.

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs
git add rs_openai/src/interfaces/organization/groups.rs
git add rs_openai/src/apis/organization/groups.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add groups create/retrieve/update/list/delete"
```

---

### Task 4: Groups.Users (add/list/retrieve/remove members) + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/organization/groups.rs` (append Groups.Users types to the end of the file — do not touch the Groups types Task 3 added)
- Modify: `rs_openai/src/apis/organization/groups.rs` (fill in `OrganizationGroupUsers`'s body, remove its stale `#[allow(dead_code)]`, update its doc comment if Task 3 left it as "Not yet implemented")
- Test: `rs_openai/src/apis/organization/groups.rs` (extend the existing `#[cfg(test)] mod tests` block from Task 3)

**Interfaces:**
- Consumes: `OrganizationGroupUsers<'a> { openai: &'a OpenAI, group_id: String }` from Task 3 (already scaffolded, do not change its fields), `interfaces::organization::groups` module from Task 3 (append to it, same file).
- Produces: `interfaces::organization::groups::{AddGroupUserRequest, GroupUser, ListGroupUsersRequest, ListGroupUsersResponse, RemoveGroupUserResponse}`, `OrganizationGroupUsers::create`/`::list`, and a new ID-scoped `OrganizationGroupUser<'a>` struct (not previously scaffolded — Groups.Users needs per-user `retrieve`/`delete`, matching the established precedent of adding an ID-scoped accessor beyond what Phase 2a's scaffold anticipated, same as `VectorStoreFile<'a>`/`ContainerFile<'a>` in earlier slices) with `::retrieve`/`::delete`.

- [ ] **Step 1: Write the failing test — `create` adds a member and deserializes a `GroupUser`**

Append to the `tests` module in `rs_openai/src/apis/organization/groups.rs` (add these imports to the existing `use` list at the top of the test module: `crate::interfaces::organization::groups::{AddGroupUserRequest, ListGroupUsersRequest}`):

```rust
    #[tokio::test]
    async fn group_users_create_adds_member_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/groups/group_1/users"))
            .and(body_json(serde_json::json!({"user_id": "user_abc"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "email": "dev@example.com",
                "name": "Dev User"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AddGroupUserRequest::builder().user_id("user_abc").build();
        let result = client
            .organization()
            .group("group_1")
            .users()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.id, "user_abc");
        assert_eq!(result.name, "Dev User");
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests::group_users_create_adds_member_and_deserializes_response`
Expected: FAIL to compile — `AddGroupUserRequest`/`GroupUser` and `OrganizationGroupUsers::create` don't exist yet.

- [ ] **Step 3: Append Groups.Users types**

Append to the end of `rs_openai/src/interfaces/organization/groups.rs` (after `DeleteGroupResponse`):

```rust

// Groups.Users sub-resource.

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddGroupUserRequest {
    #[builder(into)]
    pub user_id: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct GroupUser {
    pub id: String,
    pub email: Option<String>,
    pub name: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListGroupUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListGroupUsersResponse {
    pub object: String,
    pub data: Vec<GroupUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveGroupUserResponse {
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Implement the methods**

In `rs_openai/src/apis/organization/groups.rs`, replace the `OrganizationGroupUsers` struct's `#[allow(dead_code)]`-marked block (left over from Task 3/Phase 2a) with:

```rust
pub struct OrganizationGroupUsers<'a> {
    openai: &'a OpenAI,
    group_id: String,
}

impl<'a> OrganizationGroupUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String) -> Self {
        Self { openai, group_id }
    }

    pub async fn create(
        &self,
        req: &groups::AddGroupUserRequest,
    ) -> OpenAIResponse<groups::GroupUser> {
        self.openai
            .post(&format!("/organization/groups/{}/users", self.group_id), req)
            .await
    }

    pub async fn list(
        &self,
        req: &groups::ListGroupUsersRequest,
    ) -> OpenAIResponse<groups::ListGroupUsersResponse> {
        self.openai
            .get(&format!("/organization/groups/{}/users", self.group_id), req)
            .await
    }

    pub fn user(&self, user_id: impl Into<String>) -> OrganizationGroupUser<'a> {
        OrganizationGroupUser::new(self.openai, self.group_id.clone(), user_id.into())
    }
}

pub struct OrganizationGroupUser<'a> {
    openai: &'a OpenAI,
    group_id: String,
    user_id: String,
}

impl<'a> OrganizationGroupUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, group_id: String, user_id: String) -> Self {
        Self { openai, group_id, user_id }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<groups::GroupUser> {
        self.openai
            .get(
                &format!("/organization/groups/{}/users/{}", self.group_id, self.user_id),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::RemoveGroupUserResponse> {
        self.openai
            .delete(
                &format!("/organization/groups/{}/users/{}", self.group_id, self.user_id),
                &(),
            )
            .await
    }
}
```

Remove the `#[allow(dead_code)]` attribute that was on the old `OrganizationGroupUsers` struct — it now has real methods reading its fields.

- [ ] **Step 5: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests::group_users_create_adds_member_and_deserializes_response`
Expected: PASS

- [ ] **Step 6: Write the failing tests — `list`, `retrieve`, `delete` (proving `RemoveGroupUserResponse` has no `id`)**

Append to the same test module:

```rust
    #[tokio::test]
    async fn group_users_list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups/group_1/users"))
            .and(query_param("limit", "20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{"id": "user_abc", "email": "dev@example.com", "name": "Dev User"}],
                "first_id": "user_abc",
                "last_id": "user_abc",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListGroupUsersRequest::builder().limit(20).build();
        let result = client
            .organization()
            .group("group_1")
            .users()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
    }

    #[tokio::test]
    async fn group_user_retrieve_deserializes_membership() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/groups/group_1/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_abc",
                "email": "dev@example.com",
                "name": "Dev User"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .group("group_1")
            .users()
            .user("user_abc")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.name, "Dev User");
    }

    #[tokio::test]
    async fn group_user_delete_deserializes_confirmation_without_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/groups/group_1/users/user_abc"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "group.user.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .group("group_1")
            .users()
            .user("user_abc")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
        // RemoveGroupUserResponse has no `id` field -- this test compiling and
        // passing is itself the proof that the asymmetry is modeled correctly.
    }
```

- [ ] **Step 7: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::groups::tests`
Expected: all Groups + Groups.Users tests PASS (8 total in this file: 5 from Task 3 + 3 new... actually 4 new counting Step 1's test — 9 total).

- [ ] **Step 8: Update doc comments**

In `rs_openai/src/apis/organization/groups.rs`, ensure `OrganizationGroup::users()`'s doc comment reads "Scoped accessor for this group's member users." (no "Not yet implemented" — fix if Task 3 left it stale).

- [ ] **Step 9: Final cross-check — re-read the whole branch's diff for doc-comment staleness**

Run `git diff master --stat` (or `git log --oneline master..HEAD` then `git diff <merge-base>..HEAD`) and re-read `rs_openai/src/apis/organization/mod.rs` in full: confirm `users()`/`user()`/`invites()`/`invite()`/`groups()`/`group()` doc comments all correctly say "implemented" (no stale "Not yet implemented"), and `usage()`/`projects()`/`project()`/`roles()`/`role()` correctly still say "Not yet implemented" (out of scope for this slice, must not have been touched). Confirm `apis::organization::users::OrganizationUserRoles` and the top-level `Organization::roles()`/`role()` accessors are untouched and still `#[allow(dead_code)]` where applicable.

- [ ] **Step 10: Run fmt, clippy, and the full test suite one final time**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, all tests passing (baseline 199 + Task 1's 4 + Task 2's 5 + Task 3's 5 + Task 4's 4 = 217).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/groups.rs
git add rs_openai/src/apis/organization/groups.rs
git commit -m "feat(organization): add groups.users add/list/retrieve/remove, completing sub-slice 2 of 4"
```
