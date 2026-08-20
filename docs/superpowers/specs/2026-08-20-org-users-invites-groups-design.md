# Organization/Admin: Users + Invites + Groups (sub-slice 2 of 4) — Design

Status: Approved
Date: 2026-08-20

## Context

Second of 4 sub-slices for the Organization/admin scaffold (see `docs/superpowers/specs/2026-08-19-org-admin-api-keys-audit-logs-design.md` for sub-slice 1's context and the overall 4-way decomposition: (1) Admin API Keys + Audit Logs — shipped, (2) Users + Invites + Groups — this slice, (3) Usage, (4) Projects + its 5 nested sub-resources).

**Research method, same as sub-slice 1**: `developers.openai.com`'s reference pages for this resource tree are not reliably fetchable via WebFetch. Used `gh api repos/openai/openai-python/contents/...` against OpenAI's official, OpenAPI-spec-generated Python SDK source instead — confirmed authoritative (each file's header reads "File generated from our OpenAPI spec by Castiron"). Relevant source paths: `src/openai/types/admin/organization/{organization_user.py,invite.py,group.py,...}` for types, `src/openai/resources/admin/organization/{invites.py,users/users.py,groups/groups.py,groups/users.py}` for confirmed HTTP methods/routes.

**A larger surface than the slice name suggests, confirmed via research and scoped down via `AskUserQuestion`**: the full People/Groups area of Organization/admin actually spans 7 distinct sub-resources — Users (retrieve/list/update/delete), Users.Roles (assign/list/revoke a user's roles), Invites (full CRUD), Groups (full CRUD), Groups.Users (add/list/remove members), Groups.Roles (assign/list/revoke a group's roles), and a standalone top-level Roles resource (full CRUD over custom role *definitions*). The three Roles-related pieces (top-level Roles, Users.Roles, Groups.Roles) are a genuinely separate concern from core people/group management — they're about defining and assigning custom permission sets, not about who exists or which group they're in. Per this project's established pattern (defer a real, separable sub-concern rather than force it into an unrelated slice), **this slice covers Users + Invites + Groups + Groups.Users only, deferring all three Roles-related sub-resources to a future slice.** The existing Phase 2a scaffold's `OrganizationUser::roles()` → `OrganizationUserRoles` accessor and the top-level `Organization::roles()`/`role()` accessors stay untouched, still `#[allow(dead_code)]` where applicable, still honestly signaling "not yet implemented."

**Notable confirmed shape**: `OrganizationUser` (the response type for retrieve/list/update) has a `user: Option<User>` *nested* field carrying a different, smaller set of fields (`id`, `object`, `banned`, `banned_at`, `email`, `enabled`, `name`, `picture`) than the outer object itself (`id`, `added_at`, `email`, `name`, `role`, etc.) — this is not a typo or duplication risk in the source; both are separately populated, confirmed by reading the actual generated Pydantic model, not assumed from field-name similarity. Modeled as two distinct types: `OrganizationUser` (the outer, primary response) and `OrganizationUserNested` (the inner `user` field's shape) — named to avoid the obvious `User`/`OrganizationUser` confusion, since `OrganizationUser` is already taken by the primary type and OpenAI's own source resolves this by nesting one Pydantic class named `User` inside a different top-level module namespace, a resolution mechanism Rust's flat module-qua-type-name system doesn't have.

**Users has no `create`** — users only join an organization by accepting an `Invite`; this is confirmed structurally (no `user_create_params.py` exists at `types/admin/organization/` top level, and `resources/admin/organization/users/users.py` defines only `retrieve`/`update`/`list`/`delete`). `Groups.Users`, by contrast, DOES have `create` — adding an *existing* org user to a group is a distinct action from that user joining the org in the first place.

## Goals

- **Users**: `OrganizationUser::retrieve()` (`GET /organization/users/{user_id}`), `::update()` (`POST /organization/users/{user_id}`), `::delete()` (`DELETE /organization/users/{user_id}`), `OrganizationUsers::list()` (`GET /organization/users`). No `create`.
- `OrganizationUserNested`: `id: String`, `object: String` (always `"user"`), `banned: Option<bool>`, `banned_at: Option<i64>`, `email: Option<String>`, `enabled: Option<bool>`, `name: Option<String>`, `picture: Option<String>`.
- `OrganizationUserProjectsData`: `id: Option<String>`, `name: Option<String>`, `role: Option<String>`.
- `OrganizationUserProjects`: `object: String` (always `"list"`), `data: Vec<OrganizationUserProjectsData>`.
- `OrganizationUser` (response, used uniformly for retrieve/update/list): `id: String`, `added_at: i64`, `object: String` (always `"organization.user"`), `api_key_last_used_at: Option<i64>`, `created: Option<i64>`, `developer_persona: Option<String>`, `email: Option<String>`, `is_default: Option<bool>`, `is_scale_tier_authorized_purchaser: Option<bool>`, `is_scim_managed: Option<bool>`, `is_service_account: Option<bool>`, `name: Option<String>`, `projects: Option<OrganizationUserProjects>`, `role: Option<String>` (`"owner"` or `"reader"`, confirmed a plain string in the source, not an enum), `technical_level: Option<String>`, `user: Option<OrganizationUserNested>`.
- `UpdateOrganizationUserRequest`: `developer_persona: Option<String>`, `role: Option<String>`, `role_id: Option<String>`, `technical_level: Option<String>` — all optional (confirmed: a partial-update PATCH-style body sent as POST, matching the crate's existing `Update*Request` convention of all-optional fields).
- `ListOrganizationUsersRequest`: `after: Option<String>`, `limit: Option<u32>` — the confirmed source also has an `emails: Vec<String>` filter, but it's deferred here (see Non-goals) since `serde_urlencoded` (this crate's query-serialization backend, confirmed via `client.rs`'s `get()` using `reqwest`'s `.query()`) cannot serialize a `Vec<String>` field at all, empirically verified in a scratch project: `serde_urlencoded::to_string` on a struct with a `Vec<String>` field returns `Err(Custom("unsupported value"))`, not just a differently-formatted query string. This is the same class of unconfirmed-compound-query-param risk sub-slice 1 flagged and deferred for audit logs' array filters — now concretely confirmed as a real limitation, not just a hypothetical one.
- `ListOrganizationUsersResponse`/`DeleteOrganizationUserResponse` (`{id, deleted, object}`, `object` always `"organization.user.deleted"`): the crate's universal list/delete-response shapes.
- **Invites**: `OrganizationInvites::create()` (`POST /organization/invites`), `::list()` (`GET /organization/invites`), `OrganizationInvite::retrieve()` (`GET /organization/invites/{invite_id}`), `::delete()` (`DELETE /organization/invites/{invite_id}`).
- `InviteProject`: `id: String`, `role: String` (`"member"` or `"owner"`, plain string).
- `CreateInviteRequest`: `email: String` (required), `role: String` (required, `"reader"` or `"owner"`), `projects: Option<Vec<InviteProject>>`.
- `Invite` (response, used uniformly for create/retrieve/list): `id: String`, `created_at: i64`, `email: String`, `object: String` (always `"organization.invite"`), `projects: Vec<InviteProject>`, `role: String`, `status: String` (`"accepted"`/`"expired"`/`"pending"`, plain string — confirmed a closed 3-value set in the source, but kept a plain `String` per this project's standing convention of not enum-closing a field unless there's a concrete reason to construct/match on it programmatically, matching how `AuditLogEvent.kind`/`role` fields above are handled), `accepted_at: Option<i64>`, `expires_at: Option<i64>`.
- `ListInvitesRequest`: `after: Option<String>`, `limit: Option<u32>`.
- `ListInvitesResponse`/`DeleteInviteResponse` (`{id, deleted, object}`, `object` always `"organization.invite.deleted"`).
- **Groups**: `OrganizationGroups::create()` (`POST /organization/groups`), `::list()` (`GET /organization/groups`), `OrganizationGroup::retrieve()` (`GET /organization/groups/{group_id}`), `::update()` (`POST /organization/groups/{group_id}`), `::delete()` (`DELETE /organization/groups/{group_id}`).
- `CreateGroupRequest`: `name: String` (required).
- `UpdateGroupRequest`: `name: String` (required — confirmed non-optional in the source, unlike most `Update*Request`s in this crate; a group's display name is its only mutable field and the API requires it on every update call).
- `Group` (response for create/retrieve/list): `id: String`, `created_at: i64`, `group_type: String` (`"group"`/`"tenant_group"`, plain string), `is_scim_managed: bool`, `name: String`.
- `UpdateGroupResponse` (response for update — genuinely different shape than `Group`, confirmed via source: no `group_type` field): `id: String`, `created_at: i64`, `is_scim_managed: bool`, `name: String`.
- `ListGroupsRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>`.
- `ListGroupsResponse`/`DeleteGroupResponse` (`{id, deleted, object}`, `object` always `"group.deleted"`).
- **Groups.Users**: `OrganizationGroupUsers::create()` (`POST /organization/groups/{group_id}/users`), `::list()` (`GET /organization/groups/{group_id}/users`), `OrganizationGroupUser::retrieve()` (`GET /organization/groups/{group_id}/users/{user_id}`), `::delete()` (`DELETE /organization/groups/{group_id}/users/{user_id}`).
- `AddGroupUserRequest`: `user_id: String` (required).
- `GroupUser` (response for create/retrieve/list — a deliberately minimal, distinct shape from `OrganizationUser`, confirmed via source: this endpoint returns only membership-inspection fields, not the full user record): `id: String`, `email: Option<String>`, `name: String`.
- `ListGroupUsersRequest`: `after: Option<String>`, `limit: Option<u32>`, `order: Option<crate::interfaces::chat::SortOrder>`.
- `ListGroupUsersResponse`: the universal list shape, `data: Vec<GroupUser>`.
- `RemoveGroupUserResponse` (delete): `deleted: bool`, `object: String` (always `"group.user.deleted"`) — confirmed to NOT include an `id` field, unlike every other delete-response type in this crate so far (the group-user pair is identified by the two path segments, not echoed back).

## Non-goals (explicit, deferred to later slices)

- **Users' `emails` list filter** — deferred pending a real fix to this crate's query-serialization pipeline (it currently cannot serialize any `Vec<String>` query field, confirmed empirically; not specific to this endpoint). `ListOrganizationUsersRequest` ships with only `after`/`limit`, matching every other `List*Request` in this crate.
- **Top-level Roles** (custom role definitions: create/retrieve/update/list/delete), **Users.Roles** (assigning/listing/revoking a user's roles), and **Groups.Roles** (assigning/listing/revoking a group's roles) — a separable concern (custom permission definitions and assignments) from core people/group management. The existing scaffold's `OrganizationUserRoles`/`OrganizationGroupRoles`-equivalent accessors and top-level `roles()`/`role()` stay as honest, unimplemented stubs.
- **The rest of Organization/admin** (usage, projects + its sub-tree) — sub-slices 3-4, not this one.

## Design

### File organization

Following sub-slice 1's established convention: `rs_openai/src/interfaces/organization/users.rs`, `invites.rs`, `groups.rs` (new files, mirroring the existing `apis/organization/{users,invites,groups}.rs` split), each registered via `pub mod users; pub mod invites; pub mod groups;` in `interfaces/organization/mod.rs` (alphabetically alongside the existing `admin_api_keys`/`audit_logs`).

`Groups.Users` types live in `interfaces/organization/groups.rs` alongside `Group` itself (not a separate `groups/users.rs` file) — this crate's established precedent for a single-level ID-scoped sub-resource is to colocate it with its parent when the sub-resource is small (see `apis::vector_stores::files` being the one exception, split out because Vector Store Files is a large, independently-testable surface; Groups.Users here is 4 methods and 4 types, matching the size of e.g. Containers' Files sub-resource, which stayed in its own file — but Containers Files was *itself* the file's whole purpose, whereas here `groups.rs` is Groups' own file already. Precedent check: `apis::organization::users.rs`'s existing scaffold already nests `OrganizationUserRoles` inside `users.rs` rather than a separate file, confirming colocation is this specific module's established pattern for admin-tier sub-resources one level deep).

### Types

```rust
// interfaces/organization/users.rs
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

```rust
// interfaces/organization/invites.rs
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

```rust
// interfaces/organization/groups.rs
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

// Groups.Users sub-resource, colocated here (see File organization above).

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

### Naming

`OrganizationUser` (interfaces type, matching OpenAI's own object name) and `OrganizationUser<'a>` (apis accessor, the already-scaffolded Phase 2a name) share a bare name — but per this module's established convention (confirmed by reading `apis/organization/admin_api_keys.rs`: `use crate::interfaces::organization::admin_api_keys;` then qualified paths like `admin_api_keys::AdminApiKey`, NOT a flat `pub use ...::*;` re-export), `apis::organization` deliberately never glob-imports its interfaces submodules at all. So the two `OrganizationUser`s never actually share a scope — `apis/organization/users.rs` imports `crate::interfaces::organization::users` and refers to the interfaces type as `users::OrganizationUser` throughout, the same mechanical pattern already used for every type in sub-slice 1. Not a collision requiring resolution, just the module's normal reference style. `Group`/`Invite`/`GroupUser` don't collide with anything either.

### Confirmed asymmetries (do not assume symmetry without checking — standing crate rule)

- `Group` (create/retrieve/list response) vs. `UpdateGroupResponse` (update response) are genuinely different shapes — the latter omits `group_type`. Confirmed via source, not assumed. Two separate types, not one reused struct.
- `UpdateGroupRequest.name` is required, unlike this crate's usual all-optional `Update*Request` convention — confirmed via source (a group's only mutable field, and the API requires it on every update).
- `RemoveGroupUserResponse` omits `id`, unlike every other delete-response type in this crate — confirmed via source (the pair is identified by the two path segments already in the URL).
- `GroupUser` (Groups.Users response) is a deliberately smaller, different shape than `OrganizationUser` (Users response) — confirmed via source, not a shorthand/alias of the same type.

## Testing

`wiremock` tests covering: Users — `retrieve` deserializing the nested `user`/`projects` sub-objects; `update` serializing only the provided optional fields; `list` serializing `after`/`limit` and deserializing a paginated response; `delete`. Invites — `create` serializing `projects` and deserializing a full `Invite`; a minimal `create` test with `projects` omitted; `list`; `retrieve`; `delete`. Groups — `create`; `retrieve`; `update` deserializing into the distinct `UpdateGroupResponse` shape (proving the asymmetry is real, not accidentally reusing `Group`); `list` serializing `order`; `delete`. Groups.Users — `create` (add a member); `list`; `retrieve`; `delete` deserializing `RemoveGroupUserResponse` and confirming no `id` field is expected.

## Versioning

Purely additive — new modules under the existing `interfaces::organization`/`apis::organization` namespaces, no `client.rs` changes (same Bearer-auth `get`/`post`/`delete` reused throughout), no changes to already-shipped sub-slice 1 code. Pre-1.0 minor bump alongside the rest of the accumulated Phase 2b work, per established practice.
