# Organization Projects Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement sub-slice 4 of 4 for the Organization/admin API — the final Organization/admin sub-slice, and the final piece of all 9 original Phase 2a scaffolds: Projects CRUD, Projects.ApiKeys, Projects.RateLimits, Projects.Groups, Projects.ServiceAccounts (+ nested ServiceAccounts.ApiKeys), Projects.Users.

**Architecture:** Five tasks. Task 1 builds Projects CRUD itself (`interfaces/organization/projects/mod.rs` + the top-level accessor methods). Tasks 2-5 each add one (or two, for Task 2) sub-resource file under `interfaces/organization/projects/` and fill in the matching pre-scaffolded `apis/organization/projects/<name>.rs` file. No `client.rs` changes anywhere — every method reuses the existing generic Bearer-auth `get`/`post`/`delete`; this slice has no query-array-serialization need (unlike sub-slice 3's Usage), since its only array-shaped field (`scopes` on a service-account API key) is a JSON body field, which ordinary `serde` handles natively.

**Tech Stack:** Rust, `bon::Builder`, `serde`, `wiremock`, `tokio::test` — same stack as every prior slice.

## Global Constraints

- Spec: `docs/superpowers/specs/2026-08-20-org-projects-design.md`. This is sub-slice 4 of 4 for Organization/admin (the final one); sub-slices 1-3 are already shipped and merged. The 6 newer, unscaffolded project-level resources (`certificates`, `data_retention`, `hosted_tool_permissions`, `model_permissions`, `spend_alerts`, `spend_limit`) are explicitly out of scope — do not implement them. Roles-related sub-resources (project-level Groups.Roles/Users.Roles, and everything already deferred at the org level) are also out of scope.
- No `client.rs` changes anywhere in this plan.
- `apis::organization::projects` does NOT do a flat `pub use crate::interfaces::organization::projects::<name>::*;` re-export anywhere. Each `apis/organization/projects/<name>.rs` file imports its sibling interfaces submodule directly (`use crate::interfaces::organization::projects::<name>;`, or `use crate::interfaces::organization::projects;` for the top-level `mod.rs` file) and references types via qualified paths (e.g. `api_keys::ProjectApiKey`, `projects::Project`), matching the exact pattern already used throughout `apis/organization/`.
- `interfaces/organization/projects/mod.rs` is built incrementally across tasks: Task 1 creates it with the top-level `Project`-related types only (no `pub mod` sub-declarations yet); each of Tasks 2-5 adds its own `pub mod <name>;` line to this same file as it creates that sub-resource's interfaces file. Do not add a `pub mod` line for a sub-resource before its file exists — the crate must compile after every single task's commit, not just at the end of the plan.
- `Project` (create/retrieve/update/list response) has NO `delete` — the crate's lifecycle-ending operation for a project is `archive` (`POST /organization/projects/{id}/archive`), returning the same `Project` type. Do not add a `delete` method to `OrganizationProject`.
- `Projects.ApiKeys` has NO `create` method — do not add one. `Projects.RateLimits` has NO `create`/`delete` methods — do not add them, only `list`/`update`.
- `Projects.Groups` has NO `update` method — do not add one, only `create`/`retrieve`/`list`/`delete`.
- `RemoveProjectGroupResponse` has only `deleted`/`object` fields — no `id`.
- `CreateProjectServiceAccountResponse` (create-only) and `ProjectServiceAccount` (retrieve/update/list) are two genuinely different types — do not collapse them into one struct. `CreateProjectServiceAccountResponse` has an extra `api_key: Option<CreateProjectServiceAccountApiKey>` field and a narrower `role` value set than `ProjectServiceAccount`.
- `CreateProjectServiceAccountApiKeyRequest.scopes: Option<Vec<String>>` is an ordinary JSON POST body field — do NOT introduce any query-pairs/`to_query_pairs()` machinery for it or anywhere in this plan; that machinery (from sub-slice 3) solves a query-STRING-specific problem this slice does not have.
- `ServiceAccounts.ApiKeys` types and methods live in `service_accounts.rs` (both interfaces and apis sides), not a separate file — matching the existing scaffold's own colocation pattern (e.g. Groups.Users inside `groups.rs` at the org level in sub-slice 2).
- New ID-scoped accessor structs beyond the flat Phase 2a scaffold are expected and correct where the real API needs one level deeper than the scaffold anticipated (`OrganizationProjectRateLimit`, `OrganizationProjectGroup`, `OrganizationProjectServiceAccountApiKeys`) — matching the established `VectorStoreFile`/`ContainerFile`/`OrganizationGroupUser` precedent.
- Stage commits with **explicit file paths** (`git add path/to/file.rs`), never `git add -A`.
- Run `cargo fmt` and `cargo clippy -p rs_openai --all-targets -- -D warnings` before every commit; both must be clean.
- Environment: this machine's shell routes HTTP through a local proxy that breaks `wiremock` tests. Prefix every `cargo test`/`build`/`clippy` invocation with `NO_PROXY=127.0.0.1,localhost`. This is a local-shell-only concern — never write it into crate code, `Cargo.toml`, or CI config.
- Test client construction: `OpenAI::new("test-key").with_base_url(mock_server.uri())` — NOT `OpenAI::new_with_base_url(...)`, which does not exist (this exact bug has been caught and fixed multiple times across this project; do not repeat it).
- Baseline: 230/230 tests passing before Task 1 starts (verified directly against the live repo).

---

### Task 1: Projects CRUD (`create`/`retrieve`/`update`/`list`/`archive`)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/mod.rs` (add `pub mod projects;`, alphabetically — the file currently has `admin_api_keys`, `audit_logs`, `groups`, `invites`, `usage`, `users`; insert `projects` between `invites` and `usage`)
- Create: `rs_openai/src/interfaces/organization/projects/mod.rs` (top-level `Project`-related types only — no `pub mod` sub-declarations yet, those are added by Tasks 2-5)
- Modify: `rs_openai/src/apis/organization/projects/mod.rs` (add `create`/`list` to `OrganizationProjects`, `retrieve`/`update`/`archive` to `OrganizationProject`; remove stale `#[allow(dead_code)]` from `OrganizationProjects`; leave all 6 existing sub-resource accessor methods — `api_keys()`/`api_key()`/`groups()`/`rate_limits()`/`service_accounts()`/`service_account()`/`users()`/`user()` — untouched, they're out of scope for this task)
- Test: `rs_openai/src/apis/organization/projects/mod.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::projects::{OrganizationProjects, OrganizationProject}` struct shapes from Phase 2a (`OrganizationProjects<'a> { openai: &'a OpenAI }`, `OrganizationProject<'a> { openai: &'a OpenAI, id: String }`, unchanged — all 6 existing sub-resource accessor methods stay as-is, do not touch).
- Produces: `interfaces::organization::projects::{CreateProjectRequest, UpdateProjectRequest, Project, ListProjectsRequest, ListProjectsResponse}`, `OrganizationProjects::create`/`::list`, `OrganizationProject::retrieve`/`::update`/`::archive`. Structurally independent of Tasks 2-5 (they each add their own sub-module file to `interfaces/organization/projects/`, not consuming anything from this task).

- [ ] **Step 1: Write the failing test — `create` serializes the full request and deserializes the response**

Add a test module to `rs_openai/src/apis/organization/projects/mod.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::{
        CreateProjectRequest, ListProjectsRequest, UpdateProjectRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_full_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects"))
            .and(body_json(serde_json::json!({
                "name": "New Project",
                "external_key_id": "ext_1",
                "geography": "us"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectRequest::builder()
            .name("New Project")
            .external_key_id("ext_1")
            .geography("us")
            .build();
        let result = client.organization().projects().create(&req).await.unwrap();

        assert_eq!(result.id, "proj_1");
        assert_eq!(result.status.as_deref(), Some("active"));
    }

    #[tokio::test]
    async fn create_with_only_required_fields_omits_optional_ones() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects"))
            .and(body_json(serde_json::json!({"name": "Minimal Project"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_2",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "Minimal Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectRequest::builder().name("Minimal Project").build();
        let result = client.organization().projects().create(&req).await.unwrap();

        assert_eq!(result.id, "proj_2");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::tests`
Expected: FAIL to compile — `interfaces::organization::projects` module and `create()` method don't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/projects/mod.rs`:

```rust
//! Organization projects (`/organization/projects`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateProjectRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub external_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub geography: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub external_key_id: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub geography: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Project {
    pub id: String,
    pub created_at: i64,
    pub object: String,
    pub archived_at: Option<i64>,
    pub external_key_id: Option<String>,
    pub name: Option<String>,
    pub status: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_archived: Option<bool>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectsResponse {
    pub object: String,
    pub data: Vec<Project>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/mod.rs`, change:

```rust
pub mod admin_api_keys;
pub mod audit_logs;
pub mod groups;
pub mod invites;
pub mod usage;
pub mod users;
```

to:

```rust
pub mod admin_api_keys;
pub mod audit_logs;
pub mod groups;
pub mod invites;
pub mod projects;
pub mod usage;
pub mod users;
```

- [ ] **Step 5: Implement the methods**

In `rs_openai/src/apis/organization/projects/mod.rs`, add the import and fill in `OrganizationProjects`/`OrganizationProject`'s bodies. Replace:

```rust
use crate::client::OpenAI;

#[allow(dead_code)]
pub struct OrganizationProjects<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationProjects<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }
}
```

with:

```rust
use crate::client::OpenAI;
use crate::interfaces::organization::projects;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjects<'a> {
    openai: &'a OpenAI,
}

impl<'a> OrganizationProjects<'a> {
    pub(crate) fn new(openai: &'a OpenAI) -> Self {
        Self { openai }
    }

    pub async fn create(
        &self,
        req: &projects::CreateProjectRequest,
    ) -> OpenAIResponse<projects::Project> {
        self.openai.post("/organization/projects", req).await
    }

    pub async fn list(
        &self,
        req: &projects::ListProjectsRequest,
    ) -> OpenAIResponse<projects::ListProjectsResponse> {
        self.openai.get("/organization/projects", req).await
    }
}
```

Then, inside `impl<'a> OrganizationProject<'a>`, add (immediately after the `pub(crate) fn new` constructor, before the existing `api_keys()` accessor method):

```rust
    pub async fn retrieve(&self) -> OpenAIResponse<projects::Project> {
        self.openai
            .get(&format!("/organization/projects/{}", self.id), &())
            .await
    }

    pub async fn update(
        &self,
        req: &projects::UpdateProjectRequest,
    ) -> OpenAIResponse<projects::Project> {
        self.openai
            .post(&format!("/organization/projects/{}", self.id), req)
            .await
    }

    pub async fn archive(&self) -> OpenAIResponse<projects::Project> {
        self.openai
            .post(
                &format!("/organization/projects/{}/archive", self.id),
                &(),
            )
            .await
    }
```

Check `rs_openai/src/client.rs` for the exact signatures of `get`/`post` before writing calls (they take a route `&str` and a serializable reference, matching every other `apis::organization::*` file already in this crate — do not guess, read the actual signatures).

- [ ] **Step 6: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::tests`
Expected: 2 tests PASS

- [ ] **Step 7: Write the failing tests — `retrieve`, `update`, `list`, `archive`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_project() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().project("proj_1").retrieve().await.unwrap();

        assert_eq!(result.name.as_deref(), Some("New Project"));
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1"))
            .and(body_json(serde_json::json!({"name": "Renamed Project"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "Renamed Project",
                "status": "active"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectRequest::builder().name("Renamed Project").build();
        let result = client.organization().project("proj_1").update(&req).await.unwrap();

        assert_eq!(result.name.as_deref(), Some("Renamed Project"));
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects"))
            .and(query_param("limit", "10"))
            .and(query_param("include_archived", "true"))
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
        let req = ListProjectsRequest::builder()
            .limit(10)
            .include_archived(true)
            .build();
        let result = client.organization().projects().list(&req).await.unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn archive_deserializes_archived_project() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/archive"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "proj_1",
                "created_at": 1700000000_i64,
                "object": "organization.project",
                "name": "New Project",
                "status": "archived",
                "archived_at": 1700000200_i64
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client.organization().project("proj_1").archive().await.unwrap();

        assert_eq!(result.status.as_deref(), Some("archived"));
        assert_eq!(result.archived_at, Some(1700000200));
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::tests`
Expected: 6 tests PASS

- [ ] **Step 9: Update doc comments**

In `rs_openai/src/apis/organization/mod.rs`, change:

```rust
    /// Scoped accessor for organization projects. Not yet implemented.
    pub fn projects(&self) -> projects::OrganizationProjects<'a> {
        projects::OrganizationProjects::new(self.openai)
    }

    /// Scoped accessor for a specific organization project. Not yet implemented.
    pub fn project(&self, id: impl Into<String>) -> projects::OrganizationProject<'a> {
        projects::OrganizationProject::new(self.openai, id.into())
    }
```

to:

```rust
    /// Scoped accessor for organization projects.
    pub fn projects(&self) -> projects::OrganizationProjects<'a> {
        projects::OrganizationProjects::new(self.openai)
    }

    /// Scoped accessor for a specific organization project.
    pub fn project(&self, id: impl Into<String>) -> projects::OrganizationProject<'a> {
        projects::OrganizationProject::new(self.openai, id.into())
    }
```

Leave the module-level `//!` header comment at the top of the file untouched for now — it still correctly says `projects` is not yet implemented, since `projects()`'s own CRUD is only half the picture (its sub-resources aren't done until Task 5). Task 5's final cross-check updates the header once the whole slice is complete.

- [ ] **Step 10: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: fmt clean, clippy clean, 236/236 tests passing (230 baseline + 6 new).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/mod.rs
git add rs_openai/src/interfaces/organization/projects/mod.rs
git add rs_openai/src/apis/organization/projects/mod.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add projects create/retrieve/update/list/archive"
```

---

### Task 2: Projects.ApiKeys (`list`/`retrieve`/`delete`) + Projects.RateLimits (`list`/`update`)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/projects/mod.rs` (add `pub mod api_keys;` and `pub mod rate_limits;`)
- Create: `rs_openai/src/interfaces/organization/projects/api_keys.rs`
- Create: `rs_openai/src/interfaces/organization/projects/rate_limits.rs`
- Modify: `rs_openai/src/apis/organization/projects/api_keys.rs` (add methods, remove stale `#[allow(dead_code)]` from both structs)
- Modify: `rs_openai/src/apis/organization/projects/rate_limits.rs` (add methods; add a NEW `OrganizationProjectRateLimit<'a>` ID-scoped struct + a `.rate_limit(id)` accessor on `OrganizationProjectRateLimits`, since the scaffold only modeled the collection level but `update` needs an ID-scoped one; remove stale `#[allow(dead_code)]` from `OrganizationProjectRateLimits`)
- Test: both files above (add `#[cfg(test)] mod tests` blocks — none exist yet)

**Interfaces:**
- Consumes: existing `apis::organization::projects::{OrganizationProjectApiKeys, OrganizationProjectApiKey, OrganizationProjectRateLimits}` struct shapes from Phase 2a, unchanged. `OrganizationProjectRateLimit` is a NEW struct this task adds (not previously scaffolded).
- Produces: `interfaces::organization::projects::api_keys::{ProjectApiKeyOwnerServiceAccount, ProjectApiKeyOwnerUser, ProjectApiKeyOwner, ProjectApiKey, ListProjectApiKeysRequest, ListProjectApiKeysResponse, DeleteProjectApiKeyResponse}`, `interfaces::organization::projects::rate_limits::{ProjectRateLimit, ListProjectRateLimitsRequest, ListProjectRateLimitsResponse, UpdateProjectRateLimitRequest}`, `OrganizationProjectApiKeys::list`, `OrganizationProjectApiKey::retrieve`/`::delete`, `OrganizationProjectRateLimits::list_rate_limits`/`::rate_limit(id)`, `OrganizationProjectRateLimit::update`. Structurally independent of Tasks 1/3/4/5.

- [ ] **Step 1: Write the failing test — `list` (api_keys) deserializes an owner with nested user sub-object**

Add a test module to `rs_openai/src/apis/organization/projects/api_keys.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::api_keys::ListProjectApiKeysRequest;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_deserializes_owner_with_nested_user() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/api_keys"))
            .and(query_param("limit", "10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "key_1",
                    "created_at": 1700000000_i64,
                    "name": "Dev Key",
                    "object": "organization.project.api_key",
                    "owner": {
                        "type": "user",
                        "user": {
                            "id": "user_1",
                            "created_at": 1700000000_i64,
                            "email": "dev@example.com",
                            "name": "Dev User",
                            "role": "owner"
                        }
                    },
                    "owner_project_access": "active",
                    "redacted_value": "sk-...abcd"
                }],
                "first_id": "key_1",
                "last_id": "key_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectApiKeysRequest::builder().limit(10).build();
        let result = client
            .organization()
            .project("proj_1")
            .api_keys()
            .list(&req)
            .await
            .unwrap();

        assert_eq!(result.data.len(), 1);
        let owner = &result.data[0].owner;
        assert_eq!(owner.kind.as_deref(), Some("user"));
        assert_eq!(owner.user.as_ref().unwrap().email, "dev@example.com");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::api_keys::tests`
Expected: FAIL to compile — `interfaces::organization::projects::api_keys` module doesn't exist yet.

- [ ] **Step 3: Create the api_keys types file**

Create `rs_openai/src/interfaces/organization/projects/api_keys.rs`:

```rust
//! A project's API keys (`/organization/projects/{project_id}/api_keys`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwnerServiceAccount {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwnerUser {
    pub id: String,
    pub created_at: i64,
    pub email: String,
    pub name: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKeyOwner {
    pub service_account: Option<ProjectApiKeyOwnerServiceAccount>,
    #[serde(rename = "type")]
    pub kind: Option<String>,
    pub user: Option<ProjectApiKeyOwnerUser>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectApiKey {
    pub id: String,
    pub created_at: i64,
    pub last_used_at: Option<i64>,
    pub name: String,
    pub object: String,
    pub owner: ProjectApiKeyOwner,
    pub owner_project_access: String,
    pub redacted_value: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectApiKeysRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub owner_project_access: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectApiKeysResponse {
    pub object: String,
    pub data: Vec<ProjectApiKey>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectApiKeyResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Create the rate_limits types file**

Create `rs_openai/src/interfaces/organization/projects/rate_limits.rs`:

```rust
//! A project's rate limits (`/organization/projects/{project_id}/rate_limits`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectRateLimit {
    pub id: String,
    pub max_requests_per_1_minute: i64,
    pub max_tokens_per_1_minute: i64,
    pub model: String,
    pub object: String,
    pub batch_1_day_max_input_tokens: Option<i64>,
    pub max_audio_megabytes_per_1_minute: Option<i64>,
    pub max_images_per_1_minute: Option<i64>,
    pub max_requests_per_1_day: Option<i64>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectRateLimitsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub before: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectRateLimitsResponse {
    pub object: String,
    pub data: Vec<ProjectRateLimit>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectRateLimitRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_1_day_max_input_tokens: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_audio_megabytes_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_images_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_day: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_requests_per_1_minute: Option<i64>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens_per_1_minute: Option<i64>,
}
```

- [ ] **Step 5: Register both modules**

In `rs_openai/src/interfaces/organization/projects/mod.rs`, add at the top of the file, above the existing type definitions:

```rust
pub mod api_keys;
pub mod rate_limits;
```

- [ ] **Step 6: Implement the api_keys methods**

Replace the full contents of `rs_openai/src/apis/organization/projects/api_keys.rs` with:

```rust
//! A project's API keys (`/organization/projects/{project_id}/api_keys`): list, retrieve,
//! delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::api_keys;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn list(
        &self,
        req: &api_keys::ListProjectApiKeysRequest,
    ) -> OpenAIResponse<api_keys::ListProjectApiKeysResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/api_keys", self.project_id),
                req,
            )
            .await
    }
}

pub struct OrganizationProjectApiKey<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectApiKey<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<api_keys::ProjectApiKey> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/api_keys/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<api_keys::DeleteProjectApiKeyResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/api_keys/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }
}
```

- [ ] **Step 7: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::api_keys::tests`
Expected: PASS

- [ ] **Step 8: Write the failing tests — `retrieve`/`delete` (api_keys), `list_rate_limits`/`update` (rate_limits)**

Add to the same test module in `rs_openai/src/apis/organization/projects/api_keys.rs`:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_service_account_owner() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/api_keys/key_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1700000000_i64,
                "name": "Service Key",
                "object": "organization.project.api_key",
                "owner": {
                    "type": "service_account",
                    "service_account": {
                        "id": "sa_1",
                        "created_at": 1700000000_i64,
                        "name": "CI Bot",
                        "role": "member"
                    }
                },
                "owner_project_access": "active",
                "redacted_value": "sk-...wxyz"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .api_key("key_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.owner.kind.as_deref(), Some("service_account"));
        assert_eq!(
            result.owner.service_account.as_ref().unwrap().name,
            "CI Bot"
        );
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/api_keys/key_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "deleted": true,
                "object": "organization.project.api_key.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .api_key("key_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
```

Add a separate test module to `rs_openai/src/apis/organization/projects/rate_limits.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::rate_limits::{
        ListProjectRateLimitsRequest, UpdateProjectRateLimitRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn list_rate_limits_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/rate_limits"))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "object": "list",
                "data": [{
                    "id": "rl_1",
                    "max_requests_per_1_minute": 500,
                    "max_tokens_per_1_minute": 10000,
                    "model": "gpt-4",
                    "object": "project.rate_limit"
                }],
                "first_id": "rl_1",
                "last_id": "rl_1",
                "has_more": false
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = ListProjectRateLimitsRequest::builder().limit(5).build();
        let result = client
            .organization()
            .project("proj_1")
            .rate_limits()
            .list_rate_limits(&req)
            .await
            .unwrap();

        assert_eq!(result.data[0].model, "gpt-4");
        assert_eq!(result.data[0].batch_1_day_max_input_tokens, None);
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/rate_limits/rl_1"))
            .and(body_json(serde_json::json!({"max_requests_per_1_minute": 1000})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "rl_1",
                "max_requests_per_1_minute": 1000,
                "max_tokens_per_1_minute": 10000,
                "model": "gpt-4",
                "object": "project.rate_limit"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectRateLimitRequest::builder()
            .max_requests_per_1_minute(1000)
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .rate_limits()
            .rate_limit("rl_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.max_requests_per_1_minute, 1000);
    }
}
```

- [ ] **Step 9: Implement the rate_limits methods**

Replace the full contents of `rs_openai/src/apis/organization/projects/rate_limits.rs` with:

```rust
//! A project's rate limits (`/organization/projects/{project_id}/rate_limits`): list,
//! update -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::rate_limits;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectRateLimits<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectRateLimits<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn list_rate_limits(
        &self,
        req: &rate_limits::ListProjectRateLimitsRequest,
    ) -> OpenAIResponse<rate_limits::ListProjectRateLimitsResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/rate_limits", self.project_id),
                req,
            )
            .await
    }

    pub fn rate_limit(&self, id: impl Into<String>) -> OrganizationProjectRateLimit<'a> {
        OrganizationProjectRateLimit::new(self.openai, self.project_id.clone(), id.into())
    }
}

pub struct OrganizationProjectRateLimit<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectRateLimit<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn update(
        &self,
        req: &rate_limits::UpdateProjectRateLimitRequest,
    ) -> OpenAIResponse<rate_limits::ProjectRateLimit> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/rate_limits/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }
}
```

- [ ] **Step 10: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects`
Expected: 11 tests PASS (6 from Task 1 + 5 new)

- [ ] **Step 11: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, 241/241 (236 from Task 1 + 5 new — re-verify the actual current count first if Task 1 isn't merged yet by checking the actual current file state).

- [ ] **Step 12: Commit**

```bash
git add rs_openai/src/interfaces/organization/projects/mod.rs
git add rs_openai/src/interfaces/organization/projects/api_keys.rs
git add rs_openai/src/interfaces/organization/projects/rate_limits.rs
git add rs_openai/src/apis/organization/projects/api_keys.rs
git add rs_openai/src/apis/organization/projects/rate_limits.rs
git commit -m "feat(organization): add project api_keys list/retrieve/delete and rate_limits list/update"
```

---

### Task 3: Projects.Groups (`create`/`retrieve`/`list`/`delete`)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/projects/mod.rs` (add `pub mod groups;`)
- Create: `rs_openai/src/interfaces/organization/projects/groups.rs`
- Modify: `rs_openai/src/apis/organization/projects/groups.rs` (add `create`/`list` methods; add a NEW `OrganizationProjectGroup<'a>` ID-scoped struct + a `.group(id)` accessor, since the scaffold only modeled the collection level; remove stale `#[allow(dead_code)]` from `OrganizationProjectGroups`)
- Test: `rs_openai/src/apis/organization/projects/groups.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::projects::OrganizationProjectGroups` struct shape from Phase 2a, unchanged. `OrganizationProjectGroup` is a NEW struct this task adds.
- Produces: `interfaces::organization::projects::groups::{AddProjectGroupRequest, ProjectGroup, ListProjectGroupsRequest, ListProjectGroupsResponse, RemoveProjectGroupResponse}`, `OrganizationProjectGroups::create`/`::list`/`::group(id)`, `OrganizationProjectGroup::retrieve`/`::delete`. Structurally independent of Tasks 1/2/4/5.

- [ ] **Step 1: Write the failing test — `create` serializes the request and deserializes a `ProjectGroup`**

Add a test module to `rs_openai/src/apis/organization/projects/groups.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::chat::SortOrder;
    use crate::interfaces::organization::projects::groups::{
        AddProjectGroupRequest, ListProjectGroupsRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/groups"))
            .and(body_json(serde_json::json!({"group_id": "group_1", "role": "member"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created_at": 1700000000_i64,
                "group_id": "group_1",
                "group_name": "Engineering",
                "group_type": "group",
                "object": "project.group",
                "project_id": "proj_1"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AddProjectGroupRequest::builder()
            .group_id("group_1")
            .role("member")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.group_name, "Engineering");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::groups::tests`
Expected: FAIL to compile — `interfaces::organization::projects::groups` module doesn't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/projects/groups.rs`:

```rust
//! A project's groups (`/organization/projects/{project_id}/groups`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::interfaces::chat::SortOrder;
use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddProjectGroupRequest {
    #[builder(into)]
    pub group_id: String,
    #[builder(into)]
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectGroup {
    pub created_at: i64,
    pub group_id: String,
    pub group_name: String,
    pub group_type: String,
    pub object: String,
    pub project_id: String,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectGroupsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<SortOrder>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectGroupsResponse {
    pub object: String,
    pub data: Vec<ProjectGroup>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct RemoveProjectGroupResponse {
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/projects/mod.rs`, add `pub mod groups;` alphabetically among the existing `pub mod` lines (after `api_keys`, before `rate_limits`).

- [ ] **Step 5: Implement the methods**

Replace the full contents of `rs_openai/src/apis/organization/projects/groups.rs` with:

```rust
//! A project's groups (`/organization/projects/{project_id}/groups`): create, list,
//! retrieve, delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::groups;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectGroups<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectGroups<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &groups::AddProjectGroupRequest,
    ) -> OpenAIResponse<groups::ProjectGroup> {
        self.openai
            .post(
                &format!("/organization/projects/{}/groups", self.project_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &groups::ListProjectGroupsRequest,
    ) -> OpenAIResponse<groups::ListProjectGroupsResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/groups", self.project_id),
                req,
            )
            .await
    }

    pub fn group(&self, id: impl Into<String>) -> OrganizationProjectGroup<'a> {
        OrganizationProjectGroup::new(self.openai, self.project_id.clone(), id.into())
    }
}

pub struct OrganizationProjectGroup<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectGroup<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<groups::ProjectGroup> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/groups/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<groups::RemoveProjectGroupResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/groups/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::groups::tests`
Expected: PASS

- [ ] **Step 7: Write the failing tests — `retrieve`, `list`, `delete` (proving `RemoveProjectGroupResponse` has no `id`)**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_group() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "created_at": 1700000000_i64,
                "group_id": "group_1",
                "group_name": "Engineering",
                "group_type": "group",
                "object": "project.group",
                "project_id": "proj_1"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.group_type, "group");
    }

    #[tokio::test]
    async fn list_serializes_order_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/groups"))
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
        let req = ListProjectGroupsRequest::builder().order(SortOrder::Desc).build();
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation_without_id() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/groups/group_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "deleted": true,
                "object": "project.group.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .groups()
            .group("group_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
        // RemoveProjectGroupResponse has no `id` field -- this test compiling and
        // passing is itself the proof that the asymmetry is modeled correctly.
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::groups::tests`
Expected: 4 tests PASS

- [ ] **Step 9: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, +4 over whatever the current baseline is.

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/organization/projects/mod.rs
git add rs_openai/src/interfaces/organization/projects/groups.rs
git add rs_openai/src/apis/organization/projects/groups.rs
git commit -m "feat(organization): add project groups create/retrieve/list/delete"
```

---

### Task 4: Projects.ServiceAccounts (`create`/`retrieve`/`update`/`list`/`delete`) + nested ServiceAccounts.ApiKeys (`create`)

**Files:**
- Modify: `rs_openai/src/interfaces/organization/projects/mod.rs` (add `pub mod service_accounts;`)
- Create: `rs_openai/src/interfaces/organization/projects/service_accounts.rs`
- Modify: `rs_openai/src/apis/organization/projects/service_accounts.rs` (add methods; add a NEW `OrganizationProjectServiceAccountApiKeys<'a>` struct + an `.api_keys()` accessor on `OrganizationProjectServiceAccount`; remove stale `#[allow(dead_code)]` from both existing structs)
- Test: `rs_openai/src/apis/organization/projects/service_accounts.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::projects::{OrganizationProjectServiceAccounts, OrganizationProjectServiceAccount}` struct shapes from Phase 2a, unchanged. `OrganizationProjectServiceAccountApiKeys` is a NEW struct this task adds.
- Produces: `interfaces::organization::projects::service_accounts::{CreateProjectServiceAccountRequest, ProjectServiceAccount, CreateProjectServiceAccountApiKey, CreateProjectServiceAccountResponse, UpdateProjectServiceAccountRequest, ListProjectServiceAccountsRequest, ListProjectServiceAccountsResponse, DeleteProjectServiceAccountResponse, CreateProjectServiceAccountApiKeyRequest, CreateProjectServiceAccountApiKeyResponse}`, `OrganizationProjectServiceAccounts::create`/`::list`, `OrganizationProjectServiceAccount::retrieve`/`::update`/`::delete`/`::api_keys()`, `OrganizationProjectServiceAccountApiKeys::create`. Structurally independent of Tasks 1/2/3/5.

- [ ] **Step 1: Write the failing test — `create` deserializes the distinct `CreateProjectServiceAccountResponse` shape with `api_key` populated**

Add a test module to `rs_openai/src/apis/organization/projects/service_accounts.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::service_accounts::{
        CreateProjectServiceAccountApiKeyRequest, CreateProjectServiceAccountRequest,
        ListProjectServiceAccountsRequest, UpdateProjectServiceAccountRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_deserializes_distinct_response_shape_with_api_key() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/service_accounts"))
            .and(body_json(serde_json::json!({"name": "CI Bot"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "member",
                "api_key": {
                    "id": "key_1",
                    "created_at": 1700000000_i64,
                    "name": "CI Bot API Key",
                    "object": "organization.project.service_account.api_key",
                    "value": "sk-live-abc123"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectServiceAccountRequest::builder().name("CI Bot").build();
        let result = client
            .organization()
            .project("proj_1")
            .service_accounts()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "member");
        let api_key = result.api_key.unwrap();
        assert_eq!(api_key.value, "sk-live-abc123");
        // CreateProjectServiceAccountResponse has an `api_key` field that
        // ProjectServiceAccount (retrieve/update/list) does not -- this test
        // compiling and passing is itself the proof the two types stayed separate.
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::service_accounts::tests`
Expected: FAIL to compile — `interfaces::organization::projects::service_accounts` module doesn't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/projects/service_accounts.rs`:

```rust
//! A project's service accounts
//! (`/organization/projects/{project_id}/service_accounts`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct CreateProjectServiceAccountRequest {
    #[builder(into)]
    pub name: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_service_account_only: Option<bool>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectServiceAccount {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub role: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountApiKey {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountResponse {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub role: String,
    pub api_key: Option<CreateProjectServiceAccountApiKey>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectServiceAccountRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectServiceAccountsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectServiceAccountsResponse {
    pub object: String,
    pub data: Vec<ProjectServiceAccount>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectServiceAccountResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}

// ServiceAccounts.ApiKeys sub-resource, colocated here (see File organization in the spec).

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct CreateProjectServiceAccountApiKeyRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub scopes: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct CreateProjectServiceAccountApiKeyResponse {
    pub id: String,
    pub created_at: i64,
    pub name: String,
    pub object: String,
    pub value: String,
}
```

Note: `CreateProjectServiceAccountApiKeyRequest.scopes` is `Option<Vec<String>>` on an ordinary JSON POST body — this serializes correctly via `#[derive(Serialize)]` with zero special handling; do not introduce any query-pairs machinery here, that only matters for query STRINGS (see sub-slice 3), and this field is never sent as one.

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/projects/mod.rs`, add `pub mod service_accounts;` alphabetically among the existing `pub mod` lines (after `rate_limits`, before `users` once Task 5 adds it — for now, after `rate_limits`).

- [ ] **Step 5: Implement the methods**

Replace the full contents of `rs_openai/src/apis/organization/projects/service_accounts.rs` with:

```rust
//! A project's service accounts
//! (`/organization/projects/{project_id}/service_accounts`): create, list, retrieve,
//! update, delete. Includes the nested ServiceAccounts.ApiKeys sub-resource (create
//! only) -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::service_accounts;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectServiceAccounts<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectServiceAccounts<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &service_accounts::CreateProjectServiceAccountRequest,
    ) -> OpenAIResponse<service_accounts::CreateProjectServiceAccountResponse> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts",
                    self.project_id
                ),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &service_accounts::ListProjectServiceAccountsRequest,
    ) -> OpenAIResponse<service_accounts::ListProjectServiceAccountsResponse> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/service_accounts",
                    self.project_id
                ),
                req,
            )
            .await
    }
}

pub struct OrganizationProjectServiceAccount<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectServiceAccount<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<service_accounts::ProjectServiceAccount> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn update(
        &self,
        req: &service_accounts::UpdateProjectServiceAccountRequest,
    ) -> OpenAIResponse<service_accounts::ProjectServiceAccount> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }

    pub async fn delete(
        &self,
    ) -> OpenAIResponse<service_accounts::DeleteProjectServiceAccountResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/service_accounts/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub fn api_keys(&self) -> OrganizationProjectServiceAccountApiKeys<'a> {
        OrganizationProjectServiceAccountApiKeys::new(
            self.openai,
            self.project_id.clone(),
            self.id.clone(),
        )
    }
}

pub struct OrganizationProjectServiceAccountApiKeys<'a> {
    openai: &'a OpenAI,
    project_id: String,
    service_account_id: String,
}

impl<'a> OrganizationProjectServiceAccountApiKeys<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, service_account_id: String) -> Self {
        Self {
            openai,
            project_id,
            service_account_id,
        }
    }

    pub async fn create(
        &self,
        req: &service_accounts::CreateProjectServiceAccountApiKeyRequest,
    ) -> OpenAIResponse<service_accounts::CreateProjectServiceAccountApiKeyResponse> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/service_accounts/{}/api_keys",
                    self.project_id, self.service_account_id
                ),
                req,
            )
            .await
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::service_accounts::tests`
Expected: PASS

- [ ] **Step 7: Write the failing tests — `retrieve`, `update`, `list`, `delete`, nested `ApiKeys::create`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_base_shape_without_api_key_field() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "owner"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn update_serializes_only_provided_fields() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .and(body_json(serde_json::json!({"role": "owner"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "created_at": 1700000000_i64,
                "name": "CI Bot",
                "object": "organization.project.service_account",
                "role": "owner"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectServiceAccountRequest::builder().role("owner").build();
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/service_accounts"))
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
        let req = ListProjectServiceAccountsRequest::builder().limit(10).build();
        let result = client
            .organization()
            .project("proj_1")
            .service_accounts()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/service_accounts/sa_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "sa_1",
                "deleted": true,
                "object": "organization.project.service_account.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }

    #[tokio::test]
    async fn api_keys_create_serializes_scopes_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path(
                "/organization/projects/proj_1/service_accounts/sa_1/api_keys",
            ))
            .and(body_json(serde_json::json!({
                "name": "CI Key",
                "scopes": ["api.model.request"]
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "key_1",
                "created_at": 1700000000_i64,
                "name": "CI Key",
                "object": "organization.project.service_account.api_key",
                "value": "sk-live-xyz789"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = CreateProjectServiceAccountApiKeyRequest::builder()
            .name("CI Key")
            .scopes(vec!["api.model.request".to_string()])
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .service_account("sa_1")
            .api_keys()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.value, "sk-live-xyz789");
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::service_accounts::tests`
Expected: 6 tests PASS

- [ ] **Step 9: Run fmt, clippy, and the full test suite**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, +6 over whatever the current baseline is.

- [ ] **Step 10: Commit**

```bash
git add rs_openai/src/interfaces/organization/projects/mod.rs
git add rs_openai/src/interfaces/organization/projects/service_accounts.rs
git add rs_openai/src/apis/organization/projects/service_accounts.rs
git commit -m "feat(organization): add project service_accounts CRUD and nested api_keys create"
```

---

### Task 5: Projects.Users (`create`/`retrieve`/`update`/`list`/`delete`) + final cross-check

**Files:**
- Modify: `rs_openai/src/interfaces/organization/projects/mod.rs` (add `pub mod users;`)
- Create: `rs_openai/src/interfaces/organization/projects/users.rs`
- Modify: `rs_openai/src/apis/organization/projects/users.rs` (add methods, remove stale `#[allow(dead_code)]` from both structs)
- Modify: `rs_openai/src/apis/organization/mod.rs` (final cross-check: module-level `//!` header)
- Test: `rs_openai/src/apis/organization/projects/users.rs` (add a `#[cfg(test)] mod tests` block — none exists yet)

**Interfaces:**
- Consumes: existing `apis::organization::projects::{OrganizationProjectUsers, OrganizationProjectUser}` struct shapes from Phase 2a, unchanged.
- Produces: `interfaces::organization::projects::users::{AddProjectUserRequest, ProjectUser, UpdateProjectUserRequest, ListProjectUsersRequest, ListProjectUsersResponse, DeleteProjectUserResponse}`, `OrganizationProjectUsers::create`/`::list`, `OrganizationProjectUser::retrieve`/`::update`/`::delete` — the final cross-check depends on this task landing last.

- [ ] **Step 1: Write the failing test — `create` serializes the request and deserializes a `ProjectUser`**

Add a test module to `rs_openai/src/apis/organization/projects/users.rs`:

```rust
#[cfg(test)]
mod tests {
    use crate::client::OpenAI;
    use crate::interfaces::organization::projects::users::{
        AddProjectUserRequest, ListProjectUsersRequest, UpdateProjectUserRequest,
    };
    use wiremock::matchers::{body_json, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn create_serializes_request_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/users"))
            .and(body_json(serde_json::json!({
                "role": "member",
                "email": "dev@example.com"
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "member",
                "email": "dev@example.com",
                "name": "Dev User"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = AddProjectUserRequest::builder()
            .role("member")
            .email("dev@example.com")
            .build();
        let result = client
            .organization()
            .project("proj_1")
            .users()
            .create(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "member");
        assert_eq!(result.name.as_deref(), Some("Dev User"));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::users::tests`
Expected: FAIL to compile — `interfaces::organization::projects::users` module doesn't exist yet.

- [ ] **Step 3: Create the types file**

Create `rs_openai/src/interfaces/organization/projects/users.rs`:

```rust
//! A project's users (`/organization/projects/{project_id}/users`) -- see
//! docs/superpowers/specs/2026-08-20-org-projects-design.md.

use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Deserialize, Clone)]
pub struct AddProjectUserRequest {
    #[builder(into)]
    pub role: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub email: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub user_id: Option<String>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ProjectUser {
    pub id: String,
    pub added_at: i64,
    pub object: String,
    pub role: String,
    pub email: Option<String>,
    pub name: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct UpdateProjectUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub role: Option<String>,
}

#[derive(Builder, Debug, Serialize, Deserialize, Clone, Default)]
pub struct ListProjectUsersRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[builder(into)]
    pub after: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ListProjectUsersResponse {
    pub object: String,
    pub data: Vec<ProjectUser>,
    pub first_id: Option<String>,
    pub last_id: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteProjectUserResponse {
    pub id: String,
    pub deleted: bool,
    pub object: String,
}
```

- [ ] **Step 4: Register the module**

In `rs_openai/src/interfaces/organization/projects/mod.rs`, add `pub mod users;` alphabetically among the existing `pub mod` lines (after `service_accounts`, last in the list).

- [ ] **Step 5: Implement the methods**

Replace the full contents of `rs_openai/src/apis/organization/projects/users.rs` with:

```rust
//! A project's users (`/organization/projects/{project_id}/users`): create, list,
//! retrieve, update, delete -- see docs/superpowers/specs/2026-08-20-org-projects-design.md.

use crate::client::OpenAI;
use crate::interfaces::organization::projects::users;
use crate::shared::response_wrapper::OpenAIResponse;

pub struct OrganizationProjectUsers<'a> {
    openai: &'a OpenAI,
    project_id: String,
}

impl<'a> OrganizationProjectUsers<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String) -> Self {
        Self { openai, project_id }
    }

    pub async fn create(
        &self,
        req: &users::AddProjectUserRequest,
    ) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .post(
                &format!("/organization/projects/{}/users", self.project_id),
                req,
            )
            .await
    }

    pub async fn list(
        &self,
        req: &users::ListProjectUsersRequest,
    ) -> OpenAIResponse<users::ListProjectUsersResponse> {
        self.openai
            .get(
                &format!("/organization/projects/{}/users", self.project_id),
                req,
            )
            .await
    }
}

pub struct OrganizationProjectUser<'a> {
    openai: &'a OpenAI,
    project_id: String,
    id: String,
}

impl<'a> OrganizationProjectUser<'a> {
    pub(crate) fn new(openai: &'a OpenAI, project_id: String, id: String) -> Self {
        Self {
            openai,
            project_id,
            id,
        }
    }

    pub async fn retrieve(&self) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .get(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }

    pub async fn update(
        &self,
        req: &users::UpdateProjectUserRequest,
    ) -> OpenAIResponse<users::ProjectUser> {
        self.openai
            .post(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                req,
            )
            .await
    }

    pub async fn delete(&self) -> OpenAIResponse<users::DeleteProjectUserResponse> {
        self.openai
            .delete(
                &format!(
                    "/organization/projects/{}/users/{}",
                    self.project_id, self.id
                ),
                &(),
            )
            .await
    }
}
```

- [ ] **Step 6: Run test to verify it passes**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::users::tests`
Expected: PASS

- [ ] **Step 7: Write the failing tests — `retrieve`, `update`, `list`, `delete`**

Add to the same test module:

```rust
    #[tokio::test]
    async fn retrieve_deserializes_user() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "owner",
                "email": "dev@example.com"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .retrieve()
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn update_serializes_role_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .and(body_json(serde_json::json!({"role": "owner"})))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "added_at": 1700000000_i64,
                "object": "organization.project.user",
                "role": "owner",
                "email": "dev@example.com"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let req = UpdateProjectUserRequest::builder().role("owner").build();
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .update(&req)
            .await
            .unwrap();

        assert_eq!(result.role, "owner");
    }

    #[tokio::test]
    async fn list_serializes_query_params_and_deserializes_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(path("/organization/projects/proj_1/users"))
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
        let req = ListProjectUsersRequest::builder().limit(10).build();
        let result = client
            .organization()
            .project("proj_1")
            .users()
            .list(&req)
            .await
            .unwrap();

        assert!(result.data.is_empty());
    }

    #[tokio::test]
    async fn delete_deserializes_confirmation() {
        let mock_server = MockServer::start().await;

        Mock::given(method("DELETE"))
            .and(path("/organization/projects/proj_1/users/user_1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "user_1",
                "deleted": true,
                "object": "organization.project.user.deleted"
            })))
            .mount(&mock_server)
            .await;

        let client = OpenAI::new("test-key").with_base_url(mock_server.uri());
        let result = client
            .organization()
            .project("proj_1")
            .user("user_1")
            .delete()
            .await
            .unwrap();

        assert!(result.deleted);
    }
```

- [ ] **Step 8: Run tests to verify they pass**

Run: `cd rs_openai && NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai apis::organization::projects::users::tests`
Expected: 5 tests PASS

- [ ] **Step 9: Final cross-check — re-read the whole branch's diff and fix the module-level header**

This is the last task in the plan, and the last task of the entire Organization/admin effort. Run `git diff master --stat` (or `git log --oneline master..HEAD` then `git diff <merge-base>..HEAD`) and re-read `rs_openai/src/apis/organization/mod.rs` in full:

- Confirm `projects()`/`project()`'s doc comments now say "Scoped accessor for organization projects."/"Scoped accessor for a specific organization project." (no "Not yet implemented").
- Confirm `roles()`/`role()` doc comments still correctly say "Not yet implemented" — the ONLY sub-resource left unimplemented in the entire Organization/admin surface once this task lands.
- Update the module-level `//!` header (currently ends with "...Admin API keys, audit logs, users, invites, groups, and usage are implemented...; projects and roles are not yet implemented..."). Change it to reflect that `projects` is now implemented too, leaving `roles` as the sole remaining gap — phrase it for one remaining item, not a list, e.g.:

```rust
//! Organization-level administration: audit logs, admin API keys, usage, groups, invites,
//! projects, roles, and users. A distinct "admin tier", separate from the "call the model"
//! surface the rest of this crate exposes. Admin API keys, audit logs, users, invites,
//! groups, usage, and projects are implemented (see
//! docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md,
//! docs/superpowers/specs/2026-08-20-org-users-invites-groups-design.md,
//! docs/superpowers/specs/2026-08-20-org-usage-design.md, and
//! docs/superpowers/specs/2026-08-20-org-projects-design.md); only roles remains not yet
//! implemented -- see docs/superpowers/research/2026-07-31-openai-api-structure.md.
```

Check the current exact wording first (it may have drifted slightly from what's shown above across the prior three fixes) and adapt precisely rather than pasting blindly.

- Also confirm every sub-resource accessor doc comment across `rs_openai/src/apis/organization/projects/mod.rs` (the 8 accessor methods: `api_keys()`/`api_key()`/`groups()`/`rate_limits()`/`service_accounts()`/`service_account()`/`users()`/`user()`) no longer says "Not yet implemented" anywhere — Tasks 1-4 should have already cleaned these up as they landed (Task 1 didn't touch them, Tasks 2-4 implemented the underlying methods but may not have touched these particular doc strings since they're on the PARENT `OrganizationProject` struct, not the sub-resource files themselves) — if any still say "Not yet implemented", fix them now as part of this same commit.

If any of these need updating, edit them directly as part of this step (before the final fmt/clippy/test pass below).

- [ ] **Step 10: Run fmt, clippy, and the full test suite one final time**

```bash
cd rs_openai
cargo fmt
NO_PROXY=127.0.0.1,localhost cargo clippy -p rs_openai --all-targets -- -D warnings
NO_PROXY=127.0.0.1,localhost cargo test -p rs_openai
```
Expected: clean, clean, all tests passing (baseline 230 + Task 1's 6 + Task 2's 5 + Task 3's 4 + Task 4's 6 + Task 5's 5 = 256).

- [ ] **Step 11: Commit**

```bash
git add rs_openai/src/interfaces/organization/projects/mod.rs
git add rs_openai/src/interfaces/organization/projects/users.rs
git add rs_openai/src/apis/organization/projects/users.rs
git add rs_openai/src/apis/organization/mod.rs
git commit -m "feat(organization): add project users CRUD, completing sub-slice 4 of 4 and all of Organization/admin"
```
